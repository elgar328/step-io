//! Per-target projection: conform the Universal model to a target profile's
//! legal entity set, accounting every change in a [`LossReport`].
//!
//! Runs over the `AnyId` graph via the generated `Writer` primitives
//! (`all_ids`/`deps_of`/`name_of`/`complex_legal`). Steps:
//! (0) downgrade rename-safe illegal subtypes to their legal supertype keyword;
//! (1) seed = entities still illegal after downgrade;
//! (2) cascade = drop every referrer of a dropped entity (fixpoint) — this keeps
//! the kept set referentially closed, so `render_one`'s `id_of_ref`
//! (`.expect("dep id assigned")`) never panics (conservative over-drop is the
//! safe choice; the loss is recorded);
//! (3) orphan prune = entities reachable from in-degree-0 roots before but not
//! after projection. Input orphans / isolated cycles are preserved (never
//! root-reachable), so the first targeted write equals a settled write.

use std::collections::{HashMap, HashSet};

use crate::generated::model::AnyId;
use crate::generated::profile::SchemaProfile;
use crate::generated::write::Writer;

/// What a per-target write changed, for caller (CAD kernel) transparency.
#[derive(Clone, Debug, Default)]
pub struct LossReport {
    /// Dropped entities: (keyword/name, reason).
    pub dropped: Vec<(String, &'static str)>,
    /// Downgraded entities: (original keyword, target supertype keyword).
    pub downgraded: Vec<(String, &'static str)>,
}

impl LossReport {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dropped.is_empty() && self.downgraded.is_empty()
    }
}

pub(crate) struct ProjectionPlan {
    pub dropped: HashSet<AnyId>,
    pub rename: HashMap<AnyId, &'static str>,
    pub loss: LossReport,
}

const R_ILLEGAL: &str = "illegal in target schema";
const R_CASCADE: &str = "cascade (references a dropped entity)";
const R_ORPHAN: &str = "unreachable after projection";

fn display_name(any: AnyId) -> String {
    match any {
        AnyId::ComplexUnit(_) => "(complex)".to_string(),
        _ => Writer::name_of(any).to_string(),
    }
}

/// Dedup'd forward `AnyId` deps of `a` (self-edges removed).
fn deps(w: &Writer, a: AnyId) -> Vec<AnyId> {
    let mut v = Vec::new();
    w.deps_of(a, &mut v);
    v.into_iter()
        .filter(|&d| d != a)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn reach(
    roots: &[AnyId],
    fwd: &HashMap<AnyId, Vec<AnyId>>,
    allow: impl Fn(AnyId) -> bool,
) -> HashSet<AnyId> {
    let mut seen: HashSet<AnyId> = HashSet::new();
    let mut stack: Vec<AnyId> = Vec::new();
    for &r in roots {
        if allow(r) && seen.insert(r) {
            stack.push(r);
        }
    }
    while let Some(n) = stack.pop() {
        if let Some(ds) = fwd.get(&n) {
            for &d in ds {
                if allow(d) && seen.insert(d) {
                    stack.push(d);
                }
            }
        }
    }
    seen
}

pub(crate) fn plan(w: &Writer, profile: &SchemaProfile) -> ProjectionPlan {
    let all = w.all_ids();

    // Forward / reverse adjacency + in-degree (over dedup'd edges).
    let mut fwd: HashMap<AnyId, Vec<AnyId>> = HashMap::with_capacity(all.len());
    let mut rev: HashMap<AnyId, Vec<AnyId>> = HashMap::new();
    let mut indeg: HashMap<AnyId, usize> = HashMap::with_capacity(all.len());
    for &a in &all {
        indeg.entry(a).or_insert(0);
    }
    for &a in &all {
        let ds = deps(w, a);
        for &d in &ds {
            rev.entry(d).or_default().push(a);
            *indeg.entry(d).or_insert(0) += 1;
        }
        fwd.insert(a, ds);
    }

    // 0/1. downgrade + seed illegal.
    let mut dropped: HashSet<AnyId> = HashSet::new();
    let mut rename: HashMap<AnyId, &'static str> = HashMap::new();
    let mut loss = LossReport::default();
    for &a in &all {
        if let AnyId::ComplexUnit(id) = a {
            if !w.complex_legal(id, profile.legal) {
                dropped.insert(a);
                loss.dropped.push((display_name(a), R_ILLEGAL));
            }
        } else {
            let kw = Writer::name_of(a);
            if !profile.is_legal(kw) {
                if let Some(sup) = profile.downgrade(kw) {
                    rename.insert(a, sup);
                    loss.downgraded.push((kw.to_string(), sup));
                } else {
                    dropped.insert(a);
                    loss.dropped.push((kw.to_string(), R_ILLEGAL));
                }
            }
        }
    }

    // r_pre: reachable from in-degree-0 roots over the full graph.
    let roots: Vec<AnyId> = all
        .iter()
        .copied()
        .filter(|a| indeg.get(a).copied().unwrap_or(0) == 0)
        .collect();
    let r_pre = reach(&roots, &fwd, |_| true);

    // 2. cascade: drop every referrer of a dropped entity (fixpoint).
    let mut work: Vec<AnyId> = dropped.iter().copied().collect();
    while let Some(d) = work.pop() {
        if let Some(referrers) = rev.get(&d) {
            for &r in referrers {
                if dropped.insert(r) {
                    loss.dropped.push((display_name(r), R_CASCADE));
                    work.push(r);
                }
            }
        }
    }

    // 3. orphan prune: in r_pre but not reachable after skipping dropped.
    let live_roots: Vec<AnyId> = roots
        .iter()
        .copied()
        .filter(|a| !dropped.contains(a))
        .collect();
    let r_post = reach(&live_roots, &fwd, |a| !dropped.contains(&a));
    for &a in &all {
        if r_pre.contains(&a) && !r_post.contains(&a) && !dropped.contains(&a) {
            dropped.insert(a);
            loss.dropped.push((display_name(a), R_ORPHAN));
        }
    }

    // Referential closure: no survivor references a dropped entity (else
    // render_one's id_of_ref would panic). Guaranteed by the cascade above.
    debug_assert!(
        all.iter().filter(|a| !dropped.contains(a)).all(|a| fwd
            .get(a)
            .is_none_or(|ds| ds.iter().all(|d| !dropped.contains(d)))),
        "projection left a survivor referencing a dropped entity"
    );

    ProjectionPlan {
        dropped,
        rename,
        loss,
    }
}
