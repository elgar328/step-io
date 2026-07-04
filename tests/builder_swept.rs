//! Ellipse edges and swept surfaces (linear extrusion / revolution) — the
//! last analytic forms the read side exposes that the builder can author.

use step_io::build::{CurveInput, FaceBoundInput, Frame, ProfileInput, SurfaceInput};
use step_io::generated::model as m;
use step_io::scene::geometry::{CurveKind, SurfaceKind};
use step_io::{StepBuilder, read};

const Z: [f64; 3] = [0.0, 0.0, 1.0];
const X: [f64; 3] = [1.0, 0.0, 0.0];

fn frame(origin: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis: Z,
        ref_dir: X,
    }
}

#[test]
fn ellipse_edge_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");

    // A planar face bounded by one full-ellipse edge.
    let v = b.vertex([3.0, 0.0, 0.0]).expect("vertex");
    let rim = b
        .edge(v, v, CurveInput::Ellipse(frame([0.0; 3]), 3.0, 2.0))
        .expect("ellipse edge");
    let face = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3])),
            true,
            vec![FaceBoundInput::outer(vec![(rim, true)])],
        )
        .expect("face");
    b.solid(part, "plate", vec![face]).expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    let edges: Vec<_> = faces[0].bounds().next().unwrap().oriented_edges().collect();
    assert!(matches!(edges[0].0.curve().kind(), CurveKind::Ellipse(_)));
}

/// Cylinder-shaped topology (two circle rims + a seam) over the given
/// lateral surface, so swept surfaces get geometrically consistent bounds.
fn cylinder_topology(b: &mut StepBuilder, lateral_surface: SurfaceInput, r: f64, h: f64) {
    let part = b.part("swept").expect("part");
    let v_bot = b.vertex([r, 0.0, 0.0]).expect("v bot");
    let v_top = b.vertex([r, 0.0, h]).expect("v top");
    let bottom = b
        .edge(v_bot, v_bot, CurveInput::Circle(frame([0.0; 3]), r))
        .expect("bottom");
    let top = b
        .edge(v_top, v_top, CurveInput::Circle(frame([0.0, 0.0, h]), r))
        .expect("top");
    let seam = b.edge(v_bot, v_top, CurveInput::Line).expect("seam");
    let lateral = b
        .face(
            lateral_surface,
            true,
            vec![FaceBoundInput::outer(vec![
                (bottom, true),
                (seam, true),
                (top, false),
                (seam, false),
            ])],
        )
        .expect("lateral");
    b.solid(part, "swept body", vec![lateral]).expect("solid");
}

#[test]
fn linear_extrusion_surface_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    cylinder_topology(
        &mut b,
        SurfaceInput::LinearExtrusion(ProfileInput::Circle(frame([0.0; 3]), 2.0), [0.0, 0.0, 5.0]),
        2.0,
        5.0,
    );

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    assert!(matches!(
        faces[0].surface().kind(),
        SurfaceKind::LinearExtrusion(_)
    ));
    assert!(
        faces[0].to_nurbs().is_some(),
        "read-side extrusion patch conversion"
    );
}

#[test]
fn revolution_surface_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    cylinder_topology(
        &mut b,
        SurfaceInput::Revolution(
            ProfileInput::Line([2.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ),
        2.0,
        5.0,
    );

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    assert!(matches!(
        faces[0].surface().kind(),
        SurfaceKind::Revolution(_)
    ));
}

#[test]
fn polyline_edge_round_trips() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("bracket").expect("part");

    // Triangle face whose hypotenuse is a 3-point polyline (with a bend).
    let v0 = b.vertex([0.0, 0.0, 0.0]).expect("v0");
    let v1 = b.vertex([4.0, 0.0, 0.0]).expect("v1");
    let v2 = b.vertex([0.0, 4.0, 0.0]).expect("v2");
    let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
    let bend = b
        .edge(
            v1,
            v2,
            CurveInput::Polyline(vec![[4.0, 0.0, 0.0], [3.0, 3.0, 0.0], [0.0, 4.0, 0.0]]),
        )
        .expect("polyline edge");
    let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
    let face = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3])),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (bend, true),
                (e2, true),
            ])],
        )
        .expect("face");
    b.solid(part, "bracket", vec![face]).expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    let polylines: Vec<Vec<[f64; 3]>> = faces[0]
        .bounds()
        .next()
        .unwrap()
        .oriented_edges()
        .filter_map(|(e, _)| match e.curve().kind() {
            CurveKind::Polyline(p) => Some(
                p.points
                    .iter()
                    .map(|r| {
                        let m::CartesianPointRef::CartesianPoint(id) = r else {
                            panic!("unexpected point ref");
                        };
                        let c = &model.cartesian_point_arena.get(id.0).coordinates;
                        [c[0], c[1], c[2]]
                    })
                    .collect(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        polylines,
        vec![vec![[4.0, 0.0, 0.0], [3.0, 3.0, 0.0], [0.0, 4.0, 0.0]]]
    );
}

#[test]
fn single_point_polyline_is_rejected() {
    let mut b = StepBuilder::new().expect("builder");
    let _ = b.part("p").expect("part");
    let v = b.vertex([0.0, 0.0, 0.0]).expect("v");
    let err = b
        .edge(v, v, CurveInput::Polyline(vec![[0.0, 0.0, 0.0]]))
        .expect_err("one point is below LIST [2:?]");
    assert!(matches!(
        err,
        step_io::AuthorError::Cardinality {
            entity: "POLYLINE",
            attribute: "points",
            got: 1,
            min: 2,
            ..
        }
    ));
}
