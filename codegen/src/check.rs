//! Single-file round-trip verification core, exposed as a library.
//!
//! The reference-check corpus harness owns the .sqfs reading + parallel
//! infrastructure; this crate owns "check one file". For one STEP source:
//! parse -> filter to the closure subset (map A') -> generated read -> Model ->
//! generated topo write -> reparse -> same filter (map B) ->
//! `merkle_diff_maps(A', B) == None`.

use std::collections::{BTreeMap, BTreeSet};

use step_io::{Attribute, RawEntity, parse, parse_bytes};

use crate::generated::read::{in_subset, read};
use crate::generated::write::{Writer, wrap_step};
use crate::merkle::merkle_diff_maps;

/// Outcome of checking one STEP file's in-scope subset.
pub enum CheckResult {
    /// Not applicable to this run, NOT a generator bug (source parse failure on
    /// a corrupt corpus file, or no in-scope entities).
    Skip(String),
    /// Round-trip data-equivalent. `validated` = in-scope subset entity count,
    /// `escaped` = closure-typed candidates dropped by the escape filter (their
    /// transitive refs left the closure) — the coverage gap. `norm` = NormCase
    /// notes applied by the pre-read normalize pass (rewrites/drops).
    Pass {
        validated: usize,
        escaped: usize,
        norm: Vec<&'static str>,
    },
    /// Generator bug: panic in read/write, output reparse failure, or merkle
    /// mismatch. `validated`/`escaped` carried for coverage accounting.
    Fail {
        reason: String,
        validated: usize,
        escaped: usize,
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

/// Filter a parsed graph to the schema-closure subset. An entity qualifies iff
/// it is a closure entity AND its transitive reference set stays entirely within
/// the closure (no escape into out-of-scope subsystems). Returns the kept subset
/// and the count of closure-typed candidates dropped by the escape filter.
fn subset(graph: &BTreeMap<u64, RawEntity>) -> (BTreeMap<u64, RawEntity>, usize) {
    // Candidate set: closure-typed entities.
    let mut keep: BTreeSet<u64> = graph
        .iter()
        .filter(|(_, e)| in_subset(e))
        .map(|(&id, _)| id)
        .collect();
    let candidates = keep.len();
    // Iteratively drop any candidate that references a non-candidate entity
    // (escape) — fixpoint, so an escape propagates back through dependents.
    loop {
        let mut removed = false;
        let snapshot: Vec<u64> = keep.iter().copied().collect();
        for id in snapshot {
            let ent = &graph[&id];
            for r in entity_refs(ent) {
                // A ref is in-scope iff its target is still kept. A target that
                // is non-closure-typed, dangling, or already dropped (escape
                // propagation) disqualifies this entity too.
                if !keep.contains(&r) {
                    keep.remove(&id);
                    removed = true;
                    break;
                }
            }
        }
        if !removed {
            break;
        }
    }
    let escaped = candidates - keep.len();
    let map = keep
        .into_iter()
        .map(|id| (id, graph[&id].clone()))
        .collect();
    (map, escaped)
}

fn graph_of(g: &step_io::EntityGraph) -> BTreeMap<u64, RawEntity> {
    g.entities.iter().map(|(&id, e)| (id, e.clone())).collect()
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
    let (normalized, _) = crate::generated::normalize::normalize(graph_of(&g));
    let (a, _) = subset(&normalized);
    let mut left: Vec<String> = a
        .values()
        .filter(|e| ent_name(e) == type_name)
        .map(attrs_only)
        .collect();
    left.sort();

    let (model, _) = read(&a);
    let file = wrap_step(&Writer::new(&model).emit_all());
    let bg = parse(&file).expect("reparse output");
    let (b, _) = subset(&graph_of(&bg));
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
    let (normalized, _norm) = crate::generated::normalize::normalize(graph_of(&g));
    let (a, _escaped) = subset(&normalized);
    let (model, _idmap) = read(&a);
    let body = Writer::new(&model).emit_all();
    let _ = wrap_step(&body);
    eprintln!("raw roundtrip OK ({} entities)", a.len());
}

/// Bytes entry point — parses via `parse_bytes` so non-UTF-8 (Latin-1) corpus
/// files survive (matches the reference-check harness). The regenerated output
/// is our own UTF-8 text, reparsed with `parse`.
pub fn check_roundtrip_bytes(src: &[u8]) -> CheckResult {
    // Original parse failure = corrupt corpus input, not our bug.
    let g = match parse_bytes(src) {
        Ok(g) => g,
        Err(e) => return CheckResult::Skip(format!("source parse failed: {e}")),
    };
    // Pre-read normalize: rewrite non-standard values to standard (NormCase
    // notes) or drop non-normalizable entities (referrers cascade-drop via the
    // subset escape filter below). The strict model only ever sees standard data.
    let (normalized, norm) = crate::generated::normalize::normalize(graph_of(&g));
    let (a, escaped) = subset(&normalized);
    if a.is_empty() {
        return CheckResult::Skip("no in-scope entities".to_string());
    }
    let validated = a.len();

    // The generator (read/write) may panic on shapes not yet handled; treat a
    // panic as a generator bug (Fail), not a run-aborting crash at corpus scale.
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
                escaped,
                norm,
            };
        }
    };

    // Output reparse failure = generator produced malformed text = our bug.
    let b = match parse(&file) {
        Ok(bg) => subset(&graph_of(&bg)).0,
        Err(e) => {
            return CheckResult::Fail {
                reason: format!("output reparse failed: {e}"),
                validated,
                escaped,
                norm,
            };
        }
    };

    match merkle_diff_maps(&a, &b) {
        None => CheckResult::Pass {
            validated,
            escaped,
            norm,
        },
        Some(reason) => CheckResult::Fail {
            reason: format!("merkle mismatch: {reason}"),
            validated,
            escaped,
            norm,
        },
    }
}
