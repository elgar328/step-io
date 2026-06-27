//! Read-side pipeline for the codegen-generated schema-faithful model
//! (2-layer cutover, additive — gated behind the `gen` feature).
//!
//! `read` runs parse -> normalize -> drop -> generated read, returning the model
//! plus a `Report` that accounts for every input (and synthesized) entity: kept
//! (in the model) or dropped with a recorded reason. `write` emits the model back
//! to STEP text. The merkle data-equivalence oracle is NOT here — it lives in the
//! external verification harness, which calls these public functions.

use std::collections::{BTreeMap, BTreeSet};

use crate::generated::model::Model;
use crate::generated::read::{RefSlot, complex_ref_slots, in_subset, read as gen_read, ref_slots};
use crate::generated::write::{Writer, wrap_step};
use crate::{Attribute, RawEntity, parse_bytes};

mod entity_normalize;

/// Why an entity was dropped before it could enter the model. `SlotLocal` = a
/// slot value could not be normalized to standard (req-ref<-$, int<-fractional).
/// `Unimplemented` = the entity type is outside the generated closure (not yet
/// modeled, or an unknown/non-schema name). `Nonstandard` = a known closure type
/// sits in a SELECT slot that does not admit it (schema violation). `Cascade` = a
/// referrer whose target was dropped/dangling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DropKind {
    SlotLocal,
    Unimplemented,
    Nonstandard,
    Cascade,
}

/// A drop reason: kind + a human label (`<TYPE>` / `<ENT>.<slot>-><TYPE>` /
/// a slot-rule name / `via-<root>`). Aggregated as instance counts per reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropReason {
    pub kind: DropKind,
    pub key: String,
}

/// Input-to-model provenance. Every input entity plus every synthetic entity
/// added by normalization is either kept (`validated`) or dropped with a reason
/// (`drops`): `validated + sum(drops) == n_in + n_synth`. `norm` = rewrite (fix)
/// notes from the pre-read normalize pass.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Input entity count (data section, before normalization).
    pub n_in: usize,
    /// Synthetic entities added by per-entity normalization (add-only).
    pub n_synth: usize,
    /// Kept entity count (entities that entered the model).
    pub validated: usize,
    /// Dropped entities, keyed by reason (instance count per reason).
    pub drops: BTreeMap<DropReason, usize>,
    /// Non-standard rewrite notes (kept entities, fixed in place).
    pub norm: Vec<&'static str>,
}

/// A read failure: the source did not parse, or the generated read panicked on a
/// shape not yet handled (the strict reader expects normalized, in-subset data).
#[derive(Clone, Debug)]
pub enum ReadError {
    Parse(String),
    Panic,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Parse(e) => write!(f, "parse failed: {e}"),
            ReadError::Panic => write!(f, "panic in generated read"),
        }
    }
}

impl std::error::Error for ReadError {}

fn bump(drops: &mut BTreeMap<DropReason, usize>, kind: DropKind, key: impl Into<String>) {
    *drops
        .entry(DropReason {
            kind,
            key: key.into(),
        })
        .or_insert(0) += 1;
}

/// Collect every entity id referenced (transitively) by an attribute.
fn collect_refs(a: &Attribute, out: &mut Vec<u64>) {
    match a {
        Attribute::EntityRef(n) => out.push(*n),
        Attribute::List(l) => l.iter().for_each(|e| collect_refs(e, out)),
        Attribute::Typed { value, .. } => collect_refs(value, out),
        _ => {}
    }
}

fn entity_refs(ent: &RawEntity) -> Vec<u64> {
    let mut out = Vec::new();
    match ent {
        RawEntity::Simple { attributes, .. } => {
            for a in attributes {
                collect_refs(a, &mut out);
            }
        }
        RawEntity::Complex { parts, .. } => {
            for p in parts {
                for a in &p.attributes {
                    collect_refs(a, &mut out);
                }
            }
        }
    }
    out
}

fn ent_name(e: &RawEntity) -> String {
    match e {
        RawEntity::Simple { name, .. } => name.clone(),
        RawEntity::Complex { parts, .. } => {
            let mut n: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
            n.sort_unstable();
            format!("({})", n.join("+"))
        }
    }
}

/// Slot-aware nonstandard-ref check: does any ref slot of `ent` point at a kept,
/// in-closure target whose type the slot's SELECT does not admit? Returns the
/// first `<ENT>.<slot>-><TYPE>` label. Out-of-closure / dangling targets are NOT
/// flagged here — those are cascade/unimplemented.
fn nonstd_ref(ent: &RawEntity, graph: &BTreeMap<u64, RawEntity>) -> Option<String> {
    match ent {
        RawEntity::Simple {
            name, attributes, ..
        } => check_ref_slots(name, attributes, ref_slots(name), graph),
        RawEntity::Complex { parts, .. } => parts.iter().find_map(|p| {
            check_ref_slots(&p.name, &p.attributes, complex_ref_slots(&p.name), graph)
        }),
    }
}

fn check_ref_slots(
    ename: &str,
    attrs: &[Attribute],
    slots: &[RefSlot],
    graph: &BTreeMap<u64, RawEntity>,
) -> Option<String> {
    for rs in slots {
        let Some(a) = attrs.get(rs.idx) else { continue };
        let mut targets: Vec<u64> = Vec::new();
        collect_refs(a, &mut targets);
        for r in targets {
            let Some(target) = graph.get(&r) else {
                continue;
            };
            if !in_subset(target) {
                continue; // out-of-closure: cascade / unimplemented handles it
            }
            let admitted = match target {
                RawEntity::Complex { .. } => rs.complex_ok,
                RawEntity::Simple { name, .. } => rs.allowed.contains(&name.as_str()),
            };
            if !admitted {
                return Some(format!("{ename}.{}->{}", rs.name, ent_name(target)));
            }
        }
    }
    None
}

/// Filter a normalized graph to the closure subset, recording WHY each entity is
/// dropped. An entity survives iff it is a closure type, all its transitive refs
/// stay kept, and no ref slot holds a type the slot does not admit (fixpoint).
fn drop_pass(
    graph: &BTreeMap<u64, RawEntity>,
) -> (BTreeMap<u64, RawEntity>, BTreeMap<DropReason, usize>) {
    let mut drops: BTreeMap<DropReason, usize> = BTreeMap::new();
    let mut keep: BTreeSet<u64> = BTreeSet::new();
    let mut root_of: BTreeMap<u64, String> = BTreeMap::new();
    for (&id, e) in graph {
        if in_subset(e) {
            keep.insert(id);
        } else {
            let n = ent_name(e);
            root_of.insert(id, n.clone());
            bump(&mut drops, DropKind::Unimplemented, n);
        }
    }
    loop {
        let mut removed = false;
        let snapshot: Vec<u64> = keep.iter().copied().collect();
        for id in snapshot {
            let ent = &graph[&id];
            let mut cascaded = false;
            for r in entity_refs(ent) {
                if !keep.contains(&r) {
                    keep.remove(&id);
                    let root = root_of
                        .get(&r)
                        .cloned()
                        .unwrap_or_else(|| "dangling".to_string());
                    bump(&mut drops, DropKind::Cascade, format!("via-{root}"));
                    root_of.insert(id, root);
                    removed = true;
                    cascaded = true;
                    break;
                }
            }
            if cascaded {
                continue;
            }
            if let Some(reason) = nonstd_ref(ent, graph) {
                keep.remove(&id);
                root_of.insert(id, reason.clone());
                bump(&mut drops, DropKind::Nonstandard, reason);
                removed = true;
            }
        }
        if !removed {
            break;
        }
    }
    let map = keep
        .into_iter()
        .map(|id| (id, graph[&id].clone()))
        .collect();
    (map, drops)
}

/// Full pre-read normalization: `entity_normalize` (per-entity, add-only synthetic
/// fixups) then `generic_normalize` (generic slot-kind rules). Returns the map,
/// the rewrite notes, the slot-local drop reasons, and the synthetic-add count
/// (`entity_normalize` never removes, so the count delta is exactly the adds).
fn normalize_all(
    raw: BTreeMap<u64, RawEntity>,
) -> (
    BTreeMap<u64, RawEntity>,
    Vec<&'static str>,
    Vec<&'static str>,
    usize,
) {
    let mut raw = raw;
    let mut norm: Vec<&'static str> = Vec::new();
    let before = raw.len();
    entity_normalize::apply(&mut raw, &mut norm);
    let n_synth = raw.len() - before;
    let (normalized, gnorm, slot_drops) = crate::generated::generic_normalize::normalize(raw);
    norm.extend(gnorm);
    (normalized, norm, slot_drops, n_synth)
}

/// Read STEP source into the schema-faithful model, returning a provenance
/// `Report`.
///
/// # Errors
/// Returns [`ReadError::Parse`] if the source does not parse, or
/// [`ReadError::Panic`] if the generated read panics on an unhandled shape.
pub fn read(src: &[u8]) -> Result<(Model, Report), ReadError> {
    let g = parse_bytes(src).map_err(|e| ReadError::Parse(e.to_string()))?;
    let n_in = g.entities.len();
    let raw: BTreeMap<u64, RawEntity> = g.entities;
    let (normalized, norm, slot_drops, n_synth) = normalize_all(raw);
    let (kept, mut drops) = drop_pass(&normalized);
    for r in slot_drops {
        bump(&mut drops, DropKind::SlotLocal, r);
    }
    let validated = kept.len();
    let model = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gen_read(&kept).0))
        .map_err(|_| ReadError::Panic)?;
    Ok((
        model,
        Report {
            n_in,
            n_synth,
            validated,
            drops,
            norm,
        },
    ))
}

/// Emit the model back to STEP text (the Universal target — the full read model).
#[must_use]
pub fn write(model: &Model) -> String {
    wrap_step(&Writer::new(model).emit_all())
}
