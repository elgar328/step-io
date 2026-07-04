//! The ergonomic builder: `StepBuilder` assembles the standard AP242 file
//! skeleton (contexts, SI units, product chains, shape anchors) and its
//! output navigates cleanly through the read-side `Scene`.

use step_io::{StepBuilder, read};

#[test]
fn builder_skeleton_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    b.part("wheel").expect("wheel");
    b.part("axle").expect("axle");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // The read-side Scene is the consumer contract: products and units must
    // come back exactly as authored.
    let scene = model.scene();
    let names: Vec<&str> = scene.all_products().map(|p| p.name()).collect();
    assert_eq!(names, ["wheel", "axle"]);

    let units = scene.units();
    let length = units.length.expect("length unit");
    assert!((length.to_si - 0.001).abs() < 1e-12, "mm: {}", length.to_si);
    let angle = units.angle.expect("angle unit");
    assert!((angle.to_si - 1.0).abs() < 1e-12, "radian: {}", angle.to_si);
    assert_eq!(units.precision, Some(1.0e-7));

    // Skeleton entities present: header stamping targets and the per-part
    // shape chain.
    assert_eq!(model.application_context_arena.items.len(), 1);
    assert_eq!(model.application_protocol_definition_arena.items.len(), 1);
    assert_eq!(model.shape_definition_representation_arena.items.len(), 2);
    assert_eq!(model.shape_representation_arena.items.len(), 2);
    assert_eq!(model.product_related_product_category_arena.items.len(), 1);
}

#[test]
fn builder_with_no_parts_still_emits_a_valid_file() {
    let b = StepBuilder::new().expect("builder");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert!(model.product_arena.items.is_empty());
    // The unit context survives as a root: units still readable.
    assert!(model.scene().units().length.is_some());
}

#[test]
fn metre_units_and_custom_uncertainty_round_trip() {
    use step_io::build::{LengthUnit, UnitsInput};

    let mut b = StepBuilder::new_with(&UnitsInput {
        length: LengthUnit::Metre,
        uncertainty: 1e-5,
    })
    .expect("builder");
    b.part("hull").expect("part");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let units = model.scene().units();
    let length = units.length.expect("length unit");
    assert!(
        (length.to_si - 1.0).abs() < 1e-12,
        "metre: {}",
        length.to_si
    );
    assert_eq!(units.precision, Some(1e-5));
}
