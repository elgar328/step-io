//! Integration tests for the writer: synthesise IR → write → re-parse → verify.
//!
//! W-A does not target real fixture round-trip (that needs topology). Instead
//! we build the smallest IR instances by hand and check that the re-parsed
//! result matches what we put in.

use step_io::ir::arena::Arena;
use step_io::ir::assembly::{
    AssemblyTree, GroupContent, Instance, Product, ProductContent, SolidContent, Transform3d,
};
use step_io::ir::geometry::Vertex;
use step_io::ir::geometry::{
    Axis1Placement, Axis2Placement3d, Circle3, ConicalSurface, Curve, CurveForm,
    CylindricalSurface, Direction3, Ellipse3, Line3, Logical, NurbsCurve, NurbsKind, NurbsSurface,
    NurbsSurfaceKind, Plane3, Point3, SphericalSurface, Surface, SurfaceForm,
    SurfaceOfLinearExtrusion, SurfaceOfRevolution, ToroidalSurface,
};
use step_io::ir::id::{DirectionId, Placement3dId, PointId, SolidId, UnitContextId};
use step_io::ir::model::StepModel;
use step_io::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit, UnitContext};
use step_io::ir::topology::{Face, FaceKind, Orientation, Shell, Solid, Wire};
use step_io::ir::units::{NamedUnit, UnitsPool};
use step_io::parser::schema::{SchemaClass, StepSchema};
use step_io::reader::ReaderContext;
use step_io::{WriteError, parse};

fn empty_model() -> StepModel {
    StepModel::default()
}

/// units-2: push mm / radian / steradian named-unit arena entries into
/// the model's units pool and return a fully-populated `UnitContext`
/// referencing them.
fn mm_radian_steradian(model: &mut StepModel) -> UnitContext {
    let pool = model.units_pool.get_or_insert_with(UnitsPool::default);
    UnitContext {
        length: pool.push_plain_length(LengthUnit::Millimetre, false),
        plane_angle: pool.push_plain_plane_angle(AngleUnit::Radian, false),
        solid_angle: pool.push_plain_solid_angle(SolidAngleUnit::Steradian, false),
        length_uncertainty: None,
        plane_angle_uncertainty: None,
        solid_angle_uncertainty: None,
    }
}

fn reconvert(text: &str) -> StepModel {
    let graph = parse(text).expect("writer output parses");
    let result = ReaderContext::convert(&graph);
    assert!(
        result.warnings.is_empty(),
        "reader warnings on writer output: {:#?}",
        result.warnings
    );
    result.model
}

#[test]
fn empty_model_produces_valid_part21_wrapper() {
    let model = empty_model();
    let text = model.write_to_string().expect("write");
    assert!(text.starts_with("ISO-10303-21;\n"));
    assert!(text.contains("HEADER;"));
    assert!(text.contains("DATA;\n"));
    assert!(text.ends_with("END-ISO-10303-21;\n"));
    // Re-parseable.
    let _ = reconvert(&text);
}

#[test]
fn points_round_trip_values() {
    let mut model = empty_model();
    model.geometry.points = Arena::default();
    model.geometry.points.push(Point3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    });
    model.geometry.points.push(Point3 {
        x: -0.5,
        y: 0.0,
        z: 1e-7,
    });
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.points.len(), 2);
    let p0 = re.geometry.points.iter().next().unwrap();
    assert!((p0.x - 1.0).abs() < f64::EPSILON);
    assert!((p0.y - 2.0).abs() < f64::EPSILON);
    assert!((p0.z - 3.0).abs() < f64::EPSILON);
    let p1 = re.geometry.points.iter().nth(1).unwrap();
    assert!((p1.x - -0.5).abs() < f64::EPSILON);
    assert!((p1.y - 0.0).abs() < f64::EPSILON);
    assert!((p1.z - 1e-7).abs() < 1e-15);
}

#[test]
fn direction_round_trips_values() {
    let mut model = empty_model();
    model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.directions.len(), 1);
    let d = re.geometry.directions.iter().next().unwrap();
    assert!((d.x - 0.0).abs() < f64::EPSILON);
    assert!((d.y - 0.0).abs() < f64::EPSILON);
    assert!((d.z - 1.0).abs() < f64::EPSILON);
}

#[test]
fn line_with_inline_vector_round_trips() {
    let mut model = empty_model();
    let p = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let d = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    model.geometry.curves.push(Curve::Line(Line3 {
        point: p,
        direction: d,
        magnitude: 2.5,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.points.len(), 1);
    assert_eq!(re.geometry.directions.len(), 1);
    assert_eq!(re.geometry.curves.len(), 1);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Line(line) => {
            assert_eq!(line.point, PointId(0));
            assert_eq!(line.direction, DirectionId(0));
            assert!((line.magnitude - 2.5).abs() < f64::EPSILON);
        }
        other => panic!("expected Line, got {other:?}"),
    }
}

#[test]
fn plane_round_trips_axis_placement() {
    let mut model = empty_model();
    let loc = model.geometry.points.push(Point3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    });
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let refd = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let position = push_placement(&mut model, loc, Some(axis), Some(refd));
    model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.surfaces.len(), 1);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Plane(plane) => {
            let p = re.geometry.placements[plane.position];
            assert_eq!(p.location, loc);
            assert_eq!(p.axis, Some(axis));
            assert_eq!(p.ref_direction, Some(refd));
        }
        other => panic!("expected Plane, got {other:?}"),
    }
}

#[test]
fn cylinder_round_trips_radius_and_placement() {
    let mut model = empty_model();
    let loc = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let refd = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let position = push_placement(&mut model, loc, Some(axis), Some(refd));
    model
        .geometry
        .surfaces
        .push(Surface::Cylinder(CylindricalSurface {
            position,
            radius: 12.5,
        }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.surfaces.len(), 1);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Cylinder(cyl) => {
            assert!((cyl.radius - 12.5).abs() < f64::EPSILON);
            let p = re.geometry.placements[cyl.position];
            assert_eq!(p.location, loc);
        }
        other => panic!("expected Cylinder, got {other:?}"),
    }
}

#[test]
fn unset_axis_directions_round_trip_as_none() {
    let mut model = empty_model();
    let loc = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let position = push_placement(&mut model, loc, None, None);
    model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Plane(plane) => {
            let p = re.geometry.placements[plane.position];
            assert!(p.axis.is_none());
            assert!(p.ref_direction.is_none());
        }
        other => panic!("expected Plane, got {other:?}"),
    }
}

#[test]
fn unit_context_mm_radian_steradian_round_trips() {
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    // units-2: NamedUnitId may shift due to pool emit ordering — compare
    // the resolved enum values via arena lookup.
    let ctx_back = re.units.iter().next().expect("ctx");
    let pool = re.units_pool.as_ref().expect("units pool");
    match pool.named_units[ctx_back.length] {
        NamedUnit::Length(f) => assert_eq!(f.unit, LengthUnit::Millimetre),
        _ => panic!("length not Length"),
    }
    match pool.named_units[ctx_back.plane_angle] {
        NamedUnit::PlaneAngle(f) => assert_eq!(f.unit, AngleUnit::Radian),
        _ => panic!("plane_angle not PlaneAngle"),
    }
    match pool.named_units[ctx_back.solid_angle] {
        NamedUnit::SolidAngle(f) => assert_eq!(f.unit, SolidAngleUnit::Steradian),
        _ => panic!("solid_angle not SolidAngle"),
    }
}

#[test]
fn unit_context_absent_stays_none() {
    let model = empty_model();
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert!(re.units.is_empty());
}

#[test]
fn write_to_and_write_to_string_produce_identical_bytes() {
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let via_string = model.write_to_string().expect("string");
    let mut via_writer = Vec::new();
    model.write_to(&mut via_writer).expect("writer");
    assert_eq!(via_string.as_bytes(), &via_writer[..]);
}

fn xyz_placement(model: &mut StepModel) -> Placement3dId {
    let loc = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let refd = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    model.geometry.placements.push(Axis2Placement3d {
        location: loc,
        axis: Some(axis),
        ref_direction: Some(refd),
    })
}

fn push_placement(
    model: &mut StepModel,
    location: PointId,
    axis: Option<DirectionId>,
    ref_direction: Option<DirectionId>,
) -> Placement3dId {
    model.geometry.placements.push(Axis2Placement3d {
        location,
        axis,
        ref_direction,
    })
}

#[test]
fn circle_round_trips_radius() {
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    model.geometry.curves.push(Curve::Circle(Circle3 {
        position,
        radius: 5.0,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.curves.len(), 1);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Circle(c) => assert!((c.radius - 5.0).abs() < f64::EPSILON),
        other => panic!("expected Circle, got {other:?}"),
    }
}

#[test]
fn ellipse_round_trips_semi_axes() {
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    model.geometry.curves.push(Curve::Ellipse(Ellipse3 {
        position,
        semi_axis_1: 3.0,
        semi_axis_2: 1.5,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Ellipse(e) => {
            assert!((e.semi_axis_1 - 3.0).abs() < f64::EPSILON);
            assert!((e.semi_axis_2 - 1.5).abs() < f64::EPSILON);
        }
        other => panic!("expected Ellipse, got {other:?}"),
    }
}

#[test]
fn sphere_round_trips_radius() {
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    model
        .geometry
        .surfaces
        .push(Surface::Sphere(SphericalSurface {
            position,
            radius: 7.5,
        }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Sphere(s) => assert!((s.radius - 7.5).abs() < f64::EPSILON),
        other => panic!("expected Sphere, got {other:?}"),
    }
}

#[test]
fn cone_round_trips_radius_and_semi_angle() {
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    model.geometry.surfaces.push(Surface::Cone(ConicalSurface {
        position,
        radius: 4.0,
        semi_angle: std::f64::consts::FRAC_PI_6,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Cone(c) => {
            assert!((c.radius - 4.0).abs() < f64::EPSILON);
            assert!((c.semi_angle - std::f64::consts::FRAC_PI_6).abs() < f64::EPSILON);
        }
        other => panic!("expected Cone, got {other:?}"),
    }
}

#[test]
fn torus_round_trips_major_and_minor_radii() {
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    model
        .geometry
        .surfaces
        .push(Surface::Torus(ToroidalSurface {
            position,
            major_radius: 10.0,
            minor_radius: 2.0,
        }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Torus(t) => {
            assert!((t.major_radius - 10.0).abs() < f64::EPSILON);
            assert!((t.minor_radius - 2.0).abs() < f64::EPSILON);
        }
        other => panic!("expected Torus, got {other:?}"),
    }
}

#[test]
fn non_finite_real_surfaces_as_invalid_float() {
    let mut model = empty_model();
    model.geometry.points.push(Point3 {
        x: f64::NAN,
        y: 0.0,
        z: 0.0,
    });
    let err = model.write_to_string().expect_err("NaN must not serialize");
    assert!(matches!(err, WriteError::InvalidFloat { .. }));
}

#[test]
fn assembly_field_defaults_noop() {
    // Assembly isn't in W-A scope; a None value must round-trip untouched.
    let mut model = empty_model();
    model.assembly = None;
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert!(re.assembly.is_none());
    // Silence unused warning on AssemblyTree import.
    let _ = std::mem::size_of::<AssemblyTree>();
}

fn push_surface_control_grid(model: &mut StepModel) -> Vec<Vec<PointId>> {
    // 2 x 2 grid of points in z=0 plane.
    vec![
        vec![
            model.geometry.points.push(Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }),
            model.geometry.points.push(Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
        ],
        vec![
            model.geometry.points.push(Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }),
            model.geometry.points.push(Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            }),
        ],
    ]
}

#[test]
fn nurbs_surface_non_rational_round_trips() {
    let mut model = empty_model();
    let control_points = push_surface_control_grid(&mut model);
    model.geometry.surfaces.push(Surface::Nurbs(NurbsSurface {
        u_degree: 1,
        v_degree: 1,
        control_points,
        kind: NurbsSurfaceKind::NonRational,
        u_knot_multiplicities: vec![2, 2],
        v_knot_multiplicities: vec![2, 2],
        u_knots: vec![0.0, 1.0],
        v_knots: vec![0.0, 1.0],
        u_closed: false,
        v_closed: false,
        form: SurfaceForm::Unspecified,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Nurbs(s) => {
            assert_eq!(s.u_degree, 1);
            assert_eq!(s.v_degree, 1);
            assert!(s.weights().is_none());
            assert_eq!(s.control_points.len(), 2);
            assert_eq!(s.control_points[0].len(), 2);
        }
        other => panic!("expected Nurbs surface, got {other:?}"),
    }
}

#[test]
fn nurbs_surface_rational_round_trips() {
    let mut model = empty_model();
    let control_points = push_surface_control_grid(&mut model);
    model.geometry.surfaces.push(Surface::Nurbs(NurbsSurface {
        u_degree: 1,
        v_degree: 1,
        control_points,
        kind: NurbsSurfaceKind::Rational {
            weights: vec![vec![1.0, 0.8], vec![0.8, 1.0]],
        },
        u_knot_multiplicities: vec![2, 2],
        v_knot_multiplicities: vec![2, 2],
        u_knots: vec![0.0, 1.0],
        v_knots: vec![0.0, 1.0],
        u_closed: false,
        v_closed: false,
        form: SurfaceForm::Unspecified,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Nurbs(s) => {
            let weights = s.weights().expect("rational surface has weights");
            assert_eq!(weights.len(), 2);
            assert!((weights[0][1] - 0.8).abs() < f64::EPSILON);
            assert!((weights[1][0] - 0.8).abs() < f64::EPSILON);
        }
        other => panic!("expected Nurbs surface, got {other:?}"),
    }
}

fn push_linear_control_points(model: &mut StepModel) -> Vec<PointId> {
    vec![
        model.geometry.points.push(Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
        model.geometry.points.push(Point3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }),
        model.geometry.points.push(Point3 {
            x: 2.0,
            y: 1.0,
            z: 0.0,
        }),
    ]
}

#[test]
fn nurbs_curve_non_rational_round_trips() {
    let mut model = empty_model();
    let control_points = push_linear_control_points(&mut model);
    model.geometry.curves.push(Curve::Nurbs(NurbsCurve {
        degree: 2,
        control_points: control_points.clone(),
        kind: NurbsKind::NonRational,
        knot_multiplicities: vec![3, 3],
        knots: vec![0.0, 1.0],
        closed: false,
        form: CurveForm::Unspecified,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Nurbs(c) => {
            assert_eq!(c.degree, 2);
            assert_eq!(c.control_points.len(), 3);
            assert!(c.weights().is_none());
            assert_eq!(c.knot_multiplicities, vec![3, 3]);
            assert_eq!(c.knots, vec![0.0, 1.0]);
            assert!(!c.closed);
        }
        other => panic!("expected Nurbs curve, got {other:?}"),
    }
}

#[test]
fn nurbs_curve_rational_round_trips() {
    let mut model = empty_model();
    let control_points = push_linear_control_points(&mut model);
    model.geometry.curves.push(Curve::Nurbs(NurbsCurve {
        degree: 2,
        control_points,
        kind: NurbsKind::Rational {
            weights: vec![1.0, 0.7, 1.0],
        },
        knot_multiplicities: vec![3, 3],
        knots: vec![0.0, 1.0],
        closed: false,
        form: CurveForm::Unspecified,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Nurbs(c) => {
            let weights = c.weights().expect("rational curve has weights");
            assert_eq!(weights.len(), 3);
            assert!((weights[1] - 0.7).abs() < f64::EPSILON);
        }
        other => panic!("expected Nurbs curve, got {other:?}"),
    }
}

#[test]
fn nurbs_curve_form_hint_round_trips() {
    let mut model = empty_model();
    let control_points = push_linear_control_points(&mut model);
    model.geometry.curves.push(Curve::Nurbs(NurbsCurve {
        degree: 2,
        control_points,
        kind: NurbsKind::NonRational,
        knot_multiplicities: vec![3, 3],
        knots: vec![0.0, 1.0],
        closed: false,
        form: CurveForm::CircularArc,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    assert!(text.contains(".CIRCULAR_ARC."), "writer emits STEP enum");
    let re = reconvert(&text);
    match re.geometry.curves.iter().next().unwrap() {
        Curve::Nurbs(c) => assert_eq!(c.form, CurveForm::CircularArc),
        other => panic!("expected Nurbs curve, got {other:?}"),
    }
}

#[test]
fn nurbs_surface_form_hint_round_trips() {
    let mut model = empty_model();
    let control_points = push_surface_control_grid(&mut model);
    model.geometry.surfaces.push(Surface::Nurbs(NurbsSurface {
        u_degree: 1,
        v_degree: 1,
        control_points,
        kind: NurbsSurfaceKind::NonRational,
        u_knot_multiplicities: vec![2, 2],
        v_knot_multiplicities: vec![2, 2],
        u_knots: vec![0.0, 1.0],
        v_knots: vec![0.0, 1.0],
        u_closed: false,
        v_closed: false,
        form: SurfaceForm::CylindricalSurf,
        self_intersect: Logical::Unknown,
    }));
    let text = model.write_to_string().expect("write");
    assert!(
        text.contains(".CYLINDRICAL_SURF."),
        "writer emits STEP enum"
    );
    let re = reconvert(&text);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Nurbs(s) => assert_eq!(s.form, SurfaceForm::CylindricalSurf),
        other => panic!("expected Nurbs surface, got {other:?}"),
    }
}

#[test]
fn extrusion_surface_round_trips() {
    // A Line extruded along a direction produces a SurfaceOfLinearExtrusion.
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    let _ = position;
    let line_pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let line_dir = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let swept = model.geometry.curves.push(Curve::Line(Line3 {
        point: line_pt,
        direction: line_dir,
        magnitude: 1.0,
    }));
    let extrude_dir = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    });
    model
        .geometry
        .surfaces
        .push(Surface::Extrusion(SurfaceOfLinearExtrusion {
            swept_curve: swept,
            extrusion_direction: extrude_dir,
            depth: 5.0,
        }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.surfaces.len(), 1);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Extrusion(e) => {
            assert_eq!(e.swept_curve, swept);
            assert_eq!(e.extrusion_direction, extrude_dir);
            assert!((e.depth - 5.0).abs() < f64::EPSILON);
        }
        other => panic!("expected Extrusion, got {other:?}"),
    }
}

#[test]
fn revolution_surface_round_trips() {
    // A Circle swept around Z axis produces a torus-like SurfaceOfRevolution.
    let mut model = empty_model();
    let position = xyz_placement(&mut model);
    let swept = model.geometry.curves.push(Curve::Circle(Circle3 {
        position,
        radius: 2.0,
    }));
    let axis_loc = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let axis_dir = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let axis_placement = model.geometry.placements_1d.push(Axis1Placement {
        location: axis_loc,
        axis: axis_dir,
    });
    model
        .geometry
        .surfaces
        .push(Surface::Revolution(SurfaceOfRevolution {
            swept_curve: swept,
            axis_placement,
        }));
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.surfaces.len(), 1);
    match re.geometry.surfaces.iter().next().unwrap() {
        Surface::Revolution(r) => {
            assert_eq!(r.swept_curve, swept);
            let p = re.geometry.placements_1d[r.axis_placement];
            assert_eq!(p.location, axis_loc);
            assert_eq!(p.axis, axis_dir);
        }
        other => panic!("expected Revolution, got {other:?}"),
    }
}

/// Build a degenerate-but-valid Solid: one Plane face bounded by a single
/// `VERTEX_LOOP`. Reused by `vertex_loop_wire_round_trips` and the assembly
/// synthetic tests below.
fn push_minimal_solid(model: &mut StepModel) -> SolidId {
    let pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let refd = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let position = push_placement(model, pt, Some(axis), Some(refd));
    let plane_surface = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let vertex = model.geometry.vertices.push(Vertex { point: pt });
    let wire = model.topology.wires.push(Wire {
        edges: Vec::new(),
        vertex: Some(vertex),
        is_outer: true,
        orientation: Orientation::Forward,
    });
    let face = model.topology.faces.push(Face {
        surface: plane_surface,
        bounds: vec![wire],
        orientation: Orientation::Forward,
        kind: FaceKind::Advanced,
    });
    let shell = model.topology.shells.push(Shell {
        faces: vec![face],
        orientation: Orientation::Forward,
        is_open: false,
    });
    model.topology.solids.push(Solid {
        shells: vec![shell],
        name: None,
    })
}

#[test]
fn vertex_loop_wire_round_trips() {
    // A face whose boundary is a single degenerate VERTEX_LOOP — the shape
    // that sphere poles and some revolutions use.
    let mut model = empty_model();
    let _solid = push_minimal_solid(&mut model);
    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.topology.wires.len(), 1);
    let roundtripped_wire = re.topology.wires.iter().next().unwrap();
    assert!(roundtripped_wire.vertex.is_some());
    assert!(roundtripped_wire.edges.is_empty());
    assert!(roundtripped_wire.is_outer);
}

fn identity_transform(model: &mut StepModel) -> Transform3d {
    let pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let z = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let x = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let placement = push_placement(model, pt, Some(z), Some(x));
    Transform3d {
        source: placement,
        target: placement,
    }
}

#[test]
fn simple_assembly_round_trips() {
    // Root Group holding one Instance that points at a Solid leaf product.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let transform = identity_transform(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let leaf_pid = tree.products.push(Product {
        id: "Leaf".into(),
        name: "Leaf".into(),
        description: None,
        content: ProductContent::Solid(SolidContent {
            ids: vec![solid_id],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    let root_pid = tree.products.push(Product {
        id: "Root".into(),
        name: "Root".into(),
        description: None,
        content: ProductContent::Group(GroupContent {
            instances: vec![Instance {
                child: leaf_pid,
                transform,
                occurrence_id: "1".into(),
                occurrence_name: "LeafInst".into(),
            }],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    tree.root = Some(root_pid);
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    assert_eq!(r_asm.products.len(), 2);
    assert!(r_asm.root.is_some());
    let root_prod = r_asm
        .products
        .iter()
        .find(|p| p.id == "Root")
        .expect("Root product survived");
    match &root_prod.content {
        ProductContent::Group(group) => {
            assert_eq!(group.instances.len(), 1);
            assert_eq!(group.instances[0].occurrence_id, "1");
            assert_eq!(group.instances[0].occurrence_name, "LeafInst");
        }
        other @ (ProductContent::Solid(_)
        | ProductContent::SurfaceBody(_)
        | ProductContent::Wireframe(_)) => {
            panic!("expected Root Group, got {other:?}")
        }
    }
    let leaf_prod = r_asm
        .products
        .iter()
        .find(|p| p.id == "Leaf")
        .expect("Leaf product survived");
    assert!(matches!(leaf_prod.content, ProductContent::Solid(_)));
}

#[test]
fn shared_child_assembly_round_trips() {
    // Same Leaf referenced twice from the Root Group with different
    // occurrence ids — the classic shared-instance case.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let transform = identity_transform(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let leaf_pid = tree.products.push(Product {
        id: "Leaf".into(),
        name: "Leaf".into(),
        description: None,
        content: ProductContent::Solid(SolidContent {
            ids: vec![solid_id],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    let root_pid = tree.products.push(Product {
        id: "Root".into(),
        name: "Root".into(),
        description: None,
        content: ProductContent::Group(GroupContent {
            instances: vec![
                Instance {
                    child: leaf_pid,
                    transform,
                    occurrence_id: "1".into(),
                    occurrence_name: "A".into(),
                },
                Instance {
                    child: leaf_pid,
                    transform,
                    occurrence_id: "2".into(),
                    occurrence_name: "B".into(),
                },
            ],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    tree.root = Some(root_pid);
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let r_asm = re.assembly.as_ref().unwrap();
    let root_prod = r_asm.products.iter().find(|p| p.id == "Root").unwrap();
    match &root_prod.content {
        ProductContent::Group(group) => {
            assert_eq!(group.instances.len(), 2);
            assert_eq!(
                group.instances[0].child, group.instances[1].child,
                "both point at the same Leaf"
            );
            assert_eq!(group.instances[0].occurrence_id, "1");
            assert_eq!(group.instances[1].occurrence_id, "2");
        }
        other @ (ProductContent::Solid(_)
        | ProductContent::SurfaceBody(_)
        | ProductContent::Wireframe(_)) => {
            panic!("expected Root Group, got {other:?}")
        }
    }
}

#[test]
fn default_schema_is_ap214_is() {
    let model = empty_model();
    assert_eq!(model.schema.class(), Some(SchemaClass::Ap214Is));
    assert!(
        model.schema.raw().is_none(),
        "synthetic IR must not carry raw FILE_SCHEMA text"
    );
    let text = model.write_to_string().expect("write");
    // Default-schema output must advertise AP214 IS in FILE_SCHEMA.
    assert!(
        text.contains("AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }"),
        "expected AP214 IS schema descriptor, got: {text}"
    );
    // Round-trip through the reader recognises the class.
    let re = reconvert(&text);
    assert_eq!(re.schema.class(), Some(SchemaClass::Ap214Is));
}

#[test]
fn multi_body_solid_round_trips() {
    // ABSR.items may carry more than one MANIFOLD_SOLID_BREP (multi-body
    // STEP). The reader collects all of them into ProductContent::Solid;
    // the writer emits one MSB ref per SolidId in the items list.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let s1 = push_minimal_solid(&mut model);
    let s2 = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let pid = tree.products.push(Product {
        id: "MultiBody".into(),
        name: "MultiBody".into(),
        description: None,
        content: ProductContent::Solid(SolidContent { ids: vec![s1, s2] }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    tree.root = Some(pid);
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    assert_eq!(
        text.matches("MANIFOLD_SOLID_BREP").count(),
        2,
        "expected two MSB lines in output, got:\n{text}"
    );

    let re = reconvert(&text);
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    let prod = r_asm
        .products
        .iter()
        .find(|p| p.id == "MultiBody")
        .expect("MultiBody product survived");
    match &prod.content {
        ProductContent::Solid(solid) => {
            assert_eq!(solid.ids.len(), 2, "two solids should round-trip");
        }
        other => panic!("expected Solid with 2 ids, got {other:?}"),
    }
}

#[test]
fn metadata_only_product_round_trips_with_none_geometry_context() {
    // A second sibling product with no shape representation in source
    // STEP (NIST "document" style: `Group([])` + `geometry_context:
    // None`). The writer must skip its SR + SDR emission so the
    // re-read IR keeps `geometry_context: None`; falling back to a
    // default context would surface as `Some(UnitContextId(0))` and
    // break round-trip equality.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    tree.products.push(Product {
        id: "Main".into(),
        name: "Main".into(),
        description: None,
        content: ProductContent::Solid(SolidContent {
            ids: vec![solid_id],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    tree.products.push(Product {
        id: "MetadataDoc".into(),
        name: "MetadataDoc".into(),
        description: None,
        content: ProductContent::Group(GroupContent { instances: vec![] }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: None,
        product_context: None,
        pdef_context: None,
    });
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    // Skip the shared `reconvert` helper: two-root-candidate assemblies
    // legitimately emit a non-fatal "using the first" warning that is
    // unrelated to the geometry_context behaviour under test.
    let graph = parse(&text).expect("writer output parses");
    let re = ReaderContext::convert(&graph).model;
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    assert_eq!(r_asm.products.len(), 2);
    let meta = r_asm
        .products
        .iter()
        .find(|p| p.id == "MetadataDoc")
        .expect("metadata product survived");
    assert!(
        meta.geometry_context.is_none(),
        "metadata product must keep geometry_context: None; got {:?}",
        meta.geometry_context
    );
}

#[test]
fn empty_group_product_preserves_non_identity_shape_ref_frame() {
    // Empty-Group child product (NIST-style raw-material / placeholder
    // sub-assembly) carries its own placement via `shape_ref_frame`. The
    // writer emits a plain SHAPE_REPRESENTATION with that axis; on re-read
    // the SDR pass must pull the placement out of `plain_sr_frame_map`,
    // not leave it at the PRODUCT-pass placeholder Placement3dId(0).
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();
    let offset_loc = model.geometry.points.push(Point3 {
        x: 4.0,
        y: -2.0,
        z: 7.5,
    });
    let offset_axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let offset_ref = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let offset_frame = push_placement(&mut model, offset_loc, Some(offset_axis), Some(offset_ref));

    let mut tree = AssemblyTree::default();
    tree.products.push(Product {
        id: "Main".into(),
        name: "Main".into(),
        description: None,
        content: ProductContent::Solid(SolidContent {
            ids: vec![solid_id],
        }),
        shape_ref_frame: identity_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    tree.products.push(Product {
        id: "Placeholder".into(),
        name: "Placeholder".into(),
        description: None,
        content: ProductContent::Group(GroupContent { instances: vec![] }),
        shape_ref_frame: offset_frame,
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: Some(UnitContextId(0)),
        product_context: None,
        pdef_context: None,
    });
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    let graph = parse(&text).expect("writer output parses");
    let re = ReaderContext::convert(&graph).model;
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    let ph = r_asm
        .products
        .iter()
        .find(|p| p.id == "Placeholder")
        .expect("placeholder product survived");
    let frame = re
        .geometry
        .placements
        .iter()
        .nth(ph.shape_ref_frame.0 as usize)
        .expect("frame id resolves");
    let loc = re
        .geometry
        .points
        .iter()
        .nth(frame.location.0 as usize)
        .expect("location point resolves");
    assert!(
        (loc.x - 4.0).abs() < 1e-9 && (loc.y + 2.0).abs() < 1e-9 && (loc.z - 7.5).abs() < 1e-9,
        "expected offset (4, -2, 7.5), got ({}, {}, {})",
        loc.x,
        loc.y,
        loc.z
    );
}

#[test]
fn explicit_ap203_schema_round_trips() {
    let mut model = empty_model();
    model.schema = StepSchema::canonical(SchemaClass::Ap203);
    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("CONFIG_CONTROL_DESIGN"),
        "expected AP203 FILE_SCHEMA string, got: {text}"
    );
    let re = reconvert(&text);
    assert_eq!(re.schema.class(), Some(SchemaClass::Ap203));
}
