//! Schema projection — the post-build prune pass.
//!
//! After `emit_all` builds the full `Vec<WriterEntity>`, [`project`] drops every
//! entity illegal in the target schema, then cascade-drops anything that
//! references a dropped entity (so no dangling `#N` survives), recording each
//! drop in a [`LossReport`]. The output-side counterpart of the input-side
//! `NonStandardInput` normalization.
//!
//! `Universal` (the as-is target) is a no-op: entities are left untouched and an
//! empty report is returned — so the default write path is byte-identical to the
//! pre-projection behaviour.
//!
//! Cascade direction note: PMI/annotation/tolerance entities reference the core
//! geometry/product they annotate (PMI → core), not vice versa. So dropping a
//! PMI cluster never cascades *up* into core geometry. The only over-drop a
//! conservative (drop-every-referencer) cascade can cause is a legal entity that
//! *optionally* references an illegal one — measured on the corpus, refined to an
//! optionality-aware rewrite later if it matters.

use std::collections::{HashMap, HashSet};

use crate::early::profile::{LossReport, SchemaProfile};
use crate::parser::entity::Attribute;
use crate::writer::entity::{WriterBody, WriterEntity};

/// Drop target-illegal entities + their referencers from `entities`, returning
/// what was dropped. No-op for `Universal`.
pub(crate) fn project(entities: &mut Vec<WriterEntity>, profile: &SchemaProfile) -> LossReport {
    let mut report = LossReport::default();
    if profile.is_universal() {
        return report;
    }

    // 0. Downgrade: rename a target-illegal subtype to its legal supertype when
    //    the rename is lossless (subtype adds no attributes, e.g. DESIGN_CONTEXT
    //    which is absent from AP214 -> PRODUCT_DEFINITION_CONTEXT). This keeps
    //    product-management from cascade-collapsing on cross-AP projection:
    //    PRODUCT_DEFINITION references the context via frame_of_reference, so
    //    dropping the subtype would take the whole product chain with it.
    //    Renaming before the seed step makes the entity legal so it survives.
    //    The downgrade table is baked from the .exp inheritance graph
    //    (`AP*_DOWNGRADE`): every target-illegal subtype that reaches a
    //    target-legal ancestor via a rename-safe (attribute-identical) chain.
    for e in entities.iter_mut() {
        if let WriterBody::Simple { name, .. } = &mut e.body {
            if !profile.is_legal(name) {
                if let Some(supertype) = profile.downgrade(name) {
                    *name = supertype.to_owned();
                }
            }
        }
    }

    // 1. Seed: entities whose own name (simple) or any part (complex) is illegal.
    let seed: HashSet<u64> = entities
        .iter()
        .filter(|e| !entity_legal(e, profile))
        .map(|e| e.id)
        .collect();
    if seed.is_empty() {
        return report;
    }
    let mut dropped = seed.clone();

    // 2. Forward + reverse ref indices over the pre-projection graph (one pass).
    //    parents_of (child -> referencers) drives the cascade; children_of
    //    (entity -> referenced) drives the reachability passes.
    let mut parents_of: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut children_of: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut refs_scratch: Vec<u64> = Vec::new();
    for e in entities.iter() {
        refs_scratch.clear();
        collect_refs(e, &mut refs_scratch);
        for &child in &refs_scratch {
            parents_of.entry(child).or_default().push(e.id);
            children_of.entry(e.id).or_default().push(child);
        }
    }

    // Roots = entities nothing references (in-degree 0), frozen before any drop.
    // R_pre = set reachable from roots over the full pre-projection graph.
    let roots: Vec<u64> = entities
        .iter()
        .map(|e| e.id)
        .filter(|id| !parents_of.contains_key(id))
        .collect();
    let r_pre = reach(&roots, &children_of, |_| true);

    // Cascade: any entity referencing a dropped id is itself dropped (fixpoint).
    let mut queue: Vec<u64> = dropped.iter().copied().collect();
    while let Some(id) = queue.pop() {
        if let Some(parents) = parents_of.get(&id) {
            for &p in parents {
                if dropped.insert(p) {
                    queue.push(p);
                }
            }
        }
    }

    // 3. Reachability prune: an entity reachable from a root before projection
    //    (r_pre) but not after (r_post, skipping dropped nodes) is an orphan the
    //    projection stranded — drop it so the first write already equals the
    //    settled write. Entities never root-reachable (isolated cycles; input
    //    orphans are themselves roots) stay untouched — faithful preservation.
    let r_post = reach(&roots, &children_of, |id| !dropped.contains(&id));
    let mut orphans: HashSet<u64> = HashSet::new();
    for e in entities.iter() {
        if !dropped.contains(&e.id) && r_pre.contains(&e.id) && !r_post.contains(&e.id) {
            orphans.insert(e.id);
        }
    }
    for &id in &orphans {
        dropped.insert(id);
    }

    // 4. Record drops (stable emit order) by reason, then retain survivors.
    for e in entities.iter() {
        if dropped.contains(&e.id) {
            let reason = if seed.contains(&e.id) {
                "dropped (illegal in target schema)"
            } else if orphans.contains(&e.id) {
                "dropped (unreachable after projection)"
            } else {
                "dropped (references a dropped entity — cascade)"
            };
            report.record(entity_name(e), reason);
        }
    }
    entities.retain(|e| !dropped.contains(&e.id));

    // 5. Guard: the cascade guarantees no survivor references a dropped id.
    debug_assert!(
        {
            let mut r = Vec::new();
            entities.iter().all(|e| {
                r.clear();
                collect_refs(e, &mut r);
                r.iter().all(|id| !dropped.contains(id))
            })
        },
        "projection left a dangling reference",
    );

    report
}

/// BFS-reachable set from `roots` over the `children_of` forward edges,
/// visiting only nodes for which `allow` holds (the post-projection pass passes
/// `allow = not dropped` to skip dropped nodes).
fn reach(
    roots: &[u64],
    children_of: &HashMap<u64, Vec<u64>>,
    allow: impl Fn(u64) -> bool,
) -> HashSet<u64> {
    let mut seen: HashSet<u64> = HashSet::new();
    let mut queue: Vec<u64> = Vec::new();
    for &r in roots {
        if allow(r) && seen.insert(r) {
            queue.push(r);
        }
    }
    while let Some(id) = queue.pop() {
        if let Some(children) = children_of.get(&id) {
            for &c in children {
                if allow(c) && seen.insert(c) {
                    queue.push(c);
                }
            }
        }
    }
    seen
}

/// An entity is legal iff its name (simple) or every constituent part name
/// (complex) is legal in the target.
fn entity_legal(e: &WriterEntity, profile: &SchemaProfile) -> bool {
    match &e.body {
        WriterBody::Simple { name, .. } => profile.is_legal(name),
        WriterBody::Complex { parts } => parts.iter().all(|(n, _)| profile.is_legal(n)),
    }
}

/// Display name for the loss report: the simple name, or the `+`-joined part
/// names for a complex entity.
fn entity_name(e: &WriterEntity) -> String {
    match &e.body {
        WriterBody::Simple { name, .. } => name.clone(),
        WriterBody::Complex { parts } => parts
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join("+"),
    }
}

/// Collect every `EntityRef` id this entity references (recursing into lists and
/// typed values).
fn collect_refs(e: &WriterEntity, out: &mut Vec<u64>) {
    match &e.body {
        WriterBody::Simple { attrs, .. } => {
            for a in attrs {
                collect_refs_attr(a, out);
            }
        }
        WriterBody::Complex { parts } => {
            for (_, attrs) in parts {
                for a in attrs {
                    collect_refs_attr(a, out);
                }
            }
        }
    }
}

fn collect_refs_attr(a: &Attribute, out: &mut Vec<u64>) {
    match a {
        Attribute::EntityRef(id) => out.push(*id),
        Attribute::List(items) => {
            for it in items {
                collect_refs_attr(it, out);
            }
        }
        Attribute::Typed { value, .. } => collect_refs_attr(value, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::SchemaTarget;

    fn simple(id: u64, name: &str, refs: &[u64]) -> WriterEntity {
        WriterEntity {
            id,
            body: WriterBody::Simple {
                name: name.into(),
                attrs: refs.iter().map(|&r| Attribute::EntityRef(r)).collect(),
            },
        }
    }

    #[test]
    fn universal_is_noop() {
        let mut ents = vec![
            simple(1, "CARTESIAN_POINT", &[]),
            simple(2, "DATUM_SYSTEM", &[]),
        ];
        let before = ents.clone();
        let report = project(
            &mut ents,
            &SchemaProfile::for_target(SchemaTarget::Universal),
        );
        assert_eq!(ents, before);
        assert!(report.dropped().is_empty());
    }

    #[test]
    fn ap203_drops_illegal_and_cascades() {
        // DATUM_SYSTEM is AP242-only (illegal in AP203). A legal entity that
        // references it cascade-drops; an unrelated legal entity survives.
        let mut ents = vec![
            simple(1, "CARTESIAN_POINT", &[]), // legal, no refs -> survives
            simple(2, "DATUM_SYSTEM", &[]),    // illegal in AP203 -> dropped
            simple(3, "REPRESENTATION", &[2]), // legal but refs #2 -> cascade dropped
        ];
        let report = project(&mut ents, &SchemaProfile::for_target(SchemaTarget::Ap203));
        let surviving: Vec<u64> = ents.iter().map(|e| e.id).collect();
        assert_eq!(
            surviving,
            vec![1],
            "only the unrelated legal entity survives"
        );
        assert_eq!(report.dropped().len(), 2);
    }

    #[test]
    fn complex_illegal_if_any_part_illegal() {
        // A complex whose parts are all AP203-legal survives; one with an
        // AP242-only part drops.
        let legal = WriterEntity {
            id: 1,
            body: WriterBody::Complex {
                parts: vec![("CARTESIAN_POINT".into(), vec![]), ("POINT".into(), vec![])],
            },
        };
        let illegal = WriterEntity {
            id: 2,
            body: WriterBody::Complex {
                parts: vec![
                    ("SHAPE_ASPECT".into(), vec![]),
                    ("DATUM_SYSTEM".into(), vec![]),
                ],
            },
        };
        let mut ents = vec![legal, illegal];
        let report = project(&mut ents, &SchemaProfile::for_target(SchemaTarget::Ap203));
        assert_eq!(ents.iter().map(|e| e.id).collect::<Vec<_>>(), vec![1]);
        assert_eq!(report.dropped().len(), 1);
    }

    #[test]
    fn reachability_prunes_projection_orphan_keeps_input_orphan() {
        // #10 (legal PDR) references illegal #20 -> #10 cascade-drops. #30 (legal
        // REPRESENTATION) was reachable only via #10 -> stranded -> reachability
        // prunes it. #40 (legal, never referenced = input orphan = root) survives.
        let mut ents = vec![
            WriterEntity {
                id: 10,
                body: WriterBody::Simple {
                    name: "PROPERTY_DEFINITION_REPRESENTATION".into(),
                    attrs: vec![Attribute::EntityRef(20), Attribute::EntityRef(30)],
                },
            },
            simple(20, "DATUM_SYSTEM", &[]),
            simple(30, "REPRESENTATION", &[]),
            simple(40, "CARTESIAN_POINT", &[]),
        ];
        let report = project(&mut ents, &SchemaProfile::for_target(SchemaTarget::Ap203));
        assert_eq!(
            ents.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![40],
            "only the input orphan (root) survives"
        );
        assert_eq!(report.dropped().len(), 3); // #20 seed, #10 cascade, #30 orphan
    }

    #[test]
    fn reachability_keeps_isolated_cycle() {
        // #50 <-> #60 form an externally-unreferenced cycle (input orphan cycle).
        // Neither is in-degree-0, but neither is root-reachable (r_pre false), so
        // the prune leaves them — faithful preservation. #70 (illegal) seed-drops
        // to force the projection to run.
        let mut ents = vec![
            simple(50, "REPRESENTATION", &[60]),
            simple(60, "REPRESENTATION", &[50]),
            simple(70, "DATUM_SYSTEM", &[]),
        ];
        let report = project(&mut ents, &SchemaProfile::for_target(SchemaTarget::Ap203));
        assert_eq!(
            ents.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![50, 60],
            "isolated cycle preserved"
        );
        assert_eq!(report.dropped().len(), 1); // only #70
    }
}
