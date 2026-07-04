//! Layer and visibility authoring: `layer()` / `hide()` / `hide_layer()`
//! read back through the Scene's `layer()` and `is_visible()`.

use step_io::build::Rgb;
use step_io::build::{CurveInput, FaceBoundInput, Frame, StyleTarget, SurfaceInput};
use step_io::{StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

/// Three separate triangular faces + a solid over them.
fn three_face_plate(
    b: &mut StepBuilder,
    part: step_io::build::Part,
) -> (
    Vec<step_io::generated::model::AdvancedFaceId>,
    step_io::generated::model::ManifoldSolidBrepId,
) {
    let mut faces = Vec::new();
    for i in 0..3 {
        let base = f64::from(i) * 2.0;
        let v0 = b.vertex([base, 0.0, 0.0]).expect("v0");
        let v1 = b.vertex([base + 1.0, 0.0, 0.0]).expect("v1");
        let v2 = b.vertex([base, 1.0, 0.0]).expect("v2");
        let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
        let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
        let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
        faces.push(
            b.face(
                SurfaceInput::Plane(frame([base, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
                true,
                vec![FaceBoundInput::outer(vec![
                    (e0, true),
                    (e1, true),
                    (e2, true),
                ])],
            )
            .expect("face"),
        );
    }
    let solid = b.solid(part, "plate", faces.clone()).expect("solid");
    (faces, solid)
}

#[test]
fn layers_round_trip() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");
    let (faces, solid) = three_face_plate(&mut b, part);

    b.layer(
        "L1",
        vec![StyleTarget::Face(faces[0]), StyleTarget::Face(faces[1])],
    )
    .expect("face layer");
    b.layer("BODY", vec![StyleTarget::Solid(solid)])
        .expect("solid layer");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids[0].layer(), Some("BODY"));
    let layers: Vec<Option<&str>> = solids[0].faces().map(|f| f.layer()).collect();
    assert_eq!(layers, [Some("L1"), Some("L1"), None]);
}

#[test]
fn hide_solid_keeps_color_and_face_visibility() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");
    let (_faces, solid) = three_face_plate(&mut b, part);

    let red = Rgb {
        red: 0.8,
        green: 0.2,
        blue: 0.1,
    };
    b.style(StyleTarget::Solid(solid), red, None)
        .expect("style");
    b.hide(StyleTarget::Solid(solid)).expect("hide");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.invisibility_arena.items.len(), 1);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert!(!solids[0].is_visible(), "solid hidden");
    assert_eq!(solids[0].color(), Some(red), "colour coexists with hide");

    // Visibility is per key: hiding the solid does not cascade to faces.
    for face in solids[0].faces() {
        assert!(face.is_visible());
    }
}

#[test]
fn hide_layer_hides_its_members() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");
    let (faces, _solid) = three_face_plate(&mut b, part);

    let l1 = b
        .layer(
            "L1",
            vec![StyleTarget::Face(faces[0]), StyleTarget::Face(faces[1])],
        )
        .expect("layer");
    b.hide_layer(l1);

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let visible: Vec<bool> = solids[0].faces().map(|f| f.is_visible()).collect();
    assert_eq!(visible, [false, false, true]);
}
