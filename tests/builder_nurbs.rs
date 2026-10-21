//! NURBS authoring through `StepBuilder`: read↔write symmetry — the
//! `NurbsCurve`/`NurbsSurface` structs fed in come back identical from the
//! read side's `to_nurbs()`. Non-rational data lands as plain
//! `B_SPLINE_*_WITH_KNOTS`; rational data as the multi-part complex.

mod common;

use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
use step_io::scene::{CurveKind, SurfaceKind};
use step_io::scene::{NurbsCurve, NurbsSurface};
use step_io::{Orientation, StepBuilder, read};

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
    let mut b = StepBuilder::new();
    let part = b.part("cube");

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
            .map(|id| (*id, Orientation::Forward))
            .or_else(|| e.get(&(b_ix, a_ix)).map(|id| (*id, Orientation::Reversed)))
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
                Orientation::Forward,
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
            Orientation::Forward,
            vec![FaceBoundInput::outer(edges)],
        )
        .expect("nurbs face"),
    );
    b.solid(part, "cube body", faces).expect("solid");
    common::text(&b.finish())
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
            for (edge, _orientation) in bound.oriented_edges() {
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
    assert_eq!(model.all::<step_io::model::ComplexEntity>().len(), 5);

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

    let mut b = StepBuilder::new();
    let part = b.part("pie");
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
            Orientation::Forward,
            vec![FaceBoundInput::outer(vec![
                (spoke_in, Orientation::Forward),
                (rim, Orientation::Forward),
                (spoke_out, Orientation::Forward),
            ])],
        )
        .expect("pie face");
    b.solid(part, "pie", vec![face]).expect("solid");
    let text = common::text(&b.finish());

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    let faces: Vec<_> = solids[0].faces().collect();
    let mut rational_edges = 0;
    for bound in faces[0].bounds() {
        for (edge, _orientation) in bound.oriented_edges() {
            if edge.curve().to_nurbs() == Some(arc.clone()) {
                rational_edges += 1;
            }
        }
    }
    assert_eq!(rational_edges, 1, "rational arc reproduced");
}

#[test]
fn analytic_surface_kinds_round_trip() {
    let mut b = StepBuilder::new();
    let part = b.part("shapes");
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
                Orientation::Forward,
                vec![FaceBoundInput::outer(vec![(rim, Orientation::Forward)])],
            )
            .expect("face"),
        );
    }
    b.solid(part, "shapes", faces).expect("solid");
    let text = common::text(&b.finish());

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

#[test]
fn nurbs_that_break_the_standard_are_rejected() {
    let mut b = StepBuilder::new();
    let v0 = b.vertex([0.0, 0.0, 0.0]).expect("v0");
    let v1 = b.vertex([1.0, 0.0, 0.0]).expect("v1");
    let line = nurbs_line();
    let broken = [
        (
            "degree below 1",
            NurbsCurve {
                degree: 0,
                knots: vec![0.0, 1.0, 2.0],
                ..line.clone()
            },
        ),
        (
            "knot count",
            NurbsCurve {
                knots: vec![0.0, 0.0, 1.0],
                ..line.clone()
            },
        ),
        (
            "knots decrease",
            NurbsCurve {
                knots: vec![0.0, 1.0, 0.0, 1.0],
                ..line.clone()
            },
        ),
        (
            "repeated more often",
            NurbsCurve {
                knots: vec![0.0, 0.0, 0.0, 1.0],
                ..line.clone()
            },
        ),
        (
            "weight not positive",
            NurbsCurve {
                weights: vec![1.0, 0.0],
                ..line.clone()
            },
        ),
        (
            "weights do not match",
            NurbsCurve {
                weights: vec![1.0],
                ..line.clone()
            },
        ),
    ];
    for (why, curve) in broken {
        let err = b
            .edge(v0, v1, CurveInput::Nurbs(curve.clone()))
            .expect_err("a B-spline that breaks the standard");
        assert!(
            matches!(err, step_io::AuthorError::InvalidInput { reason } if reason.contains(why)),
            "{curve:?}: {err}"
        );
    }

    let mut ragged = bilinear_top(1.0);
    ragged.control_points[1].pop();
    let mut decreasing = bilinear_top(1.0);
    decreasing.knots_v = vec![0.0, 1.0, 0.0, 1.0];
    for surface in [ragged, decreasing] {
        let err = b
            .face(SurfaceInput::Nurbs(surface), Orientation::Forward, vec![])
            .expect_err("a B-spline surface that breaks the standard");
        assert!(matches!(err, step_io::AuthorError::InvalidInput { .. }));
    }
}

#[test]
fn parabola_and_hyperbola_edges_round_trip() {
    // F = 2 and semi-axes 3, 1.5, in the frame at (1,2,3).
    let at = frame([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let parabola_at = |u: f64| [1.0 + 2.0 * u * u, 2.0 + 4.0 * u, 3.0];
    let hyperbola_at = |u: f64| [1.0 + 3.0 * u.cosh(), 2.0 + 1.5 * u.sinh(), 3.0];

    let mut b = StepBuilder::new();
    let part = b.part("conics");
    let ends = [
        (parabola_at(-1.0), parabola_at(2.0)),
        (parabola_at(2.0), parabola_at(-1.0)),
        (hyperbola_at(-0.5), hyperbola_at(1.5)),
    ];
    let mut edges = Vec::new();
    for (i, (from, to)) in ends.into_iter().enumerate() {
        let (v0, v1) = (b.vertex(from).expect("v0"), b.vertex(to).expect("v1"));
        let curve = if i < 2 {
            CurveInput::Parabola(at, 2.0)
        } else {
            CurveInput::Hyperbola(at, 3.0, 1.5)
        };
        edges.push(b.edge(v0, v1, curve).expect("edge"));
    }
    // Something to anchor the edges: a face over the plane they lie in.
    let face = b
        .face(
            SurfaceInput::Plane(at),
            Orientation::Forward,
            vec![FaceBoundInput::outer(
                edges.iter().map(|&e| (e, Orientation::Forward)).collect(),
            )],
        )
        .expect("face");
    b.solid(part, "conics", vec![face]).expect("solid");
    let mut out = Vec::new();
    step_io::write(&b.finish(), &mut out).expect("write");

    let (model, report) = read(&out[..]).expect("read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let scene = model.scene();
    let read_back: Vec<_> = scene.all_edges().collect();
    assert_eq!(read_back.len(), 3);
    // The second parabola edge runs against the parameter.
    let senses: Vec<bool> = read_back
        .iter()
        .map(step_io::scene::Edge::same_sense)
        .collect();
    assert_eq!(senses, [true, false, true]);
    for (edge, (from, to)) in read_back.iter().zip(ends) {
        let n = edge.to_nurbs().expect("converts");
        let (first, last) = (n.control_points[0], *n.control_points.last().unwrap());
        assert!(close(first, from) && close(last, to), "{first:?} {last:?}");
        assert!(matches!(
            edge.curve().kind(),
            CurveKind::Parabola(_) | CurveKind::Hyperbola(_)
        ));
    }
}

#[test]
fn a_parabola_with_a_negative_focal_distance_keeps_its_sense() {
    // F = -2: the parameter grows while y falls, so an edge from u = -1 to
    // u = 2 runs with the curve.
    let at = frame([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let flipped_at = |u: f64| [1.0 - 2.0 * u * u, 2.0 - 4.0 * u, 3.0];
    let mut b = StepBuilder::new();
    let part = b.part("flipped");
    let v0 = b.vertex(flipped_at(-1.0)).expect("v0");
    let v1 = b.vertex(flipped_at(2.0)).expect("v1");
    let edge = b
        .edge(v0, v1, CurveInput::Parabola(at, -2.0))
        .expect("edge");
    let face = b
        .face(
            SurfaceInput::Plane(at),
            Orientation::Forward,
            vec![FaceBoundInput::outer(vec![(edge, Orientation::Forward)])],
        )
        .expect("face");
    b.solid(part, "flipped", vec![face]).expect("solid");
    let mut out = Vec::new();
    step_io::write(&b.finish(), &mut out).expect("write");

    let (model, _report) = read(&out[..]).expect("read");
    let scene = model.scene();
    let edge = scene.all_edges().next().expect("the edge");
    assert!(edge.same_sense());
    let n = edge.to_nurbs().expect("converts");
    assert!(close(n.control_points[0], flipped_at(-1.0)));
    assert!(close(n.control_points[2], flipped_at(2.0)));
}

#[test]
fn degenerate_parabolas_and_hyperbolas_are_rejected() {
    let at = frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let mut b = StepBuilder::new();
    let v0 = b.vertex([1.0, 2.0, 0.0]).expect("v0");
    let v1 = b.vertex([1.0, -2.0, 0.0]).expect("v1");
    // A zero focal distance, a zero semi-axis, and an edge without a span.
    for (curve, to) in [
        (CurveInput::Parabola(at, 0.0), v1),
        (CurveInput::Hyperbola(at, 0.0, 1.0), v1),
        (CurveInput::Parabola(at, 1.0), v0),
    ] {
        let err = b.edge(v0, to, curve).expect_err("degenerate");
        assert!(
            matches!(err, step_io::AuthorError::InvalidInput { .. }),
            "{err}"
        );
    }
}

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}
