//! The low-level escape hatch: `StepBuilder::author()` reaches the strict
//! constructor layer for anything the builder does not cover, and ids flow
//! in both directions — here a layer assignment (no builder API) is
//! authored low-level onto a builder-produced face, and comes back through
//! the read side's `layer()`.

use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
use step_io::generated::model as m;
use step_io::{StepBuilder, read};

#[test]
fn low_level_layer_on_builder_face_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");

    let v0 = b.vertex([0.0, 0.0, 0.0]).expect("v0");
    let v1 = b.vertex([1.0, 0.0, 0.0]).expect("v1");
    let v2 = b.vertex([1.0, 1.0, 0.0]).expect("v2");
    let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
    let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
    let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
    let face = b
        .face(
            SurfaceInput::Plane(Frame {
                origin: [0.0; 3],
                axis: [0.0, 0.0, 1.0],
                ref_dir: [1.0, 0.0, 0.0],
            }),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (e1, true),
                (e2, true),
            ])],
        )
        .expect("face");
    b.solid(part, "plate body", vec![face]).expect("solid");

    // The builder has no layer API — drop to the strict layer, feeding it
    // the builder-produced face id.
    b.author()
        .add_presentation_layer_assignment(
            "L1".to_owned(),
            String::new(),
            vec![m::LayeredItemRef::AdvancedFace(face)],
        )
        .expect("layer assignment");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // The low-level authored layer surfaces through the read handles.
    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces[0].layer(), Some("L1"));
}
