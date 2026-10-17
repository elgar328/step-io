//! [`read`] — STEP source to [`StepModel`] plus a provenance [`Report`].
//!
//! Reading runs in three steps. [`read`] first calls into
//! [`parser`](crate::parser) to parse the source, and finally into the
//! generated readers to turn the entities into the typed model. The step
//! in between lives here:
//! non-standard input is healed in place where a safe rewrite exists, and
//! dropped with a [`DropReason`] where none does. The [`Report`] accounts
//! for everything that came in — kept, normalized, or dropped and why — and
//! carries what the parser repaired on the way in.

use std::collections::{BTreeMap, BTreeSet};

use crate::generated::model::StepModel;
use crate::generated::read::{RefSlot, complex_ref_slots, read as gen_read, ref_slots};
use crate::parser::{Attribute, Error, ParseWarning, RawEntity, parse_bytes};

mod entity_normalize;
pub(crate) mod index;

use index::{EntityIndex, Member};

/// Why an entity was dropped before it could enter the model.
/// `NonstandardValue` = one of the entity's own attribute *values* is nonstandard
/// and could not be normalized (a required reference is missing, an integer field
/// holds a fractional value). `NonstandardReference` = one of its *reference*
/// slots points at a target the schema does not admit. `Unimplemented` = the
/// entity type is outside the generated closure (not yet modeled, or an
/// unknown/non-schema name). `Cascade` = a referrer whose target was
/// dropped/dangling. `Unclassified` = the generated read could not consume the
/// entity's own attributes (off-kind scalar, bad enum token, short arity, …) and
/// no policy layer (normalize / `drop_pass`) classified it — a frontier signal:
/// investigate and absorb via normalize or `nonstd_ref`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DropKind {
    NonstandardValue,
    Unimplemented,
    NonstandardReference,
    Cascade,
    Unclassified,
}

/// A drop reason: kind + a human label (`<TYPE>` / `<ENT>: '<slot>' references
/// <TYPE>` / a slot-rule name / `via-<root>`). Aggregated as instance counts per
/// reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropReason {
    pub kind: DropKind,
    pub key: String,
}

/// Input-to-model provenance. Every input entity plus every synthetic entity
/// added by normalization is either kept (`validated`) or dropped with a reason
/// (`drops`): `validated + sum(drops) == n_in + n_synth`. `norm` = rewrite (fix)
/// notes from the pre-read normalize pass; `parse_warnings` = what the parser
/// repaired before that. The source header (including the identified schema)
/// lives on the model: [`StepModel::header`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Report {
    /// Input entity count (data section, before normalization).
    pub n_in: usize,
    /// Synthetic entities added by per-entity normalization (add-only).
    pub n_synth: usize,
    /// Kept entity count (entities that entered the model).
    pub validated: usize,
    /// Dropped entities, per-entity: input id + reason (nonstandard-value +
    /// unimplemented + cascade + nonstandard-reference). `dropped.len()` is the
    /// total drop count.
    pub dropped: Vec<(u64, DropReason)>,
    /// Non-standard rewrite notes (kept entities, fixed in place).
    pub norm: Vec<&'static str>,
    /// What the parser repaired or skipped on the way in (blank attribute
    /// slots, an undecodable string escape, …) — fixed at the syntax level,
    /// before normalization. Each prints as a one-line message.
    pub parse_warnings: Vec<ParseWarning>,
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

/// Fill `out` with every entity id `ent` references (cleared first).
fn entity_refs(ent: &RawEntity, out: &mut Vec<u64>) {
    out.clear();
    match ent {
        RawEntity::Simple { attributes, .. } => {
            for a in attributes {
                collect_refs(a, out);
            }
        }
        RawEntity::Complex { parts, .. } => {
            for p in parts {
                for a in &p.attributes {
                    collect_refs(a, out);
                }
            }
        }
    }
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
/// first `<ENT>: '<slot>' references <TYPE>` label. Out-of-closure / dangling
/// targets are NOT flagged here — those are cascade/unimplemented.
fn nonstd_ref(
    ent: &RawEntity,
    member: Member,
    index: &EntityIndex<'_>,
    buf: &mut Vec<u64>,
) -> Option<String> {
    match (ent, member) {
        (
            RawEntity::Simple {
                name, attributes, ..
            },
            Member::Simple(ix),
        ) => check_ref_slots(name, attributes, ref_slots(ix), index, buf),
        (RawEntity::Complex { parts, .. }, _) => parts.iter().find_map(|p| {
            check_ref_slots(
                &p.name,
                &p.attributes,
                complex_ref_slots(&p.name),
                index,
                buf,
            )
        }),
        _ => None,
    }
}

fn check_ref_slots(
    ename: &str,
    attrs: &[Attribute],
    slots: &[RefSlot],
    index: &EntityIndex<'_>,
    buf: &mut Vec<u64>,
) -> Option<String> {
    for rs in slots {
        let Some(a) = attrs.get(rs.idx) else { continue };
        buf.clear();
        collect_refs(a, buf);
        for &r in buf.iter() {
            let Some(tp) = index.pos(r) else {
                continue;
            };
            let admitted = match index.member(tp) {
                Member::Out => continue, // out-of-closure: cascade / unimplemented handles it
                Member::Complex => rs.complex_ok,
                Member::Simple(t) => rs.allowed.binary_search(&t).is_ok(),
            };
            if !admitted {
                return Some(format!(
                    "{ename}: '{}' references {}",
                    rs.name,
                    ent_name(index.entity(tp))
                ));
            }
        }
    }
    None
}

/// Filter a normalized graph to the closure subset, recording WHY each entity is
/// dropped. An entity survives iff it is a closure type, all its transitive refs
/// stay kept, and no ref slot holds a type the slot does not admit (fixpoint).
/// Returns the kept flags (by index position) and how many are set.
///
/// `known_bad` = entities the generated read already failed on (own-attr); they
/// are pre-dropped roots here so their referrers cascade. Their drop reason is
/// owned by the caller's read-failure accumulator, so they are NOT re-recorded
/// in `dropped` (avoids double-counting in the provenance accounting).
fn drop_pass(
    index: &EntityIndex<'_>,
    known_bad: &BTreeSet<u64>,
) -> (Vec<bool>, usize, Vec<(u64, DropReason)>) {
    let n = index.len();
    let mut dropped: Vec<(u64, DropReason)> = Vec::new();
    let mut keep = vec![false; n];
    let mut root_of: BTreeMap<u64, String> = BTreeMap::new();
    for (pos, kept) in keep.iter_mut().enumerate() {
        let id = index.id(pos);
        if known_bad.contains(&id) {
            // Dropped by read (reason owned by read_fail); seed root_of so its
            // referrers get a cascade label, but do not re-record the drop.
            root_of.insert(id, "unclassified".to_string());
        } else if index.member(pos) != Member::Out {
            *kept = true;
        } else {
            let name = ent_name(index.entity(pos));
            root_of.insert(id, name.clone());
            dropped.push((
                id,
                DropReason {
                    kind: DropKind::Unimplemented,
                    key: name,
                },
            ));
        }
    }
    let mut refs: Vec<u64> = Vec::new();
    let mut buf: Vec<u64> = Vec::new();
    loop {
        let mut removed = false;
        // In id order; an entity only ever drops itself, so checking its flag
        // at its turn is the same as snapshotting the kept set up front.
        for pos in 0..n {
            if !keep[pos] {
                continue;
            }
            let id = index.id(pos);
            let ent = index.entity(pos);
            entity_refs(ent, &mut refs);
            let missing = refs
                .iter()
                .copied()
                .find(|&r| !index.pos(r).is_some_and(|p| keep[p]));
            if let Some(r) = missing {
                keep[pos] = false;
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
                continue;
            }
            if let Some(reason) = nonstd_ref(ent, index.member(pos), index, &mut buf) {
                keep[pos] = false;
                root_of.insert(id, reason.clone());
                dropped.push((
                    id,
                    DropReason {
                        kind: DropKind::NonstandardReference,
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
    let kept = keep.iter().filter(|&&k| k).count();
    (keep, kept, dropped)
}

/// Full pre-read normalization: `entity_normalize` (per-entity, add-only synthetic
/// fixups) then `generic_normalize` (generic slot-kind rules). Returns the map,
/// the rewrite notes, the nonstandard-value drop reasons, and the synthetic-add count
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
    let parse_warnings = g.warnings;
    let n_in = g.entities.len();
    let header = crate::header::from_raw(&g.header, g.schema);
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
    let index = EntityIndex::new(&normalized);
    for _ in 0..MAX_ITERS {
        let (kept, n_kept, kept_dropped) = drop_pass(&index, &known_bad);
        // No catch_unwind belt: the generated read is structurally panic-free
        // (codegen emits no panic!/expect/unwrap/raw indexing — gated by grep), so
        // it returns per-entity drops instead of unwinding.
        let (m, _idmap, read_drops) = gen_read(&index, &kept);
        model = m;
        dropped = kept_dropped;
        validated = n_kept;
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

    // Finalize provenance: cascade drops (this iter) + nonstandard-value (normalize) +
    // accumulated read failures (Unclassified). Each id appears exactly once —
    // `drop_pass` excludes `known_bad` from its `dropped`, and `read_fail` keys ==
    // `known_bad`, so the three sets are disjoint (accounting invariant holds).
    for (id, r) in slot_drops {
        dropped.push((
            id,
            DropReason {
                kind: DropKind::NonstandardValue,
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
    model.header = header;
    Ok((
        model,
        Report {
            n_in,
            n_synth,
            validated,
            dropped,
            norm,
            parse_warnings,
        },
    ))
}
