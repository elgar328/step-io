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

    // 2. Reverse-ref index (referenced id -> ids that reference it), then BFS:
    //    any entity referencing a dropped id is itself dropped (to fixpoint).
    let mut parents_of: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut refs_scratch: Vec<u64> = Vec::new();
    for e in entities.iter() {
        refs_scratch.clear();
        collect_refs(e, &mut refs_scratch);
        for &child in &refs_scratch {
            parents_of.entry(child).or_default().push(e.id);
        }
    }
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

    // 3. Record drops (in stable emit order) and retain survivors. Distinguish
    //    the seed (directly illegal in the target) from the cascade (legal but
    //    referencing a dropped entity) so the loss report is actionable.
    for e in entities.iter() {
        if dropped.contains(&e.id) {
            let reason = if seed.contains(&e.id) {
                "dropped (illegal in target schema)"
            } else {
                "dropped (references a dropped entity — cascade)"
            };
            report.record(entity_name(e), reason);
        }
    }
    entities.retain(|e| !dropped.contains(&e.id));

    // 4. Guard: the cascade guarantees no survivor references a dropped id.
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
}
