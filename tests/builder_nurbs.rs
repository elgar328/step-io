//! NURBS authoring through `StepBuilder`: read↔write symmetry — the
//! `NurbsCurve`/`NurbsSurface` structs fed in come back identical from the
//! read side's `to_nurbs()`. Non-rational data lands as plain
//! `B_SPLINE_*_WITH_KNOTS`; rational data as the multi-part complex.

use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
use step_io::build::{NurbsCurve, NurbsSurface};
use step_io::scene::geometry::{CurveKind, SurfaceKind};
use step_io::{StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

/// Bilinear degree-1 patch spanning the unit cube's top (geometrically the
/// z=1 plane square).
fn bilinear_top(weight: f64) -> NurbsSurface {
    NurbsSurface {
        degree_u: 1,
        degree_v: 1,
        control_points: vec![
            vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
            vec![[1.0, 0.0, 1.0], [1.0, 1.0, 1.0]],
        ],
        weights: vec![vec![weight; 2]; 2],
        knots_u: vec![0.0, 0.0, 1.0, 1.0],
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
    }
}

/// The cube from the b-rep test, with the top face over `top_patch` and one
/// bottom edge as a degree-1 NURBS line. Returns the emitted text.
fn nurbs_cube(top_patch: NurbsSurface, nurbs_edge: &NurbsCurve) -> String {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("cube").expect("part");

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
    let pairs = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let mut e = std::collections::HashMap::new();
    for (a_ix, b_ix) in pairs {
        // The (0,1) bottom-X edge is authored as a NURBS line.
        let curve = if (a_ix, b_ix) == (0, 1) {
            CurveInput::Nurbs(nurbs_edge.clone())
        } else {
            CurveInput::Line
        };
        e.insert((a_ix, b_ix), b.edge(v[a_ix], v[b_ix], curve).expect("edge"));
    }
    let edge = |a_ix: usize, b_ix: usize| {
        e.get(&(a_ix, b_ix))
            .map(|id| (*id, true))
            .or_else(|| e.get(&(b_ix, a_ix)).map(|id| (*id, false)))
            .expect("edge exists")
    };

    let plane_faces: [([usize; 4], Frame); 5] = [
        (
            [0, 2, 3, 1],
            frame([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ),
        (
            [0, 1, 5, 4],
            frame([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        ),
        (
            [2, 6, 7, 3],
            frame([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ),
        (
            [0, 4, 6, 2],
            frame([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ),
        (
            [1, 3, 7, 5],
            frame([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ),
    ];
    let mut faces = Vec::new();
    for (loop_ixs, f) in plane_faces {
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
    // Top face over the NURBS patch.
    let top_loop = [4, 5, 7, 6];
    let edges = (0..4)
        .map(|i| edge(top_loop[i], top_loop[(i + 1) % 4]))
        .collect();
    faces.push(
        b.face(
            SurfaceInput::Nurbs(top_patch),
            true,
            vec![FaceBoundInput::outer(edges)],
        )
        .expect("nurbs face"),
    );
    b.solid(part, "cube body", faces).expect("solid");
    b.finish().expect("finish")
}

fn nurbs_line() -> NurbsCurve {
    NurbsCurve {
        degree: 1,
        control_points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        weights: vec![1.0, 1.0],
        knots: vec![0.0, 0.0, 1.0, 1.0],
    }
}

#[test]
fn non_rational_nurbs_round_trips_identically() {
    let patch = bilinear_top(1.0);
    let line = nurbs_line();
    let text = nurbs_cube(patch.clone(), &line);

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 6);

    // The top face is a plain B_SPLINE_SURFACE_WITH_KNOTS and reproduces
    // the input patch exactly.
    let nurbs_faces: Vec<_> = faces
        .iter()
        .filter(|f| matches!(f.surface().kind(), SurfaceKind::BSplineWithKnots(_)))
        .collect();
    assert_eq!(nurbs_faces.len(), 1);
    assert_eq!(nurbs_faces[0].surface().to_nurbs(), Some(patch));

    // The NURBS edge reproduces the input curve exactly.
    let mut nurbs_edges = 0;
    for face in &faces {
        for bound in face.bounds() {
            for (edge, _forward) in bound.oriented_edges() {
                if matches!(edge.curve().kind(), CurveKind::BSplineWithKnots(_)) {
                    nurbs_edges += 1;
                    assert_eq!(edge.curve().to_nurbs(), Some(line.clone()));
                }
            }
        }
    }
    // The bottom-X edge is shared by two faces.
    assert_eq!(nurbs_edges, 2);
}

#[test]
fn rational_nurbs_surface_round_trips_via_complex() {
    let patch = bilinear_top(2.0); // uniform non-1 weights: rational path
    let text = nurbs_cube(patch.clone(), &nurbs_line());

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // Baseline complexes: 3 unit complexes + 1 context complex + 1 rational
    // surface complex.
    assert_eq!(model.complex_unit_arena.items.len(), 5);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    let rational: Vec<_> = faces
        .iter()
        .filter(|f| f.surface().to_nurbs() == Some(patch.clone()))
        .collect();
    assert_eq!(rational.len(), 1, "rational patch reproduced (weights 2.0)");
}

#[test]
fn rational_nurbs_curve_round_trips_via_complex() {
    // Pie slice on the z=0 plane: two straight spokes + a rational quarter
    // circle (radius 1) from (1,0,0) to (0,1,0).
    let arc = NurbsCurve {
        degree: 2,
        control_points: vec![[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        weights: vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0],
        knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
    };

    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("pie").expect("part");
    let center = b.vertex([0.0, 0.0, 0.0]).expect("center");
    let start = b.vertex([1.0, 0.0, 0.0]).expect("start");
    let end = b.vertex([0.0, 1.0, 0.0]).expect("end");
    let spoke_in = b.edge(center, start, CurveInput::Line).expect("spoke in");
    let rim = b
        .edge(start, end, CurveInput::Nurbs(arc.clone()))
        .expect("rim");
    let spoke_out = b.edge(end, center, CurveInput::Line).expect("spoke out");
    let face = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![FaceBoundInput::outer(vec![
                (spoke_in, true),
                (rim, true),
                (spoke_out, true),
            ])],
        )
        .expect("pie face");
    b.solid(part, "pie", vec![face]).expect("solid");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    let mut rational_edges = 0;
    for bound in faces[0].bounds() {
        for (edge, _forward) in bound.oriented_edges() {
            if edge.curve().to_nurbs() == Some(arc.clone()) {
                rational_edges += 1;
            }
        }
    }
    assert_eq!(rational_edges, 1, "rational arc reproduced");
}

#[test]
fn analytic_surface_kinds_round_trip() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("shapes").expect("part");
    let z = [0.0, 0.0, 1.0];
    let x = [1.0, 0.0, 0.0];

    // Each face bounded by a circle lying on its surface.
    let mut faces = Vec::new();
    for (surface, ring) in [
        (SurfaceInput::Sphere(frame([0.0; 3], z, x), 2.0), 2.0), // equator
        (SurfaceInput::Torus(frame([0.0; 3], z, x), 5.0, 1.0), 6.0), // outer equator
        (SurfaceInput::Cone(frame([0.0; 3], z, x), 3.0, 0.5), 3.0), // base rim
    ] {
        let v = b.vertex([ring, 0.0, 0.0]).expect("vertex");
        let rim = b
            .edge(v, v, CurveInput::Circle(frame([0.0; 3], z, x), ring))
            .expect("rim");
        faces.push(
            b.face(
                surface,
                true,
                vec![FaceBoundInput::outer(vec![(rim, true)])],
            )
            .expect("face"),
        );
    }
    b.solid(part, "shapes", faces).expect("solid");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let kinds: Vec<bool> = solids[0]
        .faces()
        .map(|f| {
            matches!(
                f.surface().kind(),
                SurfaceKind::Spherical(_) | SurfaceKind::Toroidal(_) | SurfaceKind::Conical(_)
            )
        })
        .collect();
    assert_eq!(kinds, [true, true, true]);
}
