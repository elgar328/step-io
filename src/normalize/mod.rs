//! Read-side pipeline for the codegen-generated schema-faithful model
//! (2-layer cutover, additive — gated behind the `gen` feature).
//!
//! `read` runs parse -> normalize -> drop -> generated read, returning the model
//! plus a `Report` that accounts for every input (and synthesized) entity: kept
//! (in the model) or dropped with a recorded reason. `write` emits the model back
//! to STEP text. The merkle data-equivalence oracle is NOT here — it lives in the
//! external verification harness, which calls these public functions.

use std::collections::{BTreeMap, BTreeSet};

use crate::generated::model::StepModel;
use crate::generated::profile::{SchemaProfile, SchemaTarget};
use crate::generated::read::{RefSlot, complex_ref_slots, in_subset, read as gen_read, ref_slots};
use crate::generated::write::{Writer, wrap_step};
use crate::{Attribute, Error, RawEntity, SchemaId, parse_bytes};

mod entity_normalize;
mod projection;
pub use projection::LossReport;

/// Why an entity was dropped before it could enter the model. `SlotLocal` = a
/// slot value could not be normalized to standard (req-ref<-$, int<-fractional).
/// `Unimplemented` = the entity type is outside the generated closure (not yet
/// modeled, or an unknown/non-schema name). `Nonstandard` = a known closure type
/// sits in a SELECT slot that does not admit it (schema violation). `Cascade` = a
/// referrer whose target was dropped/dangling. `Unclassified` = the generated
/// read could not consume the entity's own attributes (off-kind scalar, bad enum
/// token, short arity, …) and no policy layer (normalize / `drop_pass`) classified
/// it — a frontier signal: investigate and absorb via normalize or `nonstd_ref`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DropKind {
    SlotLocal,
    Unimplemented,
    Nonstandard,
    Cascade,
    Unclassified,
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
    /// Dropped entities, per-entity: input id + reason (slot-local + unimplemented
    /// + cascade + nonstandard). `dropped.len()` is the total drop count.
    pub dropped: Vec<(u64, DropReason)>,
    /// Non-standard rewrite notes (kept entities, fixed in place).
    pub norm: Vec<&'static str>,
    /// Identified source schema (AP family, edition, stage + raw `FILE_SCHEMA`).
    /// How callers (e.g. a CAD kernel) learn the precise version of the file.
    pub schema: SchemaId,
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
///
/// `known_bad` = entities the generated read already failed on (own-attr); they
/// are pre-dropped roots here so their referrers cascade. Their drop reason is
/// owned by the caller's read-failure accumulator, so they are NOT re-recorded
/// in `dropped` (avoids double-counting in the provenance accounting).
fn drop_pass(
    graph: &BTreeMap<u64, RawEntity>,
    known_bad: &BTreeSet<u64>,
) -> (BTreeSet<u64>, Vec<(u64, DropReason)>) {
    let mut dropped: Vec<(u64, DropReason)> = Vec::new();
    let mut keep: BTreeSet<u64> = BTreeSet::new();
    let mut root_of: BTreeMap<u64, String> = BTreeMap::new();
    for (&id, e) in graph {
        if known_bad.contains(&id) {
            // Dropped by read (reason owned by read_fail); seed root_of so its
            // referrers get a cascade label, but do not re-record the drop.
            root_of.insert(id, "unclassified".to_string());
        } else if in_subset(e) {
            keep.insert(id);
        } else {
            let n = ent_name(e);
            root_of.insert(id, n.clone());
            dropped.push((
                id,
                DropReason {
                    kind: DropKind::Unimplemented,
                    key: n,
                },
            ));
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
                    dropped.push((
                        id,
                        DropReason {
                            kind: DropKind::Cascade,
                            key: format!("via-{root}"),
                        },
                    ));
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
                dropped.push((
                    id,
                    DropReason {
                        kind: DropKind::Nonstandard,
                        key: reason,
                    },
                ));
                removed = true;
            }
        }
        if !removed {
            break;
        }
    }
    // Return the kept-id set (not a cloned entity map): the reader borrows the
    // entities from the full graph, so no RawEntity is cloned here.
    (keep, dropped)
}

/// Full pre-read normalization: `entity_normalize` (per-entity, add-only synthetic
/// fixups) then `generic_normalize` (generic slot-kind rules). Returns the map,
/// the rewrite notes, the slot-local drop reasons, and the synthetic-add count
/// (`entity_normalize` never removes, so the count delta is exactly the adds).
#[allow(clippy::type_complexity)]
fn normalize_all(
    raw: BTreeMap<u64, RawEntity>,
) -> (
    BTreeMap<u64, RawEntity>,
    Vec<&'static str>,
    Vec<(u64, &'static str)>,
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
/// The generated read is per-entity fallible: an entity whose own attributes the
/// strict reader cannot consume is dropped (reason recorded), never panicking.
/// A read failure pre-drops that entity in the next `drop_pass`, so its referrers
/// cascade through the same engine. The loop is bounded and converges in ≤2 real
/// iterations (own-attr readability is graph-independent, so all failures surface
/// in the first build; the second `drop_pass` cascades them to a clean set).
///
/// # Errors
/// Returns a [`Error`] only if the source does not parse. A parsed source
/// never fails the read: a malformed entity is dropped with a reason in
/// [`Report::dropped`] (the generated read is structurally panic-free).
pub fn read(src: &[u8]) -> Result<(StepModel, Report), Error> {
    /// ≤2 real iterations; one extra for slack (a violation degrades to dropping,
    /// not hanging — `debug_assert` flags non-convergence in dev).
    const MAX_ITERS: usize = 3;

    let g = parse_bytes(src)?;
    let n_in = g.entities.len();
    let schema = g.schema;
    let raw: BTreeMap<u64, RawEntity> = g.entities;
    let (normalized, norm, slot_drops, n_synth) = normalize_all(raw);

    // Outer cascade fixpoint: drop_pass (graph/ref validity) -> gen_read (own-attr
    // validity). read failures feed `known_bad` so the next drop_pass cascades
    // their referrers. `drop_pass` always seeds from the full `normalized` graph
    // (never `kept`) so the kept set is monotonically shrinking — this guarantees
    // termination; seeding from `kept` would break it.
    let mut known_bad: BTreeSet<u64> = BTreeSet::new();
    let mut read_fail: BTreeMap<u64, String> = BTreeMap::new();
    let mut model = StepModel::default();
    let mut dropped: Vec<(u64, DropReason)> = Vec::new();
    let mut validated = 0usize;
    let mut converged = false;
    for _ in 0..MAX_ITERS {
        let (kept, kept_dropped) = drop_pass(&normalized, &known_bad);
        // No catch_unwind belt: the generated read is structurally panic-free
        // (codegen emits no panic!/expect/unwrap/raw indexing — gated by grep), so
        // it returns per-entity drops instead of unwinding.
        let (m, _idmap, read_drops) = gen_read(&normalized, &kept);
        model = m;
        dropped = kept_dropped;
        validated = kept.len();
        if read_drops.is_empty() {
            converged = true;
            break;
        }
        for (id, r) in read_drops {
            read_fail.entry(id).or_insert(r);
            known_bad.insert(id);
        }
    }
    debug_assert!(
        converged,
        "fallible read did not converge in {MAX_ITERS} iters"
    );

    // Finalize provenance: cascade drops (this iter) + slot-local (normalize) +
    // accumulated read failures (Unclassified). Each id appears exactly once —
    // `drop_pass` excludes `known_bad` from its `dropped`, and `read_fail` keys ==
    // `known_bad`, so the three sets are disjoint (accounting invariant holds).
    for (id, r) in slot_drops {
        dropped.push((
            id,
            DropReason {
                kind: DropKind::SlotLocal,
                key: r.to_string(),
            },
        ));
    }
    for (id, r) in read_fail {
        dropped.push((
            id,
            DropReason {
                kind: DropKind::Unclassified,
                key: r,
            },
        ));
    }
    Ok((
        model,
        Report {
            n_in,
            n_synth,
            validated,
            dropped,
            norm,
            schema,
        },
    ))
}

/// Emit the model back to STEP text as the **Universal** target — the full read
/// model, no projection. Faithful round-trip / debug representation. For a
/// specific AP output use [`write_target`].
#[must_use]
pub fn write_universal(model: &StepModel) -> String {
    wrap_step(&Writer::new(model).emit_all())
}

/// Emit the model to a specific output target, projecting it onto that target's
/// legal entity set: rename-safe subtypes are downgraded, the rest dropped
/// (referential-closure cascade), stranded orphans pruned — all recorded in the
/// returned [`LossReport`]. `Universal` is a no-op projection (= [`write_universal`]).
///
/// There is no ground-truth to compare a projected output against; correctness
/// is "valid + fully accounted" — the output re-reads with zero drops, and
/// `input == kept + report.dropped + report.downgraded`.
#[must_use]
pub fn write_target(model: &StepModel, target: SchemaTarget) -> (String, LossReport) {
    match SchemaProfile::for_target(target) {
        None => (write_universal(model), LossReport::default()),
        Some(profile) => {
            let w = Writer::new(model);
            let plan = projection::plan(&w, profile);
            let body = w.emit_all_with_plan(&plan.dropped, plan.rename);
            (wrap_step_target(&body, profile), plan.loss)
        }
    }
}

/// Wrap a DATA body in the Part 21 envelope with the target's `FILE_SCHEMA`
/// (the only header field retargeted in the spike; `FILE_DESCRIPTION`/`FILE_NAME`
/// and the APD entity stay as the generated `wrap_step` emits them).
fn wrap_step_target(data_body: &str, profile: &SchemaProfile) -> String {
    let schema = profile
        .file_schema
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data_body}ENDSEC;\nEND-ISO-10303-21;\n"
    )
}
