//! B-rep authoring through `StepBuilder`: cube (planes + lines) and
//! cylinder (circles + cylindrical surface, shared/reused edges), read back
//! through the Scene.

use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
use step_io::scene::geometry::{CurveKind, SurfaceKind};
use step_io::{StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn cube_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("cube").expect("part");

    // 8 corners of the unit cube: index = x + 2y + 4z.
    let mut v = Vec::new();
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                v.push(
                    b.vertex([f64::from(x), f64::from(y), f64::from(z)])
                        .expect("vertex"),
                );
            }
        }
    }

    // 12 edges, keyed by their corner index pair (shared between faces).
    let pairs = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7), // along X
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7), // along Y
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7), // along Z
    ];
    let mut e = std::collections::HashMap::new();
    for (a_ix, b_ix) in pairs {
        let id = b.edge(v[a_ix], v[b_ix], CurveInput::Line).expect("edge");
        e.insert((a_ix, b_ix), id);
    }
    let edge = |a_ix: usize, b_ix: usize| {
        e.get(&(a_ix, b_ix))
            .map(|id| (*id, true))
            .or_else(|| e.get(&(b_ix, a_ix)).map(|id| (*id, false)))
            .expect("edge exists")
    };

    // 6 faces: corner loop (counter-clockwise seen from outside) + plane.
    let faces_spec: [([usize; 4], Frame); 6] = [
        (
            [0, 2, 3, 1],
            frame([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ), // bottom
        (
            [4, 5, 7, 6],
            frame([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ), // top
        (
            [0, 1, 5, 4],
            frame([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        ), // front
        (
            [2, 6, 7, 3],
            frame([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ), // back
        (
            [0, 4, 6, 2],
            frame([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ), // left
        (
            [1, 3, 7, 5],
            frame([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ), // right
    ];
    let mut faces = Vec::new();
    for (loop_ixs, f) in faces_spec {
        let edges = (0..4)
            .map(|i| edge(loop_ixs[i], loop_ixs[(i + 1) % 4]))
            .collect();
        faces.push(
            b.face(
                SurfaceInput::Plane(f),
                true,
                vec![FaceBoundInput::outer(edges)],
            )
            .expect("face"),
        );
    }
    b.solid(part, "cube body", faces).expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // Promotion: the solid-bearing part writes an ABSR, not a plain SR.
    assert_eq!(
        model.advanced_brep_shape_representation_arena.items.len(),
        1
    );
    assert_eq!(model.shape_representation_arena.items.len(), 0);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 6);
    for face in &faces {
        assert!(matches!(face.surface().kind(), SurfaceKind::Plane(_)));
        let bounds: Vec<_> = face.bounds().collect();
        assert_eq!(bounds.len(), 1);
        assert!(bounds[0].is_outer());
        let edges: Vec<_> = bounds[0].oriented_edges().collect();
        assert_eq!(edges.len(), 4);
        for (edge, _forward) in edges {
            assert!(matches!(edge.curve().kind(), CurveKind::Line(_)));
        }
    }
}

#[test]
fn cylinder_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("pin").expect("part");

    let (radius, height) = (2.0, 5.0);
    let z_dir = [0.0, 0.0, 1.0];
    let x_dir = [1.0, 0.0, 0.0];

    // Seam vertices on the bottom and top circles.
    let v_bot = b.vertex([radius, 0.0, 0.0]).expect("v bot");
    let v_top = b.vertex([radius, 0.0, height]).expect("v top");

    // Full-circle edges (from == to) and the straight seam edge.
    let bottom = b
        .edge(
            v_bot,
            v_bot,
            CurveInput::Circle(frame([0.0; 3], z_dir, x_dir), radius),
        )
        .expect("bottom circle");
    let top = b
        .edge(
            v_top,
            v_top,
            CurveInput::Circle(frame([0.0, 0.0, height], z_dir, x_dir), radius),
        )
        .expect("top circle");
    let seam = b.edge(v_bot, v_top, CurveInput::Line).expect("seam");

    // Lateral face: the seam edge is used twice, in opposite directions.
    let lateral = b
        .face(
            SurfaceInput::Cylinder(frame([0.0; 3], z_dir, x_dir), radius),
            true,
            vec![FaceBoundInput::outer(vec![
                (bottom, true),
                (seam, true),
                (top, false),
                (seam, false),
            ])],
        )
        .expect("lateral face");
    let bottom_cap = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, -1.0], x_dir)),
            true,
            vec![FaceBoundInput::outer(vec![(bottom, false)])],
        )
        .expect("bottom cap");
    let top_cap = b
        .face(
            SurfaceInput::Plane(frame([0.0, 0.0, height], z_dir, x_dir)),
            true,
            vec![FaceBoundInput::outer(vec![(top, true)])],
        )
        .expect("top cap");
    b.solid(part, "pin body", vec![lateral, bottom_cap, top_cap])
        .expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 3);

    let mut cylindrical = 0;
    let mut planes = 0;
    for face in &faces {
        match face.surface().kind() {
            SurfaceKind::Cylindrical(_) => cylindrical += 1,
            SurfaceKind::Plane(_) => planes += 1,
            other => panic!("unexpected surface kind: {other:?}"),
        }
        for bound in face.bounds() {
            for (edge, _forward) in bound.oriented_edges() {
                assert!(matches!(
                    edge.curve().kind(),
                    CurveKind::Line(_) | CurveKind::Circle(_)
                ));
            }
        }
    }
    assert_eq!((cylindrical, planes), (1, 2));

    // The lateral loop reuses the seam edge twice: 4 oriented edges.
    let lateral_face = faces
        .iter()
        .find(|f| matches!(f.surface().kind(), SurfaceKind::Cylindrical(_)))
        .unwrap();
    let lateral_edges: Vec<_> = lateral_face
        .bounds()
        .next()
        .unwrap()
        .oriented_edges()
        .collect();
    assert_eq!(lateral_edges.len(), 4);
}

#[test]
fn empty_bounds_and_faces_are_rejected() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");

    let err = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![],
        )
        .expect_err("a face without bounds is schema-invalid");
    assert!(matches!(
        err,
        step_io::AuthorError::Cardinality {
            entity: "ADVANCED_FACE",
            attribute: "bounds",
            ..
        }
    ));

    let err = b
        .solid(part, "empty", vec![])
        .expect_err("a shell without faces is schema-invalid");
    assert!(matches!(
        err,
        step_io::AuthorError::Cardinality {
            entity: "CLOSED_SHELL",
            attribute: "cfs_faces",
            ..
        }
    ));
}
