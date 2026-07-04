//! Assembly authoring through `StepBuilder::place`: shared definitions with
//! per-instance placements, read back through the Scene's occurrence chain.

use step_io::build::Frame;
use step_io::{StepBuilder, read};

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
    let text = b.finish().expect("finish");

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
    let text = b.finish().expect("finish");

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
