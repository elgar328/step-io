//! Surface colour and transparency authoring through `StepBuilder::style`,
//! read back through the Scene's presentation accessors on solids and faces.

use step_io::build::Rgb;
use step_io::build::{
    CurveInput, FaceBoundInput, Frame, StyleTarget, SurfaceInput, VoidShellNormals,
};
use step_io::{StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

/// Minimal single-face solid; returns (face, solid).
fn plate(
    b: &mut StepBuilder,
    part: step_io::build::Part,
) -> (
    step_io::generated::model::AdvancedFaceId,
    step_io::generated::model::ManifoldSolidBrepId,
) {
    let v0 = b.vertex([0.0, 0.0, 0.0]).expect("v0");
    let v1 = b.vertex([1.0, 0.0, 0.0]).expect("v1");
    let v2 = b.vertex([1.0, 1.0, 0.0]).expect("v2");
    let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
    let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
    let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
    let face = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (e1, true),
                (e2, true),
            ])],
        )
        .expect("face");
    let body = b.solid(part, "plate body", vec![face]).expect("solid");
    (face, body)
}

#[test]
fn solid_color_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");
    let (_face, body) = plate(&mut b, part);

    let red = Rgb {
        red: 0.8,
        green: 0.2,
        blue: 0.1,
    };
    b.style(StyleTarget::Solid(body), red, None).expect("style");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(
        model
            .mechanical_design_geometric_presentation_representation_arena
            .items
            .len(),
        1,
        "styles anchored in one MDGPR"
    );

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].color(), Some(red));
    assert_eq!(solids[0].transparency(), None);
}

#[test]
fn face_color_and_transparency_round_trip() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");
    let (face, _body) = plate(&mut b, part);

    let grey = Rgb {
        red: 0.5,
        green: 0.5,
        blue: 0.5,
    };
    b.style(StyleTarget::Face(face), grey, Some(0.3))
        .expect("style");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].color(), Some(grey));
    assert_eq!(faces[0].transparency(), Some(0.3));

    // The style targets the face, not the solid.
    assert_eq!(solids[0].color(), None);
    assert_eq!(solids[0].transparency(), None);
}

/// Minimal solid-with-voids: a one-face outer shell and a one-face void shell
/// (read does not enforce topological closure, so this is enough to exercise
/// the `BREP_WITH_VOIDS` style path).
fn void_body(
    b: &mut StepBuilder,
    part: step_io::build::Part,
) -> step_io::generated::model::BrepWithVoidsId {
    let mut tri = |z: f64| {
        let v0 = b.vertex([0.0, 0.0, z]).expect("v0");
        let v1 = b.vertex([1.0, 0.0, z]).expect("v1");
        let v2 = b.vertex([1.0, 1.0, z]).expect("v2");
        let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
        let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
        let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
        b.face(
            SurfaceInput::Plane(frame([0.0, 0.0, z], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (e1, true),
                (e2, true),
            ])],
        )
        .expect("face")
    };
    let outer = tri(0.0);
    let cavity = tri(0.5);
    b.solid_with_voids(
        part,
        "hollow body",
        vec![outer],
        vec![vec![cavity]],
        VoidShellNormals::AwayFromMaterial,
    )
    .expect("void solid")
}

#[test]
fn void_solid_color_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("hollow").expect("part");
    let body = void_body(&mut b, part);

    let red = Rgb {
        red: 0.8,
        green: 0.2,
        blue: 0.1,
    };
    b.style(StyleTarget::VoidSolid(body), red, Some(0.25))
        .expect("style");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    assert!(matches!(
        solids[0].key(),
        step_io::EntityKey::BrepWithVoids(_)
    ));
    assert_eq!(solids[0].color(), Some(red));
    assert_eq!(solids[0].transparency(), Some(0.25));
}

#[test]
fn void_solid_layer_and_hide_round_trip() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("hollow").expect("part");
    let body = void_body(&mut b, part);

    b.layer("BODY", vec![StyleTarget::VoidSolid(body)])
        .expect("layer");
    b.hide(StyleTarget::VoidSolid(body)).expect("hide");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].layer(), Some("BODY"));
    assert!(!solids[0].is_visible(), "hidden void solid");
}
