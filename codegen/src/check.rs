//! Single-file round-trip verification core, exposed as a library.
//!
//! An external harness owns the bulk file reading + parallel infrastructure;
//! this crate owns "check one file". For one STEP source:
//! parse -> filter to the closure subset (map A') -> generated read -> Model ->
//! generated topo write -> reparse -> same filter (map B) ->
//! `merkle_diff_maps(A', B) == None`.

use std::collections::{BTreeMap, BTreeSet};

use step_io::{Attribute, RawEntity, parse, parse_bytes};

use crate::generated::read::{RefSlot, complex_ref_slots, in_subset, read, ref_slots};
use crate::generated::write::{Writer, wrap_step};
use crate::merkle::merkle_diff_maps;

/// Why an entity was dropped before round-trip comparison. SlotLocal = a slot
/// value could not be normalized to standard (req-ref<-$, int<-fractional-real).
/// Unimplemented = the entity type is outside the generated closure (not yet
/// modeled, or an unknown/non-schema name). Nonstandard = a known closure type
/// sits in a SELECT slot that does not admit it (schema violation). Cascade = a
/// referrer whose target was dropped/dangling (root reason lives on the target's
/// own record, not duplicated here).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DropKind {
    SlotLocal,
    Unimplemented,
    Nonstandard,
    Cascade,
}

/// A drop reason aggregation key: kind + a human label (`<TYPE>` /
/// `<ENT>.<slot>-><TYPE>` / a slot-rule name / `ref-dropped`). Aggregated as
/// `BTreeMap<DropReason, usize>` (instance count per kind+label, no explosion).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropReason {
    pub kind: DropKind,
    pub key: String,
}

fn bump(drops: &mut BTreeMap<DropReason, usize>, kind: DropKind, key: impl Into<String>) {
    *drops
        .entry(DropReason {
            kind,
            key: key.into(),
        })
        .or_insert(0) += 1;
}

/// Outcome of checking one STEP file's in-scope subset.
pub enum CheckResult {
    /// Not applicable to this run, NOT a generator bug (source parse failure on
    /// a corrupt input file, or no in-scope entities).
    Skip(String),
    /// Round-trip data-equivalent. `validated` = in-scope subset entity count,
    /// `drops` = entities removed before comparison, keyed by reason (the coverage
    /// gap, formerly a single `escaped` count). `norm` = rewrite (fix) notes from
    /// the pre-read normalize pass.
    Pass {
        validated: usize,
        drops: BTreeMap<DropReason, usize>,
        norm: Vec<&'static str>,
    },
    /// Generator bug: panic in read/write, output reparse failure, or merkle
    /// mismatch. `validated`/`drops` carried for coverage accounting.
    Fail {
        reason: String,
        validated: usize,
        drops: BTreeMap<DropReason, usize>,
        norm: Vec<&'static str>,
    },
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
            attributes.iter().for_each(|a| collect_refs(a, &mut out))
        }
        RawEntity::Complex { parts, .. } => parts
            .iter()
            .for_each(|p| p.attributes.iter().for_each(|a| collect_refs(a, &mut out))),
    }
    out
}

/// Filter a parsed graph to the closure subset, recording WHY each entity is
/// dropped (the reasoned successor to the old `subset` escape filter). An entity
/// survives iff it is a closure type, all its transitive refs stay kept, and no
/// ref slot holds a type the slot does not admit. Drops are classified:
/// `Unimplemented` = non-closure type (`in_subset` false), `Cascade` = a kept
/// entity referencing a non-kept (dropped/dangling) target, `Nonstandard` = a
/// kept in-closure target in a slot whose SELECT rejects it. Fixpoint.
fn drop_pass(
    graph: &BTreeMap<u64, RawEntity>,
) -> (BTreeMap<u64, RawEntity>, BTreeMap<DropReason, usize>) {
    let mut drops: BTreeMap<DropReason, usize> = BTreeMap::new();
    // Candidate set: closure-typed entities. Non-closure types are unimplemented.
    let mut keep: BTreeSet<u64> = BTreeSet::new();
    // Root drop reason per id, so cascade can attribute itself to the ultimate
    // unimplemented/dangling type instead of a flat "ref-dropped" label.
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
    // Iteratively drop candidates — fixpoint, so a drop propagates to dependents.
    loop {
        let mut removed = false;
        let snapshot: Vec<u64> = keep.iter().copied().collect();
        for id in snapshot {
            let ent = &graph[&id];
            // cascade: a ref to a non-kept target drops this entity. Attribute it
            // to the target's root reason (propagated), so 2nd/3rd-order cascades
            // roll up to the ultimate unimplemented/dangling type.
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
            // nonstandard (slot-aware): a kept, in-closure target sitting in a
            // slot whose SELECT does not admit its type (schema violation).
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

/// Provenance accounting: every input entity (plus every synthetic entity added
/// by normalization) must be either kept (round-trip compared via merkle) or
/// dropped with a recorded reason. `n_base` = input + synthetic count, `validated`
/// = kept count, `drops` = reasoned drops (all kinds, slot-local folded in).
/// Returns `Some(msg)` if `validated + sum(drops) != n_base` — an unexplained loss
/// (an entity vanished with no reason = a silent-drop bug). The oracle treats this
/// as a Fail, so the corpus gate proves the kept/reasoned partition is total
/// (nothing slips out unaccounted).
fn account(n_base: usize, validated: usize, drops: &BTreeMap<DropReason, usize>) -> Option<String> {
    let dropped: usize = drops.values().sum();
    let accounted = validated + dropped;
    if accounted == n_base {
        None
    } else {
        Some(format!(
            "unexplained loss: base={n_base}, validated={validated}, drops={dropped} \
             (accounted={accounted}, missing={})",
            n_base as i64 - accounted as i64
        ))
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

fn graph_of(g: &step_io::EntityGraph) -> BTreeMap<u64, RawEntity> {
    g.entities.iter().map(|(&id, e)| (id, e.clone())).collect()
}

/// Full pre-read normalization in two stages: `entity_normalize` (hand-written,
/// per-entity non-standard fixups) then `generic_normalize` (generated, generic
/// slot-kind rules). Entity-level runs FIRST so a per-entity fixup of a required
/// ref pre-empts the generic req-ref<-$ drop. Returns the map, the rewrite (fix)
/// notes, and the slot-local drop reasons (entities removed by generic_normalize,
/// surfaced on the drops channel — separate from the fix notes).
fn normalize_all(
    g: &step_io::EntityGraph,
) -> (
    BTreeMap<u64, RawEntity>,
    Vec<&'static str>,
    Vec<(u64, &'static str)>,
    usize,
) {
    let mut raw = graph_of(g);
    let mut norm: Vec<&'static str> = Vec::new();
    let before = raw.len();
    crate::entity_normalize::apply(&mut raw, &mut norm);
    // entity_normalize is add-only (materializes synthetic entities, never removes),
    // so the count delta is exactly the number of synthetic entities added. The
    // accounting baseline is input + synthetic (those additions are legitimate
    // normalization output, not input loss).
    let n_synth = raw.len() - before;
    let (normalized, gnorm, slot_drops) = crate::generated::generic_normalize::normalize(raw);
    norm.extend(gnorm);
    (normalized, norm, slot_drops, n_synth)
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

/// Debug helper: return `(input_subset, regenerated)` Debug strings of every
/// entity whose type matches `type_name`, sorted. Diffing the two reveals the
/// shapes the generator round-trips wrong.
pub fn dump_type(src: &str, type_name: &str) -> (Vec<String>, Vec<String>) {
    let g = parse(src).expect("parse");
    let (normalized, _, _, _) = normalize_all(&g);
    let (a, _) = drop_pass(&normalized);
    let mut left: Vec<String> = a
        .values()
        .filter(|e| ent_name(e) == type_name)
        .map(attrs_only)
        .collect();
    left.sort();

    let (model, _) = read(&a);
    let file = wrap_step(&Writer::new(&model).emit_all());
    let bg = parse(&file).expect("reparse output");
    let (b, _) = drop_pass(&graph_of(&bg));
    let mut right: Vec<String> = b
        .values()
        .filter(|e| ent_name(e) == type_name)
        .map(attrs_only)
        .collect();
    right.sort();
    (left, right)
}

/// Type name + attributes, ignoring entity id and span (which renumber). Entity
/// refs still carry raw target ids so ref-heavy types stay noisy; refless types
/// (DIRECTION, CARTESIAN_POINT, ...) diff cleanly.
fn attrs_only(e: &RawEntity) -> String {
    match e {
        RawEntity::Simple {
            name, attributes, ..
        } => format!("{name}{attributes:?}"),
        RawEntity::Complex { parts, .. } => parts
            .iter()
            .map(|p| format!("{}{:?}", p.name, p.attributes))
            .collect::<Vec<_>>()
            .join("+"),
    }
}

/// Round-trip the in-scope subset of one STEP source through the generated
/// schema-faithful module and compare via merkle data-equivalence.
pub fn check_roundtrip(src: &str) -> CheckResult {
    check_roundtrip_bytes(src.as_bytes())
}

/// Debug: run the read+write pipeline WITHOUT catching panics, so a generated-
/// code panic propagates with a backtrace (the normal path swallows it as Fail).
pub fn check_roundtrip_raw(src: &[u8]) {
    let g = parse_bytes(src).expect("source parse");
    let (normalized, _norm, _slot_drops, _n_synth) = normalize_all(&g);
    let (a, _drops) = drop_pass(&normalized);
    let (model, _idmap) = read(&a);
    let body = Writer::new(&model).emit_all();
    let _ = wrap_step(&body);
    eprintln!("raw roundtrip OK ({} entities)", a.len());
}

/// Bytes entry point — parses via `parse_bytes` so non-UTF-8 (Latin-1) input
/// files survive (matches the harness). The regenerated output is our own UTF-8
/// text, reparsed with `parse`.
pub fn check_roundtrip_bytes(src: &[u8]) -> CheckResult {
    // Original parse failure = corrupt input, not our bug.
    let g = match parse_bytes(src) {
        Ok(g) => g,
        Err(e) => return CheckResult::Skip(format!("source parse failed: {e}")),
    };
    // Pre-read normalize: rewrite non-standard values to standard (norm notes)
    // or drop non-normalizable entities. Slot-local drops come back separately;
    // the drop pass below adds unimplemented/cascade. The strict model only ever
    // sees standard data.
    let n_in = g.entities.len();
    let (normalized, norm, slot_drops, n_synth) = normalize_all(&g);
    let (a, mut drops) = drop_pass(&normalized);
    // Fold the slot-local drops (from generic_normalize) into the same channel.
    for (_id, r) in slot_drops {
        bump(&mut drops, DropKind::SlotLocal, r);
    }
    let validated = a.len();
    // Provenance accounting: input + synthetic == kept + reasoned-drops, else a
    // silent-drop bug. Checked before the empty Skip so all-dropped files are
    // accounted too.
    if let Some(reason) = account(n_in + n_synth, validated, &drops) {
        return CheckResult::Fail {
            reason,
            validated,
            drops,
            norm,
        };
    }
    if a.is_empty() {
        return CheckResult::Skip("no in-scope entities".to_string());
    }

    // The generator (read/write) may panic on shapes not yet handled; treat a
    // panic as a generator bug (Fail), not a run-aborting crash at scale.
    let emitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (model, _idmap) = read(&a);
        let body = Writer::new(&model).emit_all();
        wrap_step(&body)
    }));
    let file = match emitted {
        Ok(f) => f,
        Err(_) => {
            return CheckResult::Fail {
                reason: "panic in generated read/write".to_string(),
                validated,
                drops,
                norm,
            };
        }
    };

    // Output reparse failure = generator produced malformed text = our bug.
    // The output is compared UNFILTERED (no output-side drop_pass): it is the emit
    // of the kept model, so it must equal the kept input `a` exactly. Re-filtering
    // would hide a generator emitting an out-of-subset/extra entity.
    let b = match parse(&file) {
        Ok(bg) => graph_of(&bg),
        Err(e) => {
            return CheckResult::Fail {
                reason: format!("output reparse failed: {e}"),
                validated,
                drops,
                norm,
            };
        }
    };

    match merkle_diff_maps(&a, &b) {
        None => CheckResult::Pass {
            validated,
            drops,
            norm,
        },
        Some(reason) => CheckResult::Fail {
            reason: format!("merkle mismatch: {reason}"),
            validated,
            drops,
            norm,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drops_of(pairs: &[(DropKind, &str, usize)]) -> BTreeMap<DropReason, usize> {
        let mut m = BTreeMap::new();
        for (kind, key, n) in pairs {
            m.insert(
                DropReason {
                    kind: kind.clone(),
                    key: key.to_string(),
                },
                *n,
            );
        }
        m
    }

    /// Totality holds: validated + sum(drops) == input → no unexplained loss.
    #[test]
    fn account_balanced_is_ok() {
        let drops = drops_of(&[
            (DropKind::Unimplemented, "FOO", 3),
            (DropKind::Cascade, "via-FOO", 2),
        ]);
        // 10 = 5 kept + 5 dropped
        assert!(account(10, 5, &drops).is_none());
    }

    /// Negative check: an entity vanished with no recorded reason (validated +
    /// drops < input). The accounting MUST flag it — proves the gate is
    /// load-bearing, not vacuous.
    #[test]
    fn account_silent_loss_fails() {
        let drops = drops_of(&[(DropKind::Unimplemented, "FOO", 2)]);
        // input 10, but only 5 kept + 2 reasoned = 7 accounted → 3 silent
        let r = account(10, 5, &drops);
        assert!(r.is_some());
        assert!(r.unwrap().contains("missing=3"));
    }

    /// Symmetric negative check: over-counting (phantom drops) also fails.
    #[test]
    fn account_overcount_fails() {
        let drops = drops_of(&[(DropKind::Unimplemented, "FOO", 9)]);
        // 5 kept + 9 dropped = 14 > input 10
        assert!(account(10, 5, &drops).is_some());
    }
}
