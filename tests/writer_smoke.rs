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
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Circle3, ConicalSurface, Curve, CurveForm,
    CylindricalSurface, Direction3, Ellipse3, Line3, Logical, NurbsCurve, NurbsKind, NurbsSurface,
    NurbsSurfaceKind, PlanarBox, PlanarBoxPlacement, PlanarExtent, PlanarExtentData, Plane3,
    Point2, Point3, SphericalSurface, Surface, SurfaceForm, SurfaceOfLinearExtrusion,
    SurfaceOfRevolution, ToroidalSurface,
};
use step_io::ir::id::{
    DirectionId, GeneralPropertyId, Placement3dId, PointId, PropertyId, SolidId, UnitContextId,
};
use step_io::ir::model::StepModel;
use step_io::ir::pmi::{PmiPool, ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier};
use step_io::ir::property::{
    DerivedDefinitionItem, GeneralProperty, GeneralPropertyAssociation, Property, PropertyPool,
};
use step_io::ir::shape_rep::{
    AllAroundShapeAspect, AngleUnit, CentreOfSymmetry, CompositeGroupShapeAspect, LengthUnit,
    SolidAngleUnit, UnitContext,
};
use step_io::ir::topology::{Face, FaceKind, Orientation, Shell, Solid, Wire};
use step_io::ir::units::{MassFlavor, MassUnit, NamedUnit, UnitsPool};
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
        length: pool.push_plain_length(LengthUnit::Millimetre),
        plane_angle: pool.push_plain_plane_angle(AngleUnit::Radian),
        solid_angle: pool.push_plain_solid_angle(SolidAngleUnit::Steradian),
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
        representation_id: None,
        outer_representation_id: None,
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![root_pid];
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    assert_eq!(r_asm.products.len(), 2);
    assert_eq!(r_asm.roots.len(), 1, "single-root assembly");
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
        representation_id: None,
        outer_representation_id: None,
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![root_pid];
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![pid];
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
        representation_id: None,
        outer_representation_id: None,
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
        representation_id: None,
        outer_representation_id: None,
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
        representation_id: None,
        outer_representation_id: None,
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
        representation_id: None,
        outer_representation_id: None,
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

#[test]
fn general_property_and_association_round_trip() {
    // AP242 user-defined attribute: a GENERAL_PROPERTY defines the
    // attribute, a GENERAL_PROPERTY_ASSOCIATION binds it to a product's
    // PROPERTY_DEFINITION.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![part_pid];
    model.assembly = Some(tree);

    let mut pool = PropertyPool::default();
    pool.properties.push(Property {
        name: "p1".into(),
        description: Some("user defined attribute".into()),
        target: part_pid,
        representation_name: String::new(),
        context: Some(UnitContextId(0)),
        items: Vec::new(),
    });
    pool.general_properties.push(GeneralProperty {
        id: String::new(),
        name: "SACHNUMMER".into(),
        description: Some("user defined attribute".into()),
    });
    pool.general_property_associations
        .push(GeneralPropertyAssociation {
            name: "user defined attribute".into(),
            description: None,
            base_definition: GeneralPropertyId(0),
            derived_definition: DerivedDefinitionItem::PropertyDefinition(PropertyId(0)),
        });
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    assert_eq!(re_pool.general_properties.len(), 1);
    assert_eq!(re_pool.general_property_associations.len(), 1);

    let gp = re_pool.general_properties.iter().next().unwrap();
    assert_eq!(gp.name, "SACHNUMMER");
    assert_eq!(gp.description.as_deref(), Some("user defined attribute"));

    let gpa = re_pool.general_property_associations.iter().next().unwrap();
    assert_eq!(gpa.name, "user defined attribute");
    assert_eq!(gpa.description, None);
    assert_eq!(gpa.base_definition, GeneralPropertyId(0));
    assert_eq!(
        gpa.derived_definition,
        DerivedDefinitionItem::PropertyDefinition(PropertyId(0))
    );
}

#[test]
fn multi_root_independent_products_round_trip() {
    // A STEP file may hold several independent top-level products with no
    // NAUO between them. `AssemblyTree.roots` lists all of them; the reader
    // must not warn (the old "N root candidates" warning was a spurious
    // LOSS trigger). `reconvert` asserts the reader produced no warnings.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let mut pids = Vec::new();
    for name in ["PartA", "PartB"] {
        let solid_id = push_minimal_solid(&mut model);
        let pid = tree.products.push(Product {
            id: name.into(),
            name: name.into(),
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
            representation_id: None,
            outer_representation_id: None,
        });
        pids.push(pid);
    }
    tree.roots = pids.clone();
    model.assembly = Some(tree);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let r_asm = re.assembly.as_ref().expect("round-tripped has assembly");
    assert_eq!(r_asm.products.len(), 2);
    assert_eq!(r_asm.roots, pids, "both independent products are roots");
}

#[test]
fn gram_conversion_based_unit_round_trips() {
    // A gram defined as a CONVERSION_BASED_UNIT (0.001 of the SI kilogram).
    // The reader must recognize the 'GRAM' CBU name, not just 'POUND'.
    // `reconvert` asserts the reader produced no warnings.
    let mut model = empty_model();
    let mut pool = UnitsPool::default();
    let kg = pool.named_units.push(NamedUnit::Mass(MassFlavor {
        unit: MassUnit::Kilogram,
        cbu_base: None,
    }));
    pool.named_units.push(NamedUnit::Mass(MassFlavor {
        unit: MassUnit::Gram,
        cbu_base: Some(kg),
    }));
    model.units_pool = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re.units_pool.as_ref().expect("round-tripped units pool");
    assert_eq!(re_pool.named_units.len(), 2);
    let gram = re_pool
        .named_units
        .iter()
        .find_map(|n| match n {
            NamedUnit::Mass(f) if f.unit == MassUnit::Gram => Some(f),
            _ => None,
        })
        .expect("gram NamedUnit survived round-trip");
    assert!(
        gram.cbu_base.is_some(),
        "gram round-trips as a CBU-wrapped unit"
    );
}

#[test]
fn shape_aspect_subtypes_round_trip() {
    // COMPOSITE_GROUP_SHAPE_ASPECT / CENTRE_OF_SYMMETRY /
    // ALL_AROUND_SHAPE_ASPECT — SHAPE_ASPECT subtypes, each its own arena.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![part_pid];
    model.assembly = Some(tree);

    model
        .composite_group_shape_aspects
        .push(CompositeGroupShapeAspect {
            name: "cg".into(),
            description: String::new(),
            target: part_pid,
            product_definitional: false,
        });
    model.centre_of_symmetries.push(CentreOfSymmetry {
        name: "cs".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: true,
    });
    model.all_around_shape_aspects.push(AllAroundShapeAspect {
        name: "aa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.composite_group_shape_aspects.len(), 1);
    assert_eq!(re.centre_of_symmetries.len(), 1);
    assert_eq!(re.all_around_shape_aspects.len(), 1);

    let cg = re.composite_group_shape_aspects.iter().next().unwrap();
    assert_eq!(cg.name, "cg");
    assert_eq!(cg.target, step_io::ProductId(0));
    let cs = re.centre_of_symmetries.iter().next().unwrap();
    assert_eq!(cs.name, "cs");
    assert!(
        cs.product_definitional,
        "centre_of_symmetry .T. round-trips"
    );
    let aa = re.all_around_shape_aspects.iter().next().unwrap();
    assert_eq!(aa.name, "aa");
}

#[test]
fn planar_extent_and_box_round_trip() {
    // PLANAR_EXTENT (base) + PLANAR_BOX with a 3D placement and another
    // with a 2D placement — one concrete_supertype arena.
    let mut model = empty_model();
    let frame3d = model.geometry.identity_placement();
    let p2 = model.geometry.points_2d.push(Point2 { x: 0.0, y: 0.0 });
    let frame2d = model.geometry.placements_2d.push(Axis2Placement2d {
        location: p2,
        ref_direction: None,
    });

    model
        .geometry
        .planar_extents
        .push(PlanarExtent::Itself(PlanarExtentData {
            name: "pe".into(),
            size_in_x: 10.0,
            size_in_y: 20.0,
        }));
    model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "pb3d".into(),
            size_in_x: 1.0,
            size_in_y: 2.0,
            placement: PlanarBoxPlacement::Placement3d(frame3d),
        }));
    model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "pb2d".into(),
            size_in_x: 3.0,
            size_in_y: 4.0,
            placement: PlanarBoxPlacement::Placement2d(frame2d),
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.geometry.planar_extents.len(), 3);
    let mut it = re.geometry.planar_extents.iter();
    match it.next().unwrap() {
        PlanarExtent::Itself(d) => {
            assert_eq!(d.name, "pe");
            assert!((d.size_in_x - 10.0).abs() < f64::EPSILON);
            assert!((d.size_in_y - 20.0).abs() < f64::EPSILON);
        }
        PlanarExtent::PlanarBox(pb) => panic!("expected Itself, got {pb:?}"),
    }
    match it.next().unwrap() {
        PlanarExtent::PlanarBox(pb) => {
            assert_eq!(pb.name, "pb3d");
            assert!(matches!(pb.placement, PlanarBoxPlacement::Placement3d(_)));
        }
        PlanarExtent::Itself(d) => panic!("expected PlanarBox, got {d:?}"),
    }
    match it.next().unwrap() {
        PlanarExtent::PlanarBox(pb) => {
            assert_eq!(pb.name, "pb2d");
            assert!(matches!(pb.placement, PlanarBoxPlacement::Placement2d(_)));
        }
        PlanarExtent::Itself(d) => panic!("expected PlanarBox, got {d:?}"),
    }
}

#[test]
fn pmi_primitives_round_trip() {
    // TOLERANCE_ZONE_FORM / TYPE_QUALIFIER / VALUE_FORMAT_TYPE_QUALIFIER —
    // the first pmi-pool entities, each a 1-attr string primitive.
    let mut model = empty_model();
    let mut pmi = PmiPool::default();
    pmi.tolerance_zone_forms.push(ToleranceZoneForm {
        name: "cylindrical".into(),
    });
    pmi.type_qualifiers.push(TypeQualifier {
        name: "maximum".into(),
    });
    pmi.value_format_type_qualifiers
        .push(ValueFormatTypeQualifier {
            format_type: "NR2 1.3".into(),
        });
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("round-tripped pmi pool");
    assert_eq!(re_pmi.tolerance_zone_forms.len(), 1);
    assert_eq!(re_pmi.type_qualifiers.len(), 1);
    assert_eq!(re_pmi.value_format_type_qualifiers.len(), 1);
    assert_eq!(
        re_pmi.tolerance_zone_forms.iter().next().unwrap().name,
        "cylindrical"
    );
    assert_eq!(
        re_pmi.type_qualifiers.iter().next().unwrap().name,
        "maximum"
    );
    assert_eq!(
        re_pmi
            .value_format_type_qualifiers
            .iter()
            .next()
            .unwrap()
            .format_type,
        "NR2 1.3"
    );
}

#[test]
fn mapped_item_round_trip() {
    // REPRESENTATION_MAP + MAPPED_ITEM — orphan round-trip: a reusable map
    // into a representation, instantiated by a mapped item. Both emit
    // standalone (no container modelled yet).
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{
        MappedItem, MappedItemData, PlainRepr, Representation, RepresentationMap,
        RepresentationMapData,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
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
    let placement = push_placement(&mut model, loc, Some(axis), Some(refd));
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "mapped".into(),
        context: Some(uc),
        frame: None,
    }));
    let rmap = model
        .representation_maps
        .push(RepresentationMap::Itself(RepresentationMapData {
            mapping_origin: RepresentationItemRef::Placement3d(placement),
            mapped_representation: rep,
        }));
    model.mapped_items.push(MappedItem::Itself(MappedItemData {
        name: "inst".into(),
        mapping_source: rmap,
        mapping_target: RepresentationItemRef::Placement3d(placement),
    }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.representation_maps.len(), 1);
    assert_eq!(re.mapped_items.len(), 1);
    let MappedItem::Itself(mi) = re.mapped_items.iter().next().unwrap();
    assert_eq!(mi.name, "inst");
    assert!(matches!(
        mi.mapping_target,
        RepresentationItemRef::Placement3d(_)
    ));
    let RepresentationMap::Itself(rm) = re.representation_maps.iter().next().unwrap();
    assert!(matches!(
        rm.mapping_origin,
        RepresentationItemRef::Placement3d(_)
    ));
}

#[test]
fn annotation_plane_round_trip() {
    // ANNOTATION_PLANE — a styled_item PMI subtype. orphan round-trip:
    // name + styles + item(a PLANE surface); `elements` is not modelled.
    use step_io::ir::pmi::{AnnotationOccurrence, AnnotationPlane};
    use step_io::ir::representation_item::RepresentationItemRef;
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
    let surf = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let mut pmi = PmiPool::default();
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
            name: "Linear Size.1".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
        }));
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.annotation_occurrences.len(), 1);
    let AnnotationOccurrence::AnnotationPlane(ap) =
        re_pmi.annotation_occurrences.iter().next().unwrap();
    assert_eq!(ap.name, "Linear Size.1");
    assert!(ap.styles.is_empty());
    assert!(matches!(ap.item, RepresentationItemRef::Surface(_)));
}

#[test]
fn datum_round_trip() {
    // DATUM — a shape_aspect subtype + identification, resolving of_shape
    // to a ProductId through the PRODUCT_DEFINITION_SHAPE chain.
    use step_io::ir::pmi::Datum;
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
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
        representation_id: None,
        outer_representation_id: None,
    });
    tree.roots = vec![part_pid];
    model.assembly = Some(tree);

    let mut pmi = PmiPool::default();
    pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.datums.len(), 1);
    let datum = re_pmi.datums.iter().next().unwrap();
    assert_eq!(datum.identification, "A");
    assert_eq!(datum.target, step_io::ProductId(0));
    assert!(!datum.product_definitional);
}

#[test]
fn draughting_pre_defined_text_font_round_trip() {
    // DRAUGHTING_PRE_DEFINED_TEXT_FONT — a pmi-pool 1-string leaf primitive.
    use step_io::ir::pmi::DraughtingPreDefinedTextFont;
    let mut model = empty_model();
    let mut pmi = PmiPool::default();
    pmi.draughting_pre_defined_text_fonts
        .push(DraughtingPreDefinedTextFont {
            name: "standard".into(),
        });
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.draughting_pre_defined_text_fonts.len(), 1);
    assert_eq!(
        re_pmi
            .draughting_pre_defined_text_fonts
            .iter()
            .next()
            .unwrap()
            .name,
        "standard"
    );
}

#[test]
fn document_file_six_attributes_round_trip() {
    // DOCUMENT_FILE is SUBTYPE OF (document, characterized_object) — STEP P21
    // encodes 6 attributes. Regression guard for the check_count(4)->6 fix.
    use step_io::ir::plm::{Document, DocumentFile, DocumentType, PlmPool};
    let mut model = empty_model();
    let mut plm = PlmPool::default();
    let kind = plm.document_types.push(DocumentType {
        product_data_type: "step file".into(),
    });
    plm.documents.push(Document::DocumentFile(DocumentFile {
        id: "shell_prt.stp".into(),
        name: "SHELL".into(),
        description: String::new(),
        kind,
        characterized_object_name: "carrier".into(),
        characterized_object_description: Some("desc".into()),
    }));
    model.plm = Some(plm);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_plm = re.plm.as_ref().expect("plm pool");
    assert_eq!(re_plm.documents.len(), 1);
    let Document::DocumentFile(df) = re_plm.documents.iter().next().unwrap() else {
        panic!("expected DocumentFile variant");
    };
    assert_eq!(df.id, "shell_prt.stp");
    assert_eq!(df.characterized_object_name, "carrier");
    assert_eq!(
        df.characterized_object_description,
        Some("desc".to_string())
    );
}

#[test]
fn numeric_representation_item_round_trip() {
    // INTEGER_REPRESENTATION_ITEM / REAL_REPRESENTATION_ITEM — representation_item
    // value-items, orphan round-trip in one interleaved arena.
    use step_io::ir::shape_rep::{
        IntegerRepresentationItem, NumericRepresentationItem, RealRepresentationItem,
    };
    let mut model = empty_model();
    model
        .numeric_representation_items
        .push(NumericRepresentationItem::Integer(
            IntegerRepresentationItem {
                name: "number of segments".into(),
                the_value: 19,
            },
        ));
    model
        .numeric_representation_items
        .push(NumericRepresentationItem::Real(RealRepresentationItem {
            name: "saved view scale".into(),
            the_value: 2.5,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.numeric_representation_items.len(), 2);
    let NumericRepresentationItem::Integer(i) =
        re.numeric_representation_items.iter().next().unwrap()
    else {
        panic!("expected Integer variant first");
    };
    assert_eq!(i.name, "number of segments");
    assert_eq!(i.the_value, 19);
    let NumericRepresentationItem::Real(r) = re.numeric_representation_items.iter().nth(1).unwrap()
    else {
        panic!("expected Real variant second");
    };
    assert_eq!(r.name, "saved view scale");
    assert!((r.the_value - 2.5).abs() < f64::EPSILON);
}

#[test]
fn tessellation_round_trip() {
    // COORDINATES_LIST + COMPLEX_TRIANGULATED_FACE — orphan tessellation
    // cluster; the face references the coordinates list.
    use step_io::ir::tessellation::{ComplexTriangulatedFace, CoordinatesList, TessellatedItem};
    let mut model = empty_model();
    let coords = model
        .tessellated_items
        .push(TessellatedItem::CoordinatesList(CoordinatesList {
            name: "pts".into(),
            npoints: 3,
            position_coords: vec![
                vec![0.0, 0.0, 0.0],
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
            ],
        }));
    model.tessellated_faces.push(ComplexTriangulatedFace {
        name: "face".into(),
        coordinates: coords,
        pnmax: 3,
        normals: vec![vec![0.0, 0.0, 1.0]],
        geometric_link: None,
        pnindex: vec![1, 2, 3],
        triangle_strips: vec![vec![1, 2, 3]],
        triangle_fans: vec![],
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.tessellated_items.len(), 1);
    assert_eq!(re.tessellated_faces.len(), 1);
    let TessellatedItem::CoordinatesList(c) = re.tessellated_items.iter().next().unwrap();
    assert_eq!(c.npoints, 3);
    assert_eq!(c.position_coords.len(), 3);
    assert!((c.position_coords[1][0] - 1.0).abs() < f64::EPSILON);
    let f = re.tessellated_faces.iter().next().unwrap();
    assert_eq!(f.name, "face");
    assert_eq!(f.pnmax, 3);
    assert_eq!(f.pnindex, vec![1, 2, 3]);
    assert_eq!(f.triangle_strips, vec![vec![1, 2, 3]]);
    assert!(f.triangle_fans.is_empty());
    assert!(f.geometric_link.is_none());
}
