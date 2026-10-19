//! Assembly authoring through `StepBuilder::place`: shared definitions with
//! per-instance placements, read back through the Scene's occurrence chain.

mod common;

use std::collections::HashMap;

use step_io::build::Frame;
use step_io::parser::{Attribute, RawEntity, parse};
use step_io::{AuthorError, StepBuilder, read};

fn assert_close3(actual: [f64; 3], expected: [f64; 3]) {
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-12, "{actual:?} != {expected:?}");
    }
}

const Z: [f64; 3] = [0.0, 0.0, 1.0];
const X: [f64; 3] = [1.0, 0.0, 0.0];

fn at(origin: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis: Z,
        ref_dir: X,
    }
}

#[test]
fn assembly_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let asm = b.part("assembly").expect("assembly");
    let wheel = b.part("wheel").expect("wheel");
    b.place(asm, wheel, at([10.0, 20.0, 30.0])).expect("place");
    let text = common::text(&b.finish().expect("finish"));

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );

    // The child is used in the assembly, so only "assembly" remains a root.
    let roots: Vec<_> = scene.root_definitions().collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(
        roots[0].product().map(|p| p.name().to_owned()).as_deref(),
        Some("assembly")
    );

    let occurrences: Vec<_> = roots[0].occurrences().collect();
    assert_eq!(occurrences.len(), 1);
    let child = occurrences[0].definition().expect("child definition");
    assert_eq!(
        child.product().map(|p| p.name().to_owned()).as_deref(),
        Some("wheel")
    );

    let t = occurrences[0].transform().expect("transform");
    assert_close3(t.to.origin, [10.0, 20.0, 30.0]);
    assert_close3(t.from.origin, [0.0, 0.0, 0.0]);

    // Pure translation: identity rotation, translation column [10,20,30].
    let mtx = t.matrix().expect("matrix");
    assert_close3([mtx[0][3], mtx[1][3], mtx[2][3]], [10.0, 20.0, 30.0]);
    for (r, row) in mtx.iter().enumerate().take(3) {
        for (c, v) in row.iter().enumerate().take(3) {
            let expected = if r == c { 1.0 } else { 0.0 };
            assert!((v - expected).abs() < 1e-12, "rotation[{r}][{c}] = {v}");
        }
    }
}

#[test]
fn shared_definition_places_multiple_instances() {
    let mut b = StepBuilder::new().expect("builder");
    let asm = b.part("axle").expect("axle");
    let wheel = b.part("wheel").expect("wheel");
    b.place(asm, wheel, at([0.0, 700.0, 0.0])).expect("left");
    b.place(asm, wheel, at([0.0, -700.0, 0.0])).expect("right");
    let text = common::text(&b.finish().expect("finish"));

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // One shared definition, two placed occurrences.
    assert_eq!(model.product_arena.items.len(), 2);
    assert_eq!(model.next_assembly_usage_occurrence_arena.items.len(), 2);
    assert_eq!(
        model
            .context_dependent_shape_representation_arena
            .items
            .len(),
        2
    );

    let scene = model.scene();
    let roots: Vec<_> = scene.root_definitions().collect();
    assert_eq!(roots.len(), 1);
    let mut origins: Vec<[f64; 3]> = roots[0]
        .occurrences()
        .filter_map(|o| o.transform().map(|t| t.to.origin))
        .collect();
    origins.sort_by(|a, b| a[1].partial_cmp(&b[1]).unwrap());
    assert_close3(origins[0], [0.0, -700.0, 0.0]);
    assert_close3(origins[1], [0.0, 700.0, 0.0]);
}

/// The id an attribute refers to.
fn id_of(a: &Attribute) -> u64 {
    match a {
        Attribute::EntityRef(id) => *id,
        other => panic!("expected a reference, got {other:?}"),
    }
}

#[test]
fn placements_relate_representations_in_their_own_contexts() {
    // top > sub > leaf (twice), and top > leaf.
    let mut b = StepBuilder::new().expect("builder");
    let top = b.part("top").expect("top");
    let sub = b.part("sub").expect("sub");
    let leaf = b.part("leaf").expect("leaf");
    b.place(sub, leaf, at([10.0, 0.0, 0.0])).expect("place");
    b.place(sub, leaf, at([0.0, 10.0, 0.0])).expect("place");
    b.place(top, sub, at([0.0, 0.0, 10.0])).expect("place");
    b.place(top, leaf, at([20.0, 0.0, 0.0])).expect("place");
    let text = common::text(&b.finish().expect("finish"));
    let graph = parse(text.as_bytes()).expect("parse");

    // Representation id -> (its items, its context).
    let mut reps = HashMap::new();
    for (&id, e) in &graph.entities {
        let RawEntity::Simple {
            name, attributes, ..
        } = e
        else {
            continue;
        };
        if name != "SHAPE_REPRESENTATION" {
            continue;
        }
        let Attribute::List(items) = &attributes[1] else {
            panic!("items: {:?}", attributes[1]);
        };
        let items: Vec<u64> = items.iter().map(id_of).collect();
        reps.insert(id, (items, id_of(&attributes[2])));
    }

    let mut placements = 0;
    for e in graph.entities.values() {
        let RawEntity::Complex { parts, .. } = e else {
            continue;
        };
        let part = |name: &str| parts.iter().find(|p| p.name == name);
        let (Some(rr), Some(rrwt)) = (
            part("REPRESENTATION_RELATIONSHIP"),
            part("REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION"),
        ) else {
            continue;
        };
        let rep_1 = &reps[&id_of(&rr.attributes[2])];
        let rep_2 = &reps[&id_of(&rr.attributes[3])];
        // RRWT WR1: the two representations are in different contexts.
        assert_ne!(rep_1.1, rep_2.1);
        // RRWT WR2: the transformation goes from an item of `rep_1` to an
        // item of `rep_2`.
        let RawEntity::Simple {
            name, attributes, ..
        } = &graph.entities[&id_of(&rrwt.attributes[0])]
        else {
            panic!("transformation is not a simple entity");
        };
        assert_eq!(name, "ITEM_DEFINED_TRANSFORMATION");
        assert!(rep_1.0.contains(&id_of(&attributes[2])));
        assert!(rep_2.0.contains(&id_of(&attributes[3])));
        placements += 1;
    }
    assert_eq!(placements, 4);
}

#[test]
fn placements_that_would_make_an_assembly_contain_itself_are_refused() {
    let mut b = StepBuilder::new().expect("builder");
    let top = b.part("top").expect("top");
    let sub = b.part("sub").expect("sub");
    let leaf = b.part("leaf").expect("leaf");
    let cyclic = |r| matches!(r, Err(AuthorError::CyclicAssembly));

    assert!(cyclic(b.place(top, top, at([0.0; 3]))));
    b.place(top, sub, at([0.0; 3])).expect("top > sub");
    assert!(cyclic(b.place(sub, top, at([0.0; 3]))));
    b.place(sub, leaf, at([0.0; 3])).expect("sub > leaf");
    assert!(cyclic(b.place(leaf, top, at([0.0; 3]))));

    // A part shared under two parents, and the same child placed again.
    b.place(top, leaf, at([1.0, 0.0, 0.0])).expect("top > leaf");
    b.place(sub, leaf, at([2.0, 0.0, 0.0]))
        .expect("sub > leaf again");

    // The refused placements left nothing behind.
    let text = common::text(&b.finish().expect("finish"));
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.next_assembly_usage_occurrence_arena.items.len(), 4);
    let scene = model.scene();
    let roots: Vec<_> = scene.root_definitions().collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].occurrences().count(), 2);
}
