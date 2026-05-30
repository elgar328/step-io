//! Integration tests for the writer: synthesise IR → write → re-parse → verify.
//!
//! W-A does not target real fixture round-trip (that needs topology). Instead
//! we build the smallest IR instances by hand and check that the re-parsed
//! result matches what we put in.

use step_io::ir::arena::Arena;
use step_io::ir::assembly::{
    AssemblyTree, GeometryLeaf, Instance, Product, SolidContent, Transform3d,
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
    CompositeShapeAspectId, ContinuousShapeAspectId, DerivedShapeAspectId, DirectionId,
    GeneralPropertyId, Placement3dId, PointId, ShapeAspectId, SolidId, UnitContextId,
};
use step_io::ir::model::StepModel;
use step_io::ir::pmi::{
    AngleSelection, AngularLocationData, DimensionalLocation, DimensionalLocationData,
    DimensionalSize, DimensionalSizeKind, PmiPool, ToleranceZoneForm, TypeQualifier,
    ValueFormatTypeQualifier,
};
use step_io::ir::property::{
    DerivedDefinitionItem, GeneralProperty, GeneralPropertyAssociation, Property, PropertyPool,
};
use step_io::ir::shape_aspect_ref::ShapeAspectRef;
use step_io::ir::shape_rep::{
    AllAroundShapeAspect, AngleUnit, CentreOfSymmetry, CompositeGroupShapeAspect, LengthUnit,
    ShapeAspect, ShapeAspectRelationship, ShapeAspectRelationshipKind, SolidAngleUnit, UnitContext,
};
use step_io::ir::topology::{Face, FaceKind, Orientation, Shell, Solid, Wire};
use step_io::ir::units::{MassFlavor, MassUnit, NamedUnit, UnitsPool};
use step_io::ir::visualization::{
    CameraModel, CameraModelD3, FoundedItem, Projection, ViewVolume, VisualizationPool,
};
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
        geometry: None,
        instances: vec![Instance {
            child: leaf_pid,
            transform,
            occurrence_id: "1".into(),
            occurrence_name: "LeafInst".into(),
        }],
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
    assert!(
        root_prod.geometry.is_none(),
        "expected Root Group (no geometry), got {:?}",
        root_prod.geometry
    );
    assert_eq!(root_prod.instances.len(), 1);
    assert_eq!(root_prod.instances[0].occurrence_id, "1");
    assert_eq!(root_prod.instances[0].occurrence_name, "LeafInst");
    let leaf_prod = r_asm
        .products
        .iter()
        .find(|p| p.id == "Leaf")
        .expect("Leaf product survived");
    assert!(matches!(leaf_prod.geometry, Some(GeometryLeaf::Solid(_))));
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
        geometry: None,
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
    assert!(
        root_prod.geometry.is_none(),
        "expected Root Group (no geometry), got {:?}",
        root_prod.geometry
    );
    assert_eq!(root_prod.instances.len(), 2);
    assert_eq!(
        root_prod.instances[0].child, root_prod.instances[1].child,
        "both point at the same Leaf"
    );
    assert_eq!(root_prod.instances[0].occurrence_id, "1");
    assert_eq!(root_prod.instances[1].occurrence_id, "2");
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
        geometry: Some(GeometryLeaf::Solid(SolidContent { ids: vec![s1, s2] })),
        instances: vec![],
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
    match &prod.geometry {
        Some(GeometryLeaf::Solid(solid)) => {
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
        geometry: None,
        instances: vec![],
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
        geometry: None,
        instances: vec![],
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
    use step_io::ir::PropertyDefinitionId;
    use step_io::ir::property::{
        CharacterizedDefinition, ProductDefinitionShape, PropertyDefinition, PropertyDefinitionData,
    };
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    // The product chain auto-mirrors PDS into property_definitions during
    // assembly emit setup at reader time, but for a hand-built IR we have
    // to push both the PDS variant (matching the assembly chain's PDS for
    // this product) and the Itself variant (the property's own PD) in
    // source-#N order: PDS first (assembly chain emits PDS during product
    // emit, before user-defined property emit), Itself second.
    pool.property_definitions
        .push(PropertyDefinition::ProductDefinitionShape(
            ProductDefinitionShape {
                inherited: PropertyDefinitionData {
                    name: String::new(),
                    description: String::new(),
                    definition: CharacterizedDefinition::ProductDefinition(part_pid),
                },
            },
        ));
    let pd_id =
        pool.property_definitions
            .push(PropertyDefinition::Itself(PropertyDefinitionData {
                name: "p1".into(),
                description: "user defined attribute".into(),
                definition: CharacterizedDefinition::ProductDefinition(part_pid),
            }));
    pool.properties.push(Property {
        name: "p1".into(),
        description: Some("user defined attribute".into()),
        definition: pd_id,
        representation_name: String::new(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(
            UnitContextId(0),
        )),
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
            derived_definition: DerivedDefinitionItem::PropertyDefinition(pd_id),
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
    // The re-read fills property_definitions with PDS first (assembly
    // chain) then Itself (PD handler), so the Itself index is 1.
    assert_eq!(
        gpa.derived_definition,
        DerivedDefinitionItem::PropertyDefinition(PropertyDefinitionId(1))
    );
}

#[test]
fn property_definition_with_general_property_target_round_trips() {
    use step_io::ir::property::{
        CharacterizedDefinition, GeneralProperty, PropertyDefinition, PropertyDefinitionData,
        PropertyPool,
    };
    // A PROPERTY_DEFINITION whose `definition` is a GENERAL_PROPERTY (the
    // general_property member of characterized_definition) — no product
    // binding. Must survive both write (PD → #gp ref) and read (resolve
    // #gp back to a GeneralProperty variant).
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);

    let mut pool = PropertyPool::default();
    let gp_id = pool.general_properties.push(GeneralProperty {
        id: "GP1".into(),
        name: "material".into(),
        description: Some("user defined attribute".into()),
    });
    pool.property_definitions
        .push(PropertyDefinition::Itself(PropertyDefinitionData {
            name: "p_mat".into(),
            description: "user defined attribute".into(),
            definition: CharacterizedDefinition::GeneralProperty(gp_id),
        }));
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    assert_eq!(re_pool.general_properties.len(), 1);
    let has_gp_pd = re_pool.property_definitions.iter().any(|pd| {
        matches!(
            pd,
            PropertyDefinition::Itself(d)
                if matches!(d.definition, CharacterizedDefinition::GeneralProperty(_))
        )
    });
    assert!(
        has_gp_pd,
        "PROPERTY_DEFINITION with a GENERAL_PROPERTY definition should round-trip"
    );
}

#[test]
fn property_definition_with_document_file_target_round_trips() {
    use step_io::ir::plm::{Document, DocumentFile, DocumentType, PlmPool};
    use step_io::ir::property::{
        CharacterizedDefinition, PropertyDefinition, PropertyDefinitionData, PropertyPool,
    };
    // A PROPERTY_DEFINITION whose `definition` is a DOCUMENT_FILE (a
    // characterized_object subtype, hence a valid characterized_definition
    // member) — no product binding. Must survive write (PD -> #doc ref) and
    // read (resolve #doc back to a Document variant).
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);

    let mut plm = PlmPool::default();
    let dtype_id = plm.document_types.push(DocumentType {
        product_data_type: "configuration controlled document".into(),
    });
    let doc_id = plm.documents.push(Document::DocumentFile(DocumentFile {
        id: "DF1".into(),
        name: "spec.pdf".into(),
        description: String::new(),
        kind: dtype_id,
        characterized_object_name: String::new(),
        characterized_object_description: None,
    }));
    model.plm = Some(plm);

    let mut pool = PropertyPool::default();
    pool.property_definitions
        .push(PropertyDefinition::Itself(PropertyDefinitionData {
            name: "doc_prop".into(),
            description: String::new(),
            definition: CharacterizedDefinition::Document(doc_id),
        }));
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    let has_doc_pd = re_pool.property_definitions.iter().any(|pd| {
        matches!(
            pd,
            PropertyDefinition::Itself(d)
                if matches!(d.definition, CharacterizedDefinition::Document(_))
        )
    });
    assert!(
        has_doc_pd,
        "PROPERTY_DEFINITION with a DOCUMENT_FILE definition should round-trip"
    );
}

#[test]
fn pd_based_shape_definition_representation_round_trips() {
    use step_io::ir::property::{
        CharacterizedDefinition, GeneralProperty, PropertyDefinition, PropertyDefinitionData,
        PropertyPool, ShapeDefinitionRepresentationLink,
    };
    use step_io::ir::shape_rep::{PlainRepr, Representation, RepresentationContextRef};
    // A SHAPE_DEFINITION_REPRESENTATION whose `definition` is a
    // PROPERTY_DEFINITION (the geometric-validation / CATIA-geometric-set PMI
    // pattern), not a product PDS — captured in the new arena and emitted.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "validation shape".into(),
        context: Some(RepresentationContextRef::Unitful(
            step_io::ir::UnitContextId(0),
        )),
        frame: None,
    }));

    let mut pool = PropertyPool::default();
    let gp_id = pool.general_properties.push(GeneralProperty {
        id: "GP1".into(),
        name: "gvp".into(),
        description: None,
    });
    let pd_id =
        pool.property_definitions
            .push(PropertyDefinition::Itself(PropertyDefinitionData {
                name: "shape for solid data".into(),
                description: String::new(),
                definition: CharacterizedDefinition::GeneralProperty(gp_id),
            }));
    pool.shape_definition_representations
        .push(ShapeDefinitionRepresentationLink {
            definition: pd_id,
            used_representation: rep,
        });
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    assert_eq!(
        re_pool.shape_definition_representations.len(),
        1,
        "the PD-based SDR link survives round-trip"
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
            geometry: Some(GeometryLeaf::Solid(SolidContent {
                ids: vec![solid_id],
            })),
            instances: vec![],
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
        dim_exp: None,
    }));
    pool.named_units.push(NamedUnit::Mass(MassFlavor {
        unit: MassUnit::Gram,
        cbu_base: Some(kg),
        dim_exp: None,
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

/// Build a model with a part product and one of each shape-aspect-family
/// arena entry (plain + 3 subtypes), returning their ids. Shared by the
/// `SHAPE_ASPECT_RELATIONSHIP` round-trip tests.
fn shape_aspect_relationship_fixture() -> (
    StepModel,
    ShapeAspectId,
    CompositeShapeAspectId,
    DerivedShapeAspectId,
    ContinuousShapeAspectId,
) {
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let cg = model
        .composite_group_shape_aspects
        .push(CompositeGroupShapeAspect {
            name: "cg".into(),
            description: String::new(),
            target: part_pid,
            product_definitional: false,
        });
    let cs = model.centre_of_symmetries.push(CentreOfSymmetry {
        name: "cs".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let aa = model.all_around_shape_aspects.push(AllAroundShapeAspect {
        name: "aa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    (model, sa, cg, cs, aa)
}

#[test]
fn shape_aspect_relationship_round_trip() {
    // SHAPE_ASPECT_RELATIONSHIP — exercises all four ShapeAspectRef
    // variants (plain shape_aspect + 3 subtypes) as relation endpoints.
    let (mut model, sa, cg, cs, aa) = shape_aspect_relationship_fixture();
    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: "r1".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            related_shape_aspect: ShapeAspectRef::CompositeGroupShapeAspect(cg),
            kind: ShapeAspectRelationshipKind::Plain,
        });
    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: "r2".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::CentreOfSymmetry(cs),
            related_shape_aspect: ShapeAspectRef::AllAroundShapeAspect(aa),
            kind: ShapeAspectRelationshipKind::Plain,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.shape_aspect_relationships.len(), 2);
    let rels: Vec<_> = re.shape_aspect_relationships.iter().collect();
    assert!(matches!(
        rels[0].relating_shape_aspect,
        ShapeAspectRef::ShapeAspect(_)
    ));
    assert!(matches!(
        rels[0].related_shape_aspect,
        ShapeAspectRef::CompositeGroupShapeAspect(_)
    ));
    assert!(matches!(
        rels[1].relating_shape_aspect,
        ShapeAspectRef::CentreOfSymmetry(_)
    ));
    assert!(matches!(
        rels[1].related_shape_aspect,
        ShapeAspectRef::AllAroundShapeAspect(_)
    ));
}

#[test]
fn id_attribute_shape_aspect_round_trip() {
    // ID_ATTRIBUTE.identified_item -> SHAPE_ASPECT. Guards the
    // Pass9PlmAttributes dispatch order: it must run after Pass8ShapeAspect so
    // shape_aspect_id_map is populated when the re-read resolves identified_item.
    // If the pass is moved back before the PMI block, the re-read drops the
    // id_attribute and this assertion fails.
    use step_io::ir::ShapeAspectRef;
    use step_io::ir::property::{IdAttribute, IdAttributeItem, PropertyPool};
    let (mut model, sa, _cg, _cs, _aa) = shape_aspect_relationship_fixture();
    model
        .properties
        .get_or_insert_with(PropertyPool::default)
        .id_attributes
        .push(IdAttribute {
            attribute_value: "id1".into(),
            identified_item: IdAttributeItem::ShapeAspect(ShapeAspectRef::ShapeAspect(sa)),
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let pool = re.properties.as_ref().expect("properties pool survives");
    assert_eq!(
        pool.id_attributes.len(),
        1,
        "ID_ATTRIBUTE->SHAPE_ASPECT survives round-trip"
    );
    assert!(matches!(
        pool.id_attributes.iter().next().unwrap().identified_item,
        IdAttributeItem::ShapeAspect(ShapeAspectRef::ShapeAspect(_))
    ));
}

#[test]
fn id_attribute_composite_shape_aspect_round_trip() {
    // ID_ATTRIBUTE.identified_item -> COMPOSITE_GROUP_SHAPE_ASPECT, exercising
    // the resolve_shape_aspect_ref family path + emit_shape_aspect_ref. The
    // composite_group_shape_aspects arena is compared by the round-trip diff,
    // so this target is FAIL-safe.
    use step_io::ir::ShapeAspectRef;
    use step_io::ir::property::{IdAttribute, IdAttributeItem, PropertyPool};
    let (mut model, _sa, cg, _cs, _aa) = shape_aspect_relationship_fixture();
    model
        .properties
        .get_or_insert_with(PropertyPool::default)
        .id_attributes
        .push(IdAttribute {
            attribute_value: "id-cg".into(),
            identified_item: IdAttributeItem::ShapeAspect(
                ShapeAspectRef::CompositeGroupShapeAspect(cg),
            ),
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let pool = re.properties.as_ref().expect("properties pool survives");
    assert_eq!(pool.id_attributes.len(), 1);
    assert!(matches!(
        pool.id_attributes.iter().next().unwrap().identified_item,
        IdAttributeItem::ShapeAspect(ShapeAspectRef::CompositeGroupShapeAspect(_))
    ));
}

#[test]
fn shape_aspect_relationship_subtypes_round_trip() {
    // SHAPE_ASPECT_ASSOCIATIVITY / SHAPE_ASPECT_DERIVING_RELATIONSHIP —
    // the two subtypes round-trip via the `kind` discriminant.
    let (mut model, sa, ..) = shape_aspect_relationship_fixture();
    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: "assoc".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            related_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            kind: ShapeAspectRelationshipKind::Associativity,
        });
    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: "deriv".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            related_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            kind: ShapeAspectRelationshipKind::DerivingRelationship,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.shape_aspect_relationships.len(), 2);
    let rels: Vec<_> = re.shape_aspect_relationships.iter().collect();
    assert_eq!(rels[0].kind, ShapeAspectRelationshipKind::Associativity);
    assert_eq!(
        rels[1].kind,
        ShapeAspectRelationshipKind::DerivingRelationship
    );
}

#[test]
fn dimensional_size_round_trip() {
    // DIMENSIONAL_SIZE (plain) + ANGULAR_SIZE — `applies_to` through
    // ShapeAspectRef, distinguished by the kind discriminant.
    let (mut model, sa, ..) = shape_aspect_relationship_fixture();
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    pmi.dimensional_sizes.push(DimensionalSize {
        applies_to: ShapeAspectRef::ShapeAspect(sa),
        name: "diameter".into(),
        kind: DimensionalSizeKind::Plain,
    });
    pmi.dimensional_sizes.push(DimensionalSize {
        applies_to: ShapeAspectRef::ShapeAspect(sa),
        name: "angle".into(),
        kind: DimensionalSizeKind::Angular(AngleSelection::Equal),
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let pmi = re.pmi.expect("pmi pool");
    assert_eq!(pmi.dimensional_sizes.len(), 2);
    let dss: Vec<_> = pmi.dimensional_sizes.iter().collect();
    assert_eq!(dss[0].name, "diameter");
    assert_eq!(dss[0].kind, DimensionalSizeKind::Plain);
    assert_eq!(
        dss[1].kind,
        DimensionalSizeKind::Angular(AngleSelection::Equal)
    );
}

#[test]
fn dimensional_location_round_trip() {
    // DIMENSIONAL_LOCATION + DIRECTED_DIMENSIONAL_LOCATION — both endpoints
    // through ShapeAspectRef, distinguished by enum variant.
    let (mut model, sa, cg, cs, aa) = shape_aspect_relationship_fixture();
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    pmi.dimensional_locations
        .push(DimensionalLocation::Plain(DimensionalLocationData {
            name: "linear distance".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            related_shape_aspect: ShapeAspectRef::CompositeGroupShapeAspect(cg),
        }));
    pmi.dimensional_locations
        .push(DimensionalLocation::Directed(DimensionalLocationData {
            name: "directed".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::CentreOfSymmetry(cs),
            related_shape_aspect: ShapeAspectRef::AllAroundShapeAspect(aa),
        }));
    pmi.dimensional_locations
        .push(DimensionalLocation::Angular(AngularLocationData {
            name: "angle".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            related_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            angle_selection: AngleSelection::Small,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let pmi = re.pmi.expect("pmi pool");
    assert_eq!(pmi.dimensional_locations.len(), 3);
    let dls: Vec<_> = pmi.dimensional_locations.iter().collect();
    let DimensionalLocation::Plain(d0) = dls[0] else {
        panic!("expected Plain");
    };
    assert_eq!(d0.name, "linear distance");
    assert!(matches!(dls[1], DimensionalLocation::Directed(_)));
    let DimensionalLocation::Angular(d2) = dls[2] else {
        panic!("expected Angular");
    };
    assert_eq!(d2.angle_selection, AngleSelection::Small);
}

#[test]
fn view_volume_round_trip() {
    // VIEW_VOLUME — a founded_item referencing a cartesian point + planar box.
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    let pt = model.geometry.points.push(Point3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    });
    let pb = model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "win".into(),
            size_in_x: 10.0,
            size_in_y: 20.0,
            placement: PlanarBoxPlacement::Placement3d(frame),
        }));
    model
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::ViewVolume(ViewVolume {
            projection_type: Projection::Parallel,
            projection_point: pt,
            view_plane_distance: 1645.0,
            front_plane_distance: 0.0,
            front_plane_clipping: false,
            back_plane_distance: 0.0,
            back_plane_clipping: true,
            view_volume_sides_clipping: false,
            view_window: pb,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let viz = re.visualization.expect("viz pool");
    let vv = viz
        .founded_items
        .iter()
        .find_map(|f| match f {
            FoundedItem::ViewVolume(v) => Some(v),
            _ => None,
        })
        .expect("view volume round-trips");
    assert_eq!(vv.projection_type, Projection::Parallel);
    assert!((vv.view_plane_distance - 1645.0).abs() < f64::EPSILON);
    assert!(!vv.front_plane_clipping);
    assert!(vv.back_plane_clipping);
}

#[test]
fn camera_model_d3_round_trip() {
    // CAMERA_MODEL_D3 — references a VIEW_VOLUME (founded_item) + a placement.
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    let pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let pb = model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "win".into(),
            size_in_x: 1.0,
            size_in_y: 2.0,
            placement: PlanarBoxPlacement::Placement3d(frame),
        }));
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let vv = viz.founded_items.push(FoundedItem::ViewVolume(ViewVolume {
        projection_type: Projection::Central,
        projection_point: pt,
        view_plane_distance: 100.0,
        front_plane_distance: 0.0,
        front_plane_clipping: false,
        back_plane_distance: 0.0,
        back_plane_clipping: false,
        view_volume_sides_clipping: false,
        view_window: pb,
    }));
    viz.camera_models
        .push(CameraModel::CameraModelD3(CameraModelD3 {
            name: "cam".into(),
            view_reference_system: frame,
            perspective_of_volume: vv,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let viz = re.visualization.expect("viz pool");
    assert_eq!(viz.camera_models.len(), 1);
    let CameraModel::CameraModelD3(cm) = viz.camera_models.iter().next().unwrap() else {
        panic!("expected CameraModelD3");
    };
    assert_eq!(cm.name, "cam");
}

#[test]
fn camera_usage_round_trip() {
    // CAMERA_USAGE — representation_map SUBTYPE that pairs a camera_model
    // origin with a target representation. Exercises the delayed-emit
    // pathway through `emit_camera_usage_arena`.
    use step_io::ir::shape_rep::{CameraUsage, PlainRepr, Representation, RepresentationMap};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    let pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let pb = model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "win".into(),
            size_in_x: 1.0,
            size_in_y: 2.0,
            placement: PlanarBoxPlacement::Placement3d(frame),
        }));
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let vv = viz.founded_items.push(FoundedItem::ViewVolume(ViewVolume {
        projection_type: Projection::Central,
        projection_point: pt,
        view_plane_distance: 100.0,
        front_plane_distance: 0.0,
        front_plane_clipping: false,
        back_plane_distance: 0.0,
        back_plane_clipping: false,
        view_volume_sides_clipping: false,
        view_window: pb,
    }));
    let cam = viz
        .camera_models
        .push(CameraModel::CameraModelD3(CameraModelD3 {
            name: "cam".into(),
            view_reference_system: frame,
            perspective_of_volume: vv,
        }));
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "target".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    model
        .representation_maps
        .push(RepresentationMap::CameraUsage(CameraUsage {
            mapping_origin: cam,
            mapped_representation: rep,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.representation_maps.len(), 1);
    let RepresentationMap::CameraUsage(cu) = re.representation_maps.iter().next().unwrap() else {
        panic!("expected CameraUsage variant");
    };
    assert_eq!(cu.mapping_origin, cam);
    assert_eq!(cu.mapped_representation, rep);
}

#[test]
fn camera_image_round_trip() {
    // CAMERA_IMAGE + CAMERA_IMAGE_3D_WITH_SCALE — mapped_item SUBTYPEs
    // referencing a camera_usage source and a planar_box target.
    use step_io::ir::shape_rep::{
        CameraImage, CameraUsage, MappedItem, PlainRepr, Representation, RepresentationMap,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    let pt = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let pb = model
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: "win".into(),
            size_in_x: 1.0,
            size_in_y: 2.0,
            placement: PlanarBoxPlacement::Placement3d(frame),
        }));
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let vv = viz.founded_items.push(FoundedItem::ViewVolume(ViewVolume {
        projection_type: Projection::Central,
        projection_point: pt,
        view_plane_distance: 100.0,
        front_plane_distance: 0.0,
        front_plane_clipping: false,
        back_plane_distance: 0.0,
        back_plane_clipping: false,
        view_volume_sides_clipping: false,
        view_window: pb,
    }));
    let cam = viz
        .camera_models
        .push(CameraModel::CameraModelD3(CameraModelD3 {
            name: "cam".into(),
            view_reference_system: frame,
            perspective_of_volume: vv,
        }));
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "target".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    let cu = model
        .representation_maps
        .push(RepresentationMap::CameraUsage(CameraUsage {
            mapping_origin: cam,
            mapped_representation: rep,
        }));
    model
        .mapped_items
        .push(MappedItem::CameraImage(CameraImage {
            name: "img".into(),
            mapping_source: cu,
            mapping_target: pb,
        }));
    model
        .mapped_items
        .push(MappedItem::CameraImage3dWithScale(CameraImage {
            name: "img3d".into(),
            mapping_source: cu,
            mapping_target: pb,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.mapped_items.len(), 2);
    let mut iter = re.mapped_items.iter();
    let MappedItem::CameraImage(ci) = iter.next().unwrap() else {
        panic!("expected CameraImage");
    };
    assert_eq!(ci.name, "img");
    let MappedItem::CameraImage3dWithScale(ci3) = iter.next().unwrap() else {
        panic!("expected CameraImage3dWithScale");
    };
    assert_eq!(ci3.name, "img3d");
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
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
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
    let MappedItem::Itself(mi) = re.mapped_items.iter().next().unwrap() else {
        panic!("expected Itself variant");
    };
    assert_eq!(mi.name, "inst");
    assert!(matches!(
        mi.mapping_target,
        RepresentationItemRef::Placement3d(_)
    ));
    let RepresentationMap::Itself(rm) = re.representation_maps.iter().next().unwrap() else {
        panic!("expected Itself variant");
    };
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
        re_pmi.annotation_occurrences.iter().next().unwrap()
    else {
        panic!("expected AnnotationPlane");
    };
    assert_eq!(ap.name, "Linear Size.1");
    assert!(ap.styles.is_empty());
    assert!(matches!(ap.item, RepresentationItemRef::Surface(_)));
}

#[test]
fn tessellated_annotation_occurrence_round_trip() {
    // TESSELLATED_ANNOTATION_OCCURRENCE — `item` points at a tessellated
    // geometric set.
    use step_io::ir::pmi::{AnnotationOccurrence, PmiPool, TessellatedAnnotationOccurrence};
    use step_io::ir::tessellation::{TessellatedGeometricSet, TessellatedItem};
    let mut model = empty_model();
    let gset = model
        .tessellated_items
        .push(TessellatedItem::TessellatedGeometricSet(
            TessellatedGeometricSet {
                name: "gset".into(),
                children: vec![],
            },
        ));
    model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::TessellatedAnnotationOccurrence(
            TessellatedAnnotationOccurrence {
                name: "anno".into(),
                styles: vec![],
                item: gset,
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.annotation_occurrences.len(), 1);
    let AnnotationOccurrence::TessellatedAnnotationOccurrence(tao) =
        re_pmi.annotation_occurrences.iter().next().unwrap()
    else {
        panic!("expected TessellatedAnnotationOccurrence");
    };
    assert_eq!(tao.name, "anno");
    assert!(tao.styles.is_empty());
}

#[test]
fn annotation_occurrence_subtypes_round_trip() {
    // ANNOTATION_SYMBOL_OCCURRENCE / ANNOTATION_TEXT_OCCURRENCE /
    // DRAUGHTING_ANNOTATION_OCCURRENCE — same shape as ANNOTATION_PLANE
    // (name + styles + item), `item` resolved through the generic
    // `representation_item` resolver.
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::pmi::{
        AnnotationOccurrence, AnnotationSymbolOccurrence, AnnotationTextOccurrence,
        DraughtingAnnotationOccurrence, PmiPool,
    };
    let mut model = empty_model();
    // Build a minimal Surface to serve as the `item` of each occurrence.
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
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::AnnotationSymbolOccurrence(
            AnnotationSymbolOccurrence {
                name: "sym".into(),
                styles: vec![],
                item: RepresentationItemRef::Surface(surf),
            },
        ));
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::AnnotationTextOccurrence(
            AnnotationTextOccurrence {
                name: "txt".into(),
                styles: vec![],
                item: RepresentationItemRef::Surface(surf),
            },
        ));
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::DraughtingAnnotationOccurrence(
            DraughtingAnnotationOccurrence {
                name: "drft".into(),
                styles: vec![],
                item: RepresentationItemRef::Surface(surf),
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.annotation_occurrences.len(), 3);
    let mut iter = re_pmi.annotation_occurrences.iter();
    let AnnotationOccurrence::AnnotationSymbolOccurrence(aso) = iter.next().unwrap() else {
        panic!("expected AnnotationSymbolOccurrence");
    };
    assert_eq!(aso.name, "sym");
    assert!(matches!(aso.item, RepresentationItemRef::Surface(_)));
    let AnnotationOccurrence::AnnotationTextOccurrence(ato) = iter.next().unwrap() else {
        panic!("expected AnnotationTextOccurrence");
    };
    assert_eq!(ato.name, "txt");
    assert!(matches!(ato.item, RepresentationItemRef::Surface(_)));
    let AnnotationOccurrence::DraughtingAnnotationOccurrence(dao) = iter.next().unwrap() else {
        panic!("expected DraughtingAnnotationOccurrence");
    };
    assert_eq!(dao.name, "drft");
    assert!(matches!(dao.item, RepresentationItemRef::Surface(_)));
}

#[test]
fn leader_curve_terminator_round_trip() {
    // LEADER_CURVE + TERMINATOR_SYMBOL + LEADER_TERMINATOR — phase
    // annotation-curve-leader. TerminatorSymbol / LeaderTerminator carry
    // an `annotated_curve` back-reference into the LeaderCurve arena.
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::pmi::{
        AnnotationOccurrence, LeaderCurve, LeaderTerminator, PmiPool, TerminatorSymbol,
    };
    let mut model = empty_model();
    // A minimal Line curve serves as LeaderCurve.item.
    let p0 = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let dir = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let line = model
        .geometry
        .curves
        .push(step_io::ir::geometry::Curve::Line(
            step_io::ir::geometry::Line3 {
                point: p0,
                direction: dir,
                magnitude: 1.0,
            },
        ));
    // A Surface serves as the TerminatorSymbol / LeaderTerminator item.
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let position = push_placement(&mut model, p0, Some(axis), Some(dir));
    let surf = model
        .geometry
        .surfaces
        .push(step_io::ir::geometry::Surface::Plane(
            step_io::ir::geometry::Plane3 { position },
        ));
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let lc_id = pmi.annotation_curve_occurrences.push(LeaderCurve {
        name: "lc".into(),
        styles: vec![],
        item: line,
    });
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::TerminatorSymbol(TerminatorSymbol {
            name: "ts".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
            annotated_curve: lc_id,
        }));
    pmi.annotation_occurrences
        .push(AnnotationOccurrence::LeaderTerminator(LeaderTerminator {
            name: "lt".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
            annotated_curve: lc_id,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.annotation_curve_occurrences.len(), 1);
    assert_eq!(re_pmi.annotation_occurrences.len(), 2);
    let mut iter = re_pmi.annotation_occurrences.iter();
    let AnnotationOccurrence::TerminatorSymbol(ts) = iter.next().unwrap() else {
        panic!("expected TerminatorSymbol");
    };
    assert_eq!(ts.name, "ts");
    let AnnotationOccurrence::LeaderTerminator(lt) = iter.next().unwrap() else {
        panic!("expected LeaderTerminator");
    };
    assert_eq!(lt.name, "lt");
    // Both reference the same LeaderCurve id.
    assert_eq!(ts.annotated_curve, lt.annotated_curve);
}

#[test]
#[allow(clippy::too_many_lines)]
fn draughting_callout_round_trip() {
    // Plain DraughtingCallout + LeaderDirected variant + Relationship —
    // phase draughting-callout. contents references both kinds of
    // element (AnnotationOccurrence and AnnotationCurveOccurrence).
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::pmi::{
        AnnotationOccurrence, DraughtingCallout, DraughtingCalloutData, DraughtingCalloutElement,
        DraughtingCalloutRelationship, LeaderCurve, LeaderTerminator, PmiPool, TerminatorSymbol,
    };
    let mut model = empty_model();
    let p0 = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let dir = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let line = model
        .geometry
        .curves
        .push(step_io::ir::geometry::Curve::Line(
            step_io::ir::geometry::Line3 {
                point: p0,
                direction: dir,
                magnitude: 1.0,
            },
        ));
    let axis = model.geometry.directions.push(Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    });
    let position = push_placement(&mut model, p0, Some(axis), Some(dir));
    let surf = model
        .geometry
        .surfaces
        .push(step_io::ir::geometry::Surface::Plane(
            step_io::ir::geometry::Plane3 { position },
        ));
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let lc_id = pmi.annotation_curve_occurrences.push(LeaderCurve {
        name: "lc".into(),
        styles: vec![],
        item: line,
    });
    let ts_id = pmi
        .annotation_occurrences
        .push(AnnotationOccurrence::TerminatorSymbol(TerminatorSymbol {
            name: "ts".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
            annotated_curve: lc_id,
        }));
    let _ = pmi
        .annotation_occurrences
        .push(AnnotationOccurrence::LeaderTerminator(LeaderTerminator {
            name: "lt".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
            annotated_curve: lc_id,
        }));
    let plain_id = pmi
        .draughting_callouts
        .push(DraughtingCallout::Plain(DraughtingCalloutData {
            name: "plain".into(),
            contents: vec![DraughtingCalloutElement::AnnotationOccurrence(ts_id)],
        }));
    let leader_id = pmi
        .draughting_callouts
        .push(DraughtingCallout::LeaderDirected(DraughtingCalloutData {
            name: "leader".into(),
            contents: vec![DraughtingCalloutElement::AnnotationCurveOccurrence(lc_id)],
        }));
    pmi.draughting_callout_relationships
        .push(DraughtingCalloutRelationship {
            name: "rel".into(),
            description: String::new(),
            relating: plain_id,
            related: leader_id,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.draughting_callouts.len(), 2);
    assert_eq!(re_pmi.draughting_callout_relationships.len(), 1);
    let mut iter = re_pmi.draughting_callouts.iter();
    let DraughtingCallout::Plain(plain) = iter.next().unwrap() else {
        panic!("expected Plain");
    };
    assert_eq!(plain.name, "plain");
    assert_eq!(plain.contents.len(), 1);
    assert!(matches!(
        plain.contents[0],
        DraughtingCalloutElement::AnnotationOccurrence(_)
    ));
    let DraughtingCallout::LeaderDirected(leader) = iter.next().unwrap() else {
        panic!("expected LeaderDirected");
    };
    assert_eq!(leader.name, "leader");
    assert!(matches!(
        leader.contents[0],
        DraughtingCalloutElement::AnnotationCurveOccurrence(_)
    ));
    let rel = re_pmi
        .draughting_callout_relationships
        .iter()
        .next()
        .unwrap();
    assert_eq!(rel.name, "rel");
}

#[test]
fn gt_relationship_round_trip() {
    // GEOMETRIC_TOLERANCE_RELATIONSHIP — pairs two geometric_tolerance arena
    // entries via GeometricToleranceRef (Plain / WithDatumReference branches).
    use step_io::ir::pmi::{
        GeometricTolerance, GeometricToleranceData, GeometricToleranceRef,
        GeometricToleranceRelationship, ToleranceMagnitude,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();
    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let measure = || {
        ToleranceMagnitude::Measure(PropertyMeasure {
            name: String::new(),
            kind: MeasureKind::Length,
            value: 0.1,
            unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
        })
    };
    let data = || GeometricToleranceData {
        name: "t".into(),
        description: String::new(),
        magnitude: measure(),
        toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
        modifiers: Vec::new(),
        unit_size: None,
        defined_area_unit: None,
    };
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let gt1 = pmi
        .geometric_tolerances
        .push(GeometricTolerance::Flatness(data()));
    let gt2 = pmi
        .geometric_tolerances
        .push(GeometricTolerance::Straightness(data()));
    pmi.geometric_tolerance_relationships
        .push(GeometricToleranceRelationship {
            name: "rel".into(),
            description: String::new(),
            relating: GeometricToleranceRef::Plain(gt1),
            related: GeometricToleranceRef::Plain(gt2),
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.geometric_tolerance_relationships.len(), 1);
    let rel = re_pmi
        .geometric_tolerance_relationships
        .iter()
        .next()
        .unwrap();
    assert_eq!(rel.name, "rel");
    assert!(matches!(rel.relating, GeometricToleranceRef::Plain(_)));
    assert!(matches!(rel.related, GeometricToleranceRef::Plain(_)));
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
fn datum_feature_round_trip() {
    // DATUM_FEATURE — a shape_aspect subtype reading into the pmi pool.
    // DATUM / DATUM_FEATURE resolve as ShapeAspectRef endpoints of a
    // SHAPE_ASPECT_RELATIONSHIP, exercising the new resolver + emitter arms.
    use step_io::ir::pmi::{Datum, DatumFeature};
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let datum = pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    let df = pmi.datum_features.push(DatumFeature {
        name: "Datum Feature A".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: true,
        kind: step_io::ir::DatumFeatureKind::Plain,
    });
    model.pmi = Some(pmi);

    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: "r".into(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::DatumFeature(df),
            related_shape_aspect: ShapeAspectRef::Datum(datum),
            kind: ShapeAspectRelationshipKind::Plain,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);

    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.datum_features.len(), 1);
    let re_df = re_pmi.datum_features.iter().next().unwrap();
    assert_eq!(re_df.name, "Datum Feature A");
    assert!(re_df.product_definitional);
    assert_eq!(re_df.target, step_io::ProductId(0));

    assert_eq!(re.shape_aspect_relationships.len(), 1);
    let rel = re.shape_aspect_relationships.iter().next().unwrap();
    assert!(matches!(
        rel.relating_shape_aspect,
        ShapeAspectRef::DatumFeature(_)
    ));
    assert!(matches!(rel.related_shape_aspect, ShapeAspectRef::Datum(_)));
}

#[test]
fn dimensional_size_with_datum_feature_round_trip() {
    // DIMENSIONAL_SIZE_WITH_DATUM_FEATURE — datum_feature arena's in_enum
    // subtype (phase dsf-datum-feature). Shares the DatumFeatureId namespace
    // with plain DATUM_FEATURE via the DatumFeatureKind discriminant.
    use step_io::ir::DatumFeatureKind;
    use step_io::ir::pmi::DatumFeature;
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    pmi.datum_features.push(DatumFeature {
        name: "df-plain".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        kind: DatumFeatureKind::Plain,
    });
    pmi.datum_features.push(DatumFeature {
        name: "df-dswdf".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        kind: DatumFeatureKind::DimensionalSizeWithDatumFeature,
    });
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("DIMENSIONAL_SIZE_WITH_DATUM_FEATURE("),
        "expected DIMENSIONAL_SIZE_WITH_DATUM_FEATURE in STEP output"
    );
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.datum_features.len(), 2);
    let mut iter = re_pmi.datum_features.iter();
    assert_eq!(iter.next().unwrap().kind, DatumFeatureKind::Plain);
    assert_eq!(
        iter.next().unwrap().kind,
        DatumFeatureKind::DimensionalSizeWithDatumFeature
    );
}

#[test]
fn geometric_tolerance_form_tolerances_round_trip() {
    // FLATNESS / STRAIGHTNESS / ROUNDNESS / CYLINDRICITY_TOLERANCE — the four
    // datum-free form tolerances, covering both ToleranceMagnitude variants:
    // a units-pool MEASURE_WITH_UNIT ref and an inline MEASURE_REPRESENTATION_ITEM.
    use step_io::ir::pmi::{GeometricTolerance, GeometricToleranceData, ToleranceMagnitude};
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::units::MeasureWithUnit;

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });

    // A units-pool MEASURE_WITH_UNIT for the `MeasureWithUnit` magnitude.
    let mwu = model
        .units_pool
        .as_mut()
        .expect("units pool seeded by mm_radian_steradian")
        .measure_with_units
        .push(MeasureWithUnit::Length {
            value: 0.05,
            unit: length_unit,
        });

    let data = |magnitude| GeometricToleranceData {
        name: "t".into(),
        description: String::new(),
        magnitude,
        toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
        modifiers: Vec::new(),
        unit_size: None,
        defined_area_unit: None,
    };
    let measure = || {
        ToleranceMagnitude::Measure(PropertyMeasure {
            name: String::new(),
            kind: MeasureKind::Length,
            value: 0.1,
            unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
        })
    };

    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    pmi.geometric_tolerances
        .push(GeometricTolerance::Flatness(data(
            ToleranceMagnitude::MeasureWithUnit(mwu),
        )));
    pmi.geometric_tolerances
        .push(GeometricTolerance::Straightness(data(measure())));
    pmi.geometric_tolerances
        .push(GeometricTolerance::Roundness(data(
            ToleranceMagnitude::MeasureWithUnit(mwu),
        )));
    pmi.geometric_tolerances
        .push(GeometricTolerance::Cylindricity(data(measure())));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    let gts: Vec<_> = re_pmi.geometric_tolerances.iter().collect();
    assert_eq!(gts.len(), 4);
    assert!(matches!(gts[0], GeometricTolerance::Flatness(_)));
    assert!(matches!(gts[1], GeometricTolerance::Straightness(_)));
    assert!(matches!(gts[2], GeometricTolerance::Roundness(_)));
    assert!(matches!(gts[3], GeometricTolerance::Cylindricity(_)));

    let GeometricTolerance::Flatness(d0) = gts[0] else {
        unreachable!()
    };
    assert!(matches!(
        d0.magnitude,
        ToleranceMagnitude::MeasureWithUnit(_)
    ));
    assert!(matches!(
        d0.toleranced_shape_aspect,
        ShapeAspectRef::ShapeAspect(_)
    ));
    let GeometricTolerance::Straightness(d1) = gts[1] else {
        unreachable!()
    };
    assert!(matches!(d1.magnitude, ToleranceMagnitude::Measure(_)));
}

#[test]
fn general_datum_reference_round_trip() {
    // DATUM_REFERENCE_COMPARTMENT / DATUM_REFERENCE_ELEMENT — the two
    // general_datum_reference variants, each with a `base` pointing at a DATUM.
    use step_io::ir::pmi::{
        Datum, GeneralDatumBase, GeneralDatumReference, GeneralDatumReferenceData,
    };
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let datum = pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    let data = || GeneralDatumReferenceData {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        base: GeneralDatumBase::Datum(datum),
    };
    pmi.general_datum_references
        .push(GeneralDatumReference::Compartment(data()));
    pmi.general_datum_references
        .push(GeneralDatumReference::Element(data()));
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    let gdrs: Vec<_> = re_pmi.general_datum_references.iter().collect();
    assert_eq!(gdrs.len(), 2);
    assert!(matches!(gdrs[0], GeneralDatumReference::Compartment(_)));
    assert!(matches!(gdrs[1], GeneralDatumReference::Element(_)));
    let GeneralDatumReference::Compartment(d0) = gdrs[0] else {
        unreachable!()
    };
    let GeneralDatumBase::Datum(_) = d0.base;
    assert_eq!(d0.target, step_io::ProductId(0));
}

#[test]
fn tolerance_zone_round_trip() {
    // TOLERANCE_ZONE — shape_aspect subtype binding a geometric_tolerance
    // SET (`defining_tolerance`) to a TOLERANCE_ZONE_FORM (`form`).
    use step_io::ir::pmi::{
        GeometricTolerance, GeometricToleranceData, GeometricToleranceRef, ToleranceMagnitude,
        ToleranceZoneForm,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::ToleranceZone;

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });

    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let gt = pmi
        .geometric_tolerances
        .push(GeometricTolerance::Flatness(GeometricToleranceData {
            name: "t".into(),
            description: String::new(),
            magnitude: ToleranceMagnitude::Measure(PropertyMeasure {
                name: String::new(),
                kind: MeasureKind::Length,
                value: 0.1,
                unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
            }),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            modifiers: Vec::new(),
            unit_size: None,
            defined_area_unit: None,
        }));
    let form = pmi.tolerance_zone_forms.push(ToleranceZoneForm {
        name: "cylindrical".into(),
    });

    model.tolerance_zones.push(ToleranceZone {
        name: "tz".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        defining_tolerance: vec![GeometricToleranceRef::Plain(gt)],
        form,
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.tolerance_zones.len(), 1, "TOLERANCE_ZONE round-trips");
    let tz = re.tolerance_zones.iter().next().unwrap();
    assert_eq!(tz.name, "tz");
    assert_eq!(
        tz.defining_tolerance.len(),
        1,
        "defining_tolerance preserved"
    );
    assert!(matches!(
        tz.defining_tolerance[0],
        GeometricToleranceRef::Plain(_)
    ));
}

#[test]
fn shape_dimension_repr_and_dim_char_repr_round_trip() {
    // SHAPE_DIMENSION_REPRESENTATION (Representation enum variant) +
    // DIMENSIONAL_CHARACTERISTIC_REPRESENTATION (PropertyPool single_struct).
    use step_io::ir::pmi::{DimensionalCharacteristic, DimensionalSize, DimensionalSizeKind};
    use step_io::ir::property::{DimensionalCharacteristicRepresentation, PropertyPool};
    use step_io::ir::shape_rep::{Representation, ShapeDimensionRepresentation};

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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let sdr_id = model
        .representations
        .push(Representation::ShapeDimensionRepresentation(
            ShapeDimensionRepresentation {
                name: "sdr".into(),
                context: Some(step_io::ir::RepresentationContextRef::Unitful(
                    UnitContextId(0),
                )),
                items: vec![],
            },
        ));
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let size_id = pmi.dimensional_sizes.push(DimensionalSize {
        applies_to: ShapeAspectRef::ShapeAspect(sa),
        name: String::new(),
        kind: DimensionalSizeKind::Plain,
    });
    let property = model.properties.get_or_insert_with(PropertyPool::default);
    property.dimensional_characteristic_representations.push(
        DimensionalCharacteristicRepresentation {
            dimension: DimensionalCharacteristic::Size(size_id),
            representation: sdr_id,
        },
    );

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_property = re.properties.expect("property pool");
    assert_eq!(
        re_property.dimensional_characteristic_representations.len(),
        1
    );
    let dcr = re_property
        .dimensional_characteristic_representations
        .iter()
        .next()
        .unwrap();
    assert!(matches!(dcr.dimension, DimensionalCharacteristic::Size(_)));
    let sdr_count = re
        .representations
        .iter()
        .filter(|r| matches!(r, Representation::ShapeDimensionRepresentation(_)))
        .count();
    assert_eq!(sdr_count, 1);
}

#[test]
fn pre_defined_marker_round_trip() {
    use step_io::ir::visualization::{PreDefinedMarker, PreDefinedMarkerData, VisualizationPool};
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.pre_defined_markers
        .push(PreDefinedMarker::Plain(PreDefinedMarkerData {
            name: "x".into(),
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.pre_defined_markers.len(), 1);
    let PreDefinedMarker::Plain(d) = re_viz.pre_defined_markers.iter().next().unwrap() else {
        panic!("expected Plain");
    };
    assert_eq!(d.name, "x");
}

#[test]
fn text_style_for_defined_font_round_trip() {
    use step_io::ir::visualization::{
        Colour, ColourRgb, TextStyleForDefinedFont, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.0,
        green: 0.0,
        blue: 1.0,
    }));
    viz.text_styles_for_defined_font
        .push(TextStyleForDefinedFont {
            text_colour: colour_id,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.text_styles_for_defined_font.len(), 1);
}

#[test]
fn invisibility_round_trip() {
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::visualization::{
        Invisibility, InvisibleItem, PlainStyledItem, StyledItem, VisualizationPool,
    };
    let mut model = empty_model();
    let placement = xyz_placement(&mut model);
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let si = viz.styled_items.push(StyledItem::Plain(PlainStyledItem {
        name: String::new(),
        styles: vec![],
        item: RepresentationItemRef::Placement3d(placement),
    }));
    viz.invisibilities.push(Invisibility {
        invisible_items: vec![InvisibleItem::StyledItem(si)],
        presentation_context: None,
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.invisibilities.len(), 1);
    let inv = re_viz.invisibilities.iter().next().unwrap();
    assert_eq!(inv.invisible_items.len(), 1);
    assert!(matches!(
        inv.invisible_items[0],
        InvisibleItem::StyledItem(_)
    ));
}

#[test]
fn unitless_context_round_trip() {
    use step_io::ir::shape_rep::{
        DraughtingModel, DraughtingModelForm, Representation, RepresentationContextRef,
        UnitlessContext,
    };
    let mut model = empty_model();
    let uc_id = model.unitless_contexts.push(UnitlessContext {
        identifier: "2D coordinate system context".into(),
        context_type: "2".into(),
        coordinate_space_dimension: Some(2),
    });
    model
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name: "Default".into(),
            items: vec![],
            context: Some(RepresentationContextRef::Unitless(uc_id)),
            form: DraughtingModelForm::Simple,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.unitless_contexts.len(), 1);
    let dm_count = re
        .representations
        .iter()
        .filter(|r| matches!(r, Representation::DraughtingModel(_)))
        .count();
    // DraughtingModel with empty items drops the carrier on read (existing
    // policy). Only assert the unitless context survives the round-trip.
    let _ = dm_count;
    let uc = re.unitless_contexts.iter().next().unwrap();
    assert_eq!(uc.identifier, "2D coordinate system context");
    assert_eq!(uc.coordinate_space_dimension, Some(2));
}

#[test]
fn plain_representation_context_round_trips() {
    use step_io::ir::shape_rep::{
        DraughtingModel, DraughtingModelForm, Representation, RepresentationContextRef,
        UnitlessContext,
    };
    // A plain simple REPRESENTATION_CONTEXT (no coordinate_space_dimension) —
    // written as a simple entity, not the GRC+PRC complex.
    let mut model = empty_model();
    let uc_id = model.unitless_contexts.push(UnitlessContext {
        identifier: String::new(),
        context_type: "document parameters".into(),
        coordinate_space_dimension: None,
    });
    model
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name: "Default".into(),
            items: vec![],
            context: Some(RepresentationContextRef::Unitless(uc_id)),
            form: DraughtingModelForm::Simple,
        }));

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("REPRESENTATION_CONTEXT(''"),
        "plain context emits as a simple REPRESENTATION_CONTEXT: {text}"
    );
    let re = reconvert(&text);
    assert_eq!(re.unitless_contexts.len(), 1);
    let uc = re.unitless_contexts.iter().next().unwrap();
    assert_eq!(uc.context_type, "document parameters");
    // `None` proves it round-tripped through the simple form (the complex
    // form would carry `Some(dim)`).
    assert_eq!(uc.coordinate_space_dimension, None);
}

#[test]
fn property_with_plain_context_round_trips() {
    use step_io::ir::property::{
        CharacterizedDefinition, GeneralProperty, Property, PropertyDefinition,
        PropertyDefinitionData, PropertyItem, PropertyPool,
    };
    use step_io::ir::shape_rep::{DescriptiveItem, RepresentationContextRef, UnitlessContext};
    // A property whose REPRESENTATION uses a plain (unit-less)
    // REPRESENTATION_CONTEXT and carries only a descriptive item — the shape
    // of the document-property that previously FAILed round-trip. Now the
    // context is modelled, so it survives both ways.
    let mut model = empty_model();
    let uc_id = model.unitless_contexts.push(UnitlessContext {
        identifier: String::new(),
        context_type: "document parameters".into(),
        coordinate_space_dimension: None,
    });

    let mut pool = PropertyPool::default();
    let gp_id = pool.general_properties.push(GeneralProperty {
        id: "GP1".into(),
        name: "doc".into(),
        description: None,
    });
    let pd_id =
        pool.property_definitions
            .push(PropertyDefinition::Itself(PropertyDefinitionData {
                name: "document property".into(),
                description: String::new(),
                definition: CharacterizedDefinition::GeneralProperty(gp_id),
            }));
    pool.properties.push(Property {
        name: "document property".into(),
        description: None,
        definition: pd_id,
        representation_name: "document format".into(),
        context: Some(RepresentationContextRef::Unitless(uc_id)),
        items: vec![PropertyItem::Descriptive(DescriptiveItem {
            name: "data format".into(),
            description: "STEP AP214".into(),
        })],
    });
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    let prop = re_pool
        .properties
        .iter()
        .find(|p| p.representation_name == "document format")
        .expect("the document property survives round-trip");
    assert!(
        matches!(prop.context, Some(RepresentationContextRef::Unitless(_))),
        "the plain context is preserved as Unitless, not dropped to None"
    );
}

#[test]
fn draughting_model_round_trip() {
    use step_io::ir::PmiPool;
    use step_io::ir::geometry::{Plane3, Surface};
    use step_io::ir::pmi::{AnnotationOccurrence, AnnotationPlane};
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{DraughtingModel, DraughtingModelForm, Representation};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let ctx_id = model.units.push(ctx);
    let placement = xyz_placement(&mut model);
    let surf = model.geometry.surfaces.push(Surface::Plane(Plane3 {
        position: placement,
    }));
    model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
            name: "ap".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
        }));
    model
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name: "dm".into(),
            items: vec![RepresentationItemRef::Surface(surf)],
            context: Some(step_io::ir::RepresentationContextRef::Unitful(ctx_id)),
            form: DraughtingModelForm::Simple,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let dm_count = re
        .representations
        .iter()
        .filter(|r| matches!(r, Representation::DraughtingModel(_)))
        .count();
    assert_eq!(dm_count, 1);
}

#[test]
fn draughting_model_shape_tessellated_complex_round_trips() {
    // The geometric-validation draughting model is emitted as a four-part
    // complex MI `(DRAUGHTING_MODEL REPRESENTATION SHAPE_REPRESENTATION
    // TESSELLATED_SHAPE_REPRESENTATION)`. A PMI CIWR references it as `rep`,
    // so it must read back into a DraughtingModel and survive the round-trip.
    use step_io::ir::geometry::{Plane3, Surface};
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{
        DraughtingModel, DraughtingModelForm, Representation, RepresentationContextRef,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let ctx_id = model.units.push(ctx);
    let placement = xyz_placement(&mut model);
    let surf = model.geometry.surfaces.push(Surface::Plane(Plane3 {
        position: placement,
    }));
    model
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name: "gvp".into(),
            items: vec![RepresentationItemRef::Surface(surf)],
            context: Some(RepresentationContextRef::Unitful(ctx_id)),
            form: DraughtingModelForm::ShapeTessellated,
        }));

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("DRAUGHTING_MODEL()")
            && text.contains("SHAPE_REPRESENTATION()")
            && text.contains("TESSELLATED_SHAPE_REPRESENTATION()"),
        "emits the four-part complex MI form: {text}"
    );
    let re = reconvert(&text);
    let dm = re
        .representations
        .iter()
        .find_map(|r| match r {
            Representation::DraughtingModel(dm) => Some(dm),
            _ => None,
        })
        .expect("draughting model survives round-trip");
    assert_eq!(dm.form, DraughtingModelForm::ShapeTessellated);
    assert_eq!(dm.items.len(), 1);
}

#[test]
fn dmia_round_trip() {
    use step_io::ir::PmiPool;
    use step_io::ir::geometry::{Plane3, Surface};
    use step_io::ir::pmi::{
        AnnotationOccurrence, AnnotationPlane, DraughtingModelIdentifiedItem,
        DraughtingModelItemAssociation, DraughtingModelItemDefinition,
    };
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{PlainRepr, Representation};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let ctx_id = model.units.push(ctx);
    let placement = xyz_placement(&mut model);
    let surf = model.geometry.surfaces.push(Surface::Plane(Plane3 {
        position: placement,
    }));
    let ap_id = model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
            name: "anno".into(),
            styles: vec![],
            item: RepresentationItemRef::Surface(surf),
        }));
    let used = model.representations.push(Representation::Plain(PlainRepr {
        name: "draughting_model".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(ctx_id)),
        frame: None,
    }));
    let def = model.representations.push(Representation::Plain(PlainRepr {
        name: "definition".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(ctx_id)),
        frame: None,
    }));
    model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_model_item_associations
        .push(DraughtingModelItemAssociation {
            name: "link".into(),
            description: None,
            definition: DraughtingModelItemDefinition::Representation(def),
            used_representation: used,
            identified_item: DraughtingModelIdentifiedItem::AnnotationOccurrence(ap_id),
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.draughting_model_item_associations.len(), 1);
    let dmia = re_pmi
        .draughting_model_item_associations
        .iter()
        .next()
        .unwrap();
    assert_eq!(dmia.name, "link");
    assert!(matches!(
        dmia.identified_item,
        DraughtingModelIdentifiedItem::AnnotationOccurrence(_)
    ));
}

#[test]
fn text_literal_round_trip() {
    use step_io::ir::pmi::{DraughtingPreDefinedTextFont, PmiPool};
    use step_io::ir::visualization::{
        Axis2Placement, FontSelect, TextLiteral, TextPath, VisualizationPool,
    };
    let mut model = empty_model();
    let placement = xyz_placement(&mut model);
    let font_id = model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_pre_defined_text_fonts
        .push(DraughtingPreDefinedTextFont {
            name: "font_a".into(),
        });
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.text_literals.push(TextLiteral {
        name: String::new(),
        literal: "hello".into(),
        placement: Axis2Placement::D3(placement),
        alignment: "baseline left".into(),
        path: TextPath::Right,
        font: FontSelect::DraughtingPreDefined(font_id),
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.text_literals.len(), 1);
    let re_t = re_viz.text_literals.iter().next().unwrap();
    assert_eq!(re_t.literal, "hello");
    assert_eq!(re_t.alignment, "baseline left");
    assert!(matches!(re_t.path, TextPath::Right));
    assert!(matches!(re_t.placement, Axis2Placement::D3(_)));
}

#[test]
fn composite_text_round_trip() {
    use step_io::ir::pmi::{DraughtingPreDefinedTextFont, PmiPool};
    use step_io::ir::visualization::{
        Axis2Placement, CompositeText, FontSelect, TextLiteral, TextOrCharacter, TextPath,
        VisualizationPool,
    };
    let mut model = empty_model();
    let placement = xyz_placement(&mut model);
    let font_id = model
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_pre_defined_text_fonts
        .push(DraughtingPreDefinedTextFont {
            name: "font_a".into(),
        });
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let tl1 = viz.text_literals.push(TextLiteral {
        name: String::new(),
        literal: "a".into(),
        placement: Axis2Placement::D3(placement),
        alignment: String::new(),
        path: TextPath::Right,
        font: FontSelect::DraughtingPreDefined(font_id),
    });
    let tl2 = viz.text_literals.push(TextLiteral {
        name: String::new(),
        literal: "b".into(),
        placement: Axis2Placement::D3(placement),
        alignment: String::new(),
        path: TextPath::Right,
        font: FontSelect::DraughtingPreDefined(font_id),
    });
    viz.composite_texts.push(CompositeText {
        name: String::new(),
        collected_text: vec![
            TextOrCharacter::TextLiteral(tl1),
            TextOrCharacter::TextLiteral(tl2),
        ],
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.text_literals.len(), 2);
    assert_eq!(re_viz.composite_texts.len(), 1);
    let re_ct = re_viz.composite_texts.iter().next().unwrap();
    assert_eq!(re_ct.collected_text.len(), 2);
}

#[test]
fn text_style_with_box_characteristics_round_trip() {
    use step_io::ir::visualization::{
        BoxCharacteristic, CharacterStyle, Colour, ColourRgb, TextStyle, TextStyleData,
        TextStyleForDefinedFont, TextStyleWithBoxCharacteristics, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    }));
    let t4df_id = viz
        .text_styles_for_defined_font
        .push(TextStyleForDefinedFont {
            text_colour: colour_id,
        });
    viz.text_styles.push(TextStyle::WithBoxCharacteristics(
        TextStyleWithBoxCharacteristics {
            inherited: TextStyleData {
                name: String::new(),
                character_appearance: CharacterStyle::TextStyleForDefinedFont(t4df_id),
            },
            characteristics: vec![
                BoxCharacteristic::Height(3.0),
                BoxCharacteristic::Width(2.001),
                BoxCharacteristic::SlantAngle(0.0),
                BoxCharacteristic::RotateAngle(0.0),
            ],
        },
    ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.text_styles.len(), 1);
    let TextStyle::WithBoxCharacteristics(re_t) = re_viz.text_styles.iter().next().unwrap() else {
        panic!("expected WithBoxCharacteristics");
    };
    assert_eq!(re_t.characteristics.len(), 4);
    assert!(matches!(
        re_t.characteristics[0],
        BoxCharacteristic::Height(v) if (v - 3.0).abs() < 1e-9
    ));
    assert!(matches!(
        re_t.characteristics[1],
        BoxCharacteristic::Width(v) if (v - 2.001).abs() < 1e-9
    ));
}

#[test]
fn point_style_round_trip() {
    use step_io::ir::visualization::{
        Colour, ColourRgb, FoundedItem, Marker, MarkerSize, PointStyle, PreDefinedMarker,
        PreDefinedMarkerData, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.0,
        green: 1.0,
        blue: 0.0,
    }));
    let marker_id = viz
        .pre_defined_markers
        .push(PreDefinedMarker::Plain(PreDefinedMarkerData {
            name: "x".into(),
        }));
    viz.founded_items.push(FoundedItem::PointStyle(PointStyle {
        name: String::new(),
        marker: Marker::Predefined(marker_id),
        marker_size: MarkerSize::PositiveLength(0.7),
        marker_colour: colour_id,
    }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    let count = re_viz
        .founded_items
        .iter()
        .filter(|f| matches!(f, FoundedItem::PointStyle(_)))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn defined_symbol_round_trip() {
    // SYMBOL_TARGET + DEFINED_SYMBOL via the GeometricRepresentationItem
    // arena. Definition is a PreDefinedSymbol, target is a SymbolTarget
    // sharing the same arena.
    use step_io::ir::visualization::{
        DefinedSymbol, DefinedSymbolDefinition, GeometricRepresentationItem, PreDefinedSymbol,
        PreDefinedSymbolData, SymbolPlacement, SymbolTarget, VisualizationPool,
    };
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let pds_id = viz
        .pre_defined_symbols
        .push(PreDefinedSymbol::Plain(PreDefinedSymbolData {
            name: "filled arrow".into(),
        }));
    let target_id =
        model
            .geometric_representation_items
            .push(GeometricRepresentationItem::SymbolTarget(SymbolTarget {
                name: "tgt".into(),
                placement: SymbolPlacement::Placement3d(frame),
                x_scale: 3.5,
                y_scale: 3.5,
            }));
    model
        .geometric_representation_items
        .push(GeometricRepresentationItem::DefinedSymbol(DefinedSymbol {
            name: "sym".into(),
            definition: DefinedSymbolDefinition::PreDefinedSymbol(pds_id),
            target: target_id,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let ds = re
        .geometric_representation_items
        .iter()
        .find_map(|i| match i {
            GeometricRepresentationItem::DefinedSymbol(d) => Some(d),
            GeometricRepresentationItem::SymbolTarget(_)
            | GeometricRepresentationItem::ShellBasedSurfaceModel(_)
            | GeometricRepresentationItem::GeometricCurveSet(_)
            | GeometricRepresentationItem::GeometricSet(_) => None,
        })
        .expect("defined_symbol round-trips");
    assert_eq!(ds.name, "sym");
    let st = re
        .geometric_representation_items
        .iter()
        .find_map(|i| match i {
            GeometricRepresentationItem::SymbolTarget(t) => Some(t),
            GeometricRepresentationItem::DefinedSymbol(_)
            | GeometricRepresentationItem::ShellBasedSurfaceModel(_)
            | GeometricRepresentationItem::GeometricCurveSet(_)
            | GeometricRepresentationItem::GeometricSet(_) => None,
        })
        .expect("symbol_target round-trips");
    assert!((st.x_scale - 3.5).abs() < f64::EPSILON);
    assert!((st.y_scale - 3.5).abs() < f64::EPSILON);
}

#[test]
fn pre_defined_point_marker_symbol_round_trip() {
    // PRE_DEFINED_POINT_MARKER_SYMBOL — pre_defined_marker subtype that
    // appears as a simple instance in the corpus (`PRE_DEFINED_POINT_MARKER_SYMBOL('x')`),
    // not as a complex MI.
    use step_io::ir::visualization::{
        PreDefinedMarker, PreDefinedPointMarkerSymbol, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.pre_defined_markers
        .push(PreDefinedMarker::PointMarkerSymbol(
            PreDefinedPointMarkerSymbol { name: "x".into() },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    let p = re_viz
        .pre_defined_markers
        .iter()
        .find_map(|m| match m {
            PreDefinedMarker::PointMarkerSymbol(p) => Some(p),
            PreDefinedMarker::Plain(_) => None,
        })
        .expect("PointMarkerSymbol round-trips");
    assert_eq!(p.name, "x");
}

#[test]
fn surface_style_boundary_round_trip() {
    // SURFACE_STYLE_BOUNDARY — founded_item subtype with a curve_or_render
    // SELECT. Uses the SurfaceStyleRendering branch to keep fixture
    // setup minimal.
    use step_io::ir::visualization::{
        Colour, ColourRgb, CurveOrRender, FoundedItem, ShadingMethod, SurfaceStyleBoundary,
        SurfaceStyleRendering, SurfaceStyleRenderingData, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.2,
        green: 0.3,
        blue: 0.4,
    }));
    let ssr_id = viz
        .surface_style_renderings
        .push(SurfaceStyleRendering::Itself(SurfaceStyleRenderingData {
            rendering_method: ShadingMethod::Constant,
            surface_colour: colour_id,
        }));
    viz.founded_items
        .push(FoundedItem::SurfaceStyleBoundary(SurfaceStyleBoundary {
            style_of_boundary: CurveOrRender::SurfaceStyleRendering(ssr_id),
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    let count = re_viz
        .founded_items
        .iter()
        .filter(|f| matches!(f, FoundedItem::SurfaceStyleBoundary(_)))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn surface_style_parameter_line_round_trip() {
    // SURFACE_STYLE_PARAMETER_LINE — founded_item subtype with a
    // curve_or_render SELECT plus a SET[1:2] of direction_count_select.
    use step_io::ir::visualization::{
        Colour, ColourRgb, CurveOrRender, DirectionCount, FoundedItem, ShadingMethod,
        SurfaceStyleParameterLine, SurfaceStyleRendering, SurfaceStyleRenderingData,
        VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.1,
        green: 0.2,
        blue: 0.3,
    }));
    let ssr_id = viz
        .surface_style_renderings
        .push(SurfaceStyleRendering::Itself(SurfaceStyleRenderingData {
            rendering_method: ShadingMethod::Constant,
            surface_colour: colour_id,
        }));
    viz.founded_items
        .push(FoundedItem::SurfaceStyleParameterLine(
            SurfaceStyleParameterLine {
                style_of_parameter_lines: CurveOrRender::SurfaceStyleRendering(ssr_id),
                direction_counts: vec![DirectionCount::U(4), DirectionCount::V(7)],
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    let sspl = re_viz
        .founded_items
        .iter()
        .find_map(|f| match f {
            FoundedItem::SurfaceStyleParameterLine(s) => Some(s),
            _ => None,
        })
        .expect("expected SurfaceStyleParameterLine");
    assert_eq!(sspl.direction_counts.len(), 2);
    assert_eq!(sspl.direction_counts[0], DirectionCount::U(4));
    assert_eq!(sspl.direction_counts[1], DirectionCount::V(7));
}

#[test]
fn symbol_style_round_trip() {
    use step_io::ir::visualization::{
        Colour, ColourRgb, FoundedItem, SymbolColour, SymbolStyle, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 0.5,
        green: 0.5,
        blue: 0.5,
    }));
    let sc_id = viz.symbol_colours.push(SymbolColour {
        colour_of_symbol: colour_id,
    });
    viz.founded_items
        .push(FoundedItem::SymbolStyle(SymbolStyle {
            name: "ss".into(),
            style_of_symbol: sc_id,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    let count = re_viz
        .founded_items
        .iter()
        .filter(|f| matches!(f, FoundedItem::SymbolStyle(_)))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn symbol_colour_round_trip() {
    // SYMBOL_COLOUR (phase symbol-colour).
    use step_io::ir::visualization::{Colour, ColourRgb, SymbolColour, VisualizationPool};

    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let colour_id = viz.colours.push(Colour::Rgb(ColourRgb {
        name: String::new(),
        red: 1.0,
        green: 0.0,
        blue: 0.0,
    }));
    viz.symbol_colours.push(SymbolColour {
        colour_of_symbol: colour_id,
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz pool");
    assert_eq!(re_viz.symbol_colours.len(), 1);
}

#[test]
fn ciwr_round_trip() {
    // CHARACTERIZED_ITEM_WITHIN_REPRESENTATION (phase characterized-object-ciwr).
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::shape_rep::{
        CharacterizedItemWithinRepresentation, CharacterizedObject, CharacterizedObjectData,
        PlainRepr, Representation,
    };

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let p0 = model.geometry.points.push(Point3 {
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
    let position = push_placement(&mut model, p0, Some(axis), Some(refd));
    let surf = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let rep_id = model.representations.push(Representation::Plain(PlainRepr {
        name: "rep".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(
            UnitContextId(0),
        )),
        frame: None,
    }));
    model
        .characterized_objects
        .push(CharacterizedObject::CharacterizedItemWithinRepresentation(
            CharacterizedItemWithinRepresentation {
                inherited: CharacterizedObjectData {
                    name: "ciwr".into(),
                    description: Some("d".into()),
                },
                item: RepresentationItemRef::Surface(surf),
                rep: rep_id,
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.characterized_objects.len(), 1);
    let CharacterizedObject::CharacterizedItemWithinRepresentation(ciwr) =
        re.characterized_objects.iter().next().unwrap()
    else {
        panic!("expected CIWR");
    };
    assert_eq!(ciwr.inherited.name, "ciwr");
    assert_eq!(ciwr.inherited.description.as_deref(), Some("d"));
    assert!(matches!(ciwr.item, RepresentationItemRef::Surface(_)));
}

#[test]
fn property_definition_with_ciwr_target_round_trips() {
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::property::{
        CharacterizedDefinition, PropertyDefinition, PropertyDefinitionData, PropertyPool,
    };
    use step_io::ir::shape_rep::{
        CharacterizedItemWithinRepresentation, CharacterizedObject, CharacterizedObjectData,
        PlainRepr, Representation,
    };
    // A geometric-validation-property PROPERTY_DEFINITION whose `definition`
    // is a CHARACTERIZED_ITEM_WITHIN_REPRESENTATION (a characterized_object
    // subtype). Must round-trip: write PD -> #ciwr (forward ref, CIWR body
    // emits later under the reserved id) and read it back as the variant.
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    model.units.push(ctx);
    let p0 = model.geometry.points.push(Point3 {
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
    let position = push_placement(&mut model, p0, Some(axis), Some(refd));
    let surf = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    let rep_id = model.representations.push(Representation::Plain(PlainRepr {
        name: "validation".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(
            UnitContextId(0),
        )),
        frame: None,
    }));
    let co_id = model.characterized_objects.push(
        CharacterizedObject::CharacterizedItemWithinRepresentation(
            CharacterizedItemWithinRepresentation {
                inherited: CharacterizedObjectData {
                    name: String::new(),
                    description: None,
                },
                item: RepresentationItemRef::Surface(surf),
                rep: rep_id,
            },
        ),
    );

    let mut pool = PropertyPool::default();
    pool.property_definitions
        .push(PropertyDefinition::Itself(PropertyDefinitionData {
            name: "geometric validation property".into(),
            description: String::new(),
            definition: CharacterizedDefinition::CharacterizedItemWithinRepresentation(co_id),
        }));
    model.properties = Some(pool);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pool = re
        .properties
        .as_ref()
        .expect("round-tripped has properties");
    let has_ciwr_pd = re_pool.property_definitions.iter().any(|pd| {
        matches!(
            pd,
            PropertyDefinition::Itself(d)
                if matches!(
                    d.definition,
                    CharacterizedDefinition::CharacterizedItemWithinRepresentation(_)
                )
        )
    });
    assert!(
        has_ciwr_pd,
        "PROPERTY_DEFINITION with a CIWR definition should round-trip"
    );
}

#[test]
fn qri_vri_round_trip() {
    // QUALIFIED_REPRESENTATION_ITEM + VALUE_REPRESENTATION_ITEM in the new
    // representation_item arena (phase repr-item-arena-1).
    use step_io::ir::pmi::{TypeQualifier, ValueFormatTypeQualifier};
    use step_io::ir::representation_item::{
        MeasureValue, QualifiedRepresentationItem, QualifierRef, RepresentationItem,
        ValueRepresentationItem,
    };

    let mut model = empty_model();
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let tq = pmi.type_qualifiers.push(TypeQualifier {
        name: "maximum".into(),
    });
    let vftq = pmi
        .value_format_type_qualifiers
        .push(ValueFormatTypeQualifier {
            format_type: "NR2 1.3".into(),
        });
    model
        .representation_items
        .push(RepresentationItem::QualifiedRepresentationItem(
            QualifiedRepresentationItem {
                name: "q".into(),
                qualifiers: vec![
                    QualifierRef::TypeQualifier(tq),
                    QualifierRef::ValueFormatTypeQualifier(vftq),
                ],
            },
        ));
    model
        .representation_items
        .push(RepresentationItem::ValueRepresentationItem(
            ValueRepresentationItem {
                name: "v".into(),
                value_component: MeasureValue::Real {
                    type_name: "POSITIVE_LENGTH_MEASURE".into(),
                    value: 0.05,
                },
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.representation_items.len(), 2);
    let mut iter = re.representation_items.iter();
    let RepresentationItem::QualifiedRepresentationItem(qri) = iter.next().unwrap() else {
        panic!("expected QRI");
    };
    assert_eq!(qri.qualifiers.len(), 2);
    let RepresentationItem::ValueRepresentationItem(vri) = iter.next().unwrap() else {
        panic!("expected VRI");
    };
    let MeasureValue::Real { type_name, value } = &vri.value_component else {
        panic!("expected Real");
    };
    assert_eq!(type_name, "POSITIVE_LENGTH_MEASURE");
    assert!((value - 0.05).abs() < 1e-9);
}

#[test]
fn measure_qualification_round_trip() {
    // MEASURE_QUALIFICATION — qualifiers SET covers the two corpus-modelled
    // value_qualifier variants (TypeQualifier, ValueFormatTypeQualifier).
    use step_io::ir::pmi::{
        MeasureQualification, TypeQualifier, ValueFormatTypeQualifier, ValueQualifier,
    };
    use step_io::ir::units::MeasureWithUnit;

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);

    let mwu_id = model
        .units_pool
        .as_mut()
        .expect("units pool seeded")
        .measure_with_units
        .push(MeasureWithUnit::Length {
            value: 1.0,
            unit: length_unit,
        });
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let tq = pmi.type_qualifiers.push(TypeQualifier {
        name: "maximum".into(),
    });
    let vftq = pmi
        .value_format_type_qualifiers
        .push(ValueFormatTypeQualifier {
            format_type: "NR2 1.3".into(),
        });
    pmi.measure_qualifications.push(MeasureQualification {
        name: "mq".into(),
        description: String::new(),
        qualified_measure: mwu_id,
        qualifiers: vec![
            ValueQualifier::TypeQualifier(tq),
            ValueQualifier::ValueFormatTypeQualifier(vftq),
        ],
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.measure_qualifications.len(), 1);
    let mq = re_pmi.measure_qualifications.iter().next().unwrap();
    assert_eq!(mq.name, "mq");
    assert_eq!(mq.qualifiers.len(), 2);
    assert!(matches!(mq.qualifiers[0], ValueQualifier::TypeQualifier(_)));
    assert!(matches!(
        mq.qualifiers[1],
        ValueQualifier::ValueFormatTypeQualifier(_)
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn projected_zone_definition_round_trip() {
    // PROJECTED_ZONE_DEFINITION — single_struct in the tolerance_zone_definition
    // arena. Refs ToleranceZone + ShapeAspect (projection_end) +
    // MeasureWithUnit (projected_length).
    use step_io::ir::pmi::{
        GeometricTolerance, GeometricToleranceData, GeometricToleranceRef, ProjectedZoneDefinition,
        ToleranceMagnitude, ToleranceZoneForm,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::ToleranceZone;
    use step_io::ir::units::MeasureWithUnit;

    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();
    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let sa_end = model.shape_aspects.push(ShapeAspect {
        name: "end".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let mwu_id = model
        .units_pool
        .as_mut()
        .expect("units pool seeded")
        .measure_with_units
        .push(MeasureWithUnit::Length {
            value: 5.0,
            unit: length_unit,
        });
    let pmi = model.pmi.get_or_insert_with(PmiPool::default);
    let gt = pmi
        .geometric_tolerances
        .push(GeometricTolerance::Flatness(GeometricToleranceData {
            name: "t".into(),
            description: String::new(),
            magnitude: ToleranceMagnitude::Measure(PropertyMeasure {
                name: String::new(),
                kind: MeasureKind::Length,
                value: 0.1,
                unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
            }),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            modifiers: Vec::new(),
            unit_size: None,
            defined_area_unit: None,
        }));
    let form = pmi.tolerance_zone_forms.push(ToleranceZoneForm {
        name: "cylindrical".into(),
    });
    let tz_id = model.tolerance_zones.push(ToleranceZone {
        name: "tz".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        defining_tolerance: vec![GeometricToleranceRef::Plain(gt)],
        form,
    });
    model
        .pmi
        .as_mut()
        .unwrap()
        .tolerance_zone_definitions
        .push(ProjectedZoneDefinition {
            zone: tz_id,
            boundaries: vec![ShapeAspectRef::ShapeAspect(sa)],
            projection_end: ShapeAspectRef::ShapeAspect(sa_end),
            projected_length: mwu_id,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.expect("pmi pool");
    assert_eq!(re_pmi.tolerance_zone_definitions.len(), 1);
    let pzd = re_pmi.tolerance_zone_definitions.iter().next().unwrap();
    assert_eq!(pzd.boundaries.len(), 1);
    assert!(matches!(pzd.projection_end, ShapeAspectRef::ShapeAspect(_)));
}

#[test]
fn datum_system_round_trip() {
    // DATUM_SYSTEM — a shape_aspect subtype whose `constituents` reference
    // the general_datum_reference arena.
    use step_io::ir::pmi::{
        Datum, GeneralDatumBase, GeneralDatumReference, GeneralDatumReferenceData,
    };
    use step_io::ir::shape_rep::DatumSystem;
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let datum = pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    let gdr = pmi
        .general_datum_references
        .push(GeneralDatumReference::Compartment(
            GeneralDatumReferenceData {
                name: String::new(),
                description: String::new(),
                target: part_pid,
                product_definitional: false,
                base: GeneralDatumBase::Datum(datum),
            },
        ));
    model.pmi = Some(pmi);

    model.datum_systems.push(DatumSystem {
        name: "DS".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        constituents: vec![gdr],
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.datum_systems.len(), 1);
    let ds = re.datum_systems.iter().next().unwrap();
    assert_eq!(ds.name, "DS");
    assert_eq!(ds.constituents.len(), 1, "constituent round-trips");
    assert_eq!(ds.target, step_io::ProductId(0));
}

#[test]
fn datum_target_cluster_round_trip() {
    // DATUM_TARGET + PLACED_DATUM_TARGET_FEATURE + FEATURE_FOR_DATUM_TARGET_RELATIONSHIP.
    // Three new shape_aspect-family entities sharing the same product chain.
    use step_io::ir::shape_aspect_ref::ShapeAspectRef;
    use step_io::ir::shape_rep::{
        DatumTarget, PlacedDatumTargetFeature, ShapeAspect, ShapeAspectRelationship,
        ShapeAspectRelationshipKind,
    };
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
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa_id = model.shape_aspects.push(ShapeAspect {
        name: "feature".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let dt_id = model.datum_targets.push(DatumTarget {
        name: "datum target".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        target_id: "A1".into(),
    });
    let _pdtf_id = model
        .placed_datum_target_features
        .push(PlacedDatumTargetFeature {
            name: "placed target".into(),
            description: String::new(),
            target: part_pid,
            product_definitional: false,
            target_id: "B2".into(),
        });
    model
        .shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name: String::new(),
            description: String::new(),
            relating_shape_aspect: ShapeAspectRef::ShapeAspect(sa_id),
            related_shape_aspect: ShapeAspectRef::DatumTarget(dt_id),
            kind: ShapeAspectRelationshipKind::FeatureForDatumTarget,
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.datum_targets.len(), 1, "DATUM_TARGET round-trips");
    assert_eq!(
        re.datum_targets.iter().next().unwrap().target_id,
        "A1",
        "target_id preserved"
    );
    assert_eq!(
        re.placed_datum_target_features.len(),
        1,
        "PLACED_DATUM_TARGET_FEATURE round-trips"
    );
    assert_eq!(
        re.placed_datum_target_features
            .iter()
            .next()
            .unwrap()
            .target_id,
        "B2"
    );
    assert_eq!(re.shape_aspect_relationships.len(), 1);
    let rel = re.shape_aspect_relationships.iter().next().unwrap();
    assert_eq!(rel.kind, ShapeAspectRelationshipKind::FeatureForDatumTarget);
    assert!(matches!(
        rel.related_shape_aspect,
        ShapeAspectRef::DatumTarget(_)
    ));
}

#[test]
fn geometric_tolerance_with_datum_reference_round_trip() {
    // PERPENDICULARITY_TOLERANCE — a geometric_tolerance_with_datum_reference
    // simple variant: magnitude + toleranced_shape_aspect + datum_system list.
    use step_io::ir::pmi::{
        GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData,
        ToleranceMagnitude,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::DatumSystem;
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });

    // DatumSystem with empty `constituents` — the constituent resolution is
    // covered by `datum_system_round_trip`; this test exercises the tolerance.
    let ds = model.datum_systems.push(DatumSystem {
        name: "DS".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        constituents: Vec::new(),
    });

    let mut pmi = PmiPool::default();
    pmi.geometric_tolerance_with_datum_references.push(
        GeometricToleranceWithDatumReference::Perpendicularity(
            GeometricToleranceWithDatumReferenceData {
                name: "t".into(),
                description: String::new(),
                magnitude: ToleranceMagnitude::Measure(PropertyMeasure {
                    name: String::new(),
                    kind: MeasureKind::Length,
                    value: 0.1,
                    unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
                }),
                toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
                datum_system: vec![ds],
                modifiers: Vec::new(),
                displacement: None,
            },
        ),
    );
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.geometric_tolerance_with_datum_references.len(), 1);
    let gt = re_pmi
        .geometric_tolerance_with_datum_references
        .iter()
        .next()
        .unwrap();
    let GeometricToleranceWithDatumReference::Perpendicularity(d) = gt else {
        panic!("expected Perpendicularity, got {gt:?}");
    };
    assert_eq!(d.datum_system.len(), 1, "datum_system round-trips");
    assert!(matches!(
        d.toleranced_shape_aspect,
        ShapeAspectRef::ShapeAspect(_)
    ));
}

#[test]
fn complex_datum_ref_tolerance_round_trip() {
    // POSITION_TOLERANCE — emitted as the multiple-inheritance complex
    // (GEOMETRIC_TOLERANCE GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE
    // POSITION_TOLERANCE) and read back through the complex handler.
    use step_io::ir::pmi::{
        GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData,
        ToleranceMagnitude,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::DatumSystem;
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let ds = model.datum_systems.push(DatumSystem {
        name: "DS".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        constituents: Vec::new(),
    });

    let mut pmi = PmiPool::default();
    pmi.geometric_tolerance_with_datum_references.push(
        GeometricToleranceWithDatumReference::Position(GeometricToleranceWithDatumReferenceData {
            name: "t".into(),
            description: String::new(),
            magnitude: ToleranceMagnitude::Measure(PropertyMeasure {
                name: String::new(),
                kind: MeasureKind::Length,
                value: 0.1,
                unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
            }),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            datum_system: vec![ds],
            modifiers: Vec::new(),
            displacement: None,
        }),
    );
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE"),
        "POSITION_TOLERANCE emits as the complex MI form"
    );
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.geometric_tolerance_with_datum_references.len(), 1);
    let gt = re_pmi
        .geometric_tolerance_with_datum_references
        .iter()
        .next()
        .unwrap();
    let GeometricToleranceWithDatumReference::Position(d) = gt else {
        panic!("expected Position, got {gt:?}");
    };
    assert_eq!(d.datum_system.len(), 1, "datum_system round-trips");
}

#[test]
#[allow(clippy::too_many_lines)]
fn geometric_tolerance_with_modifiers_round_trip() {
    // Phase gt-modifiers — GEOMETRIC_TOLERANCE_WITH_MODIFIERS part round-trip.
    // Tests both:
    // - datum-ref Position with modifier (4-part complex MI)
    // - form-tolerance Roundness with modifier (3-part complex MI; the
    //   simple form would be a standalone FLATNESS/ROUNDNESS).
    use step_io::ir::pmi::{
        Datum, GeneralDatumBase, GeneralDatumReference, GeneralDatumReferenceData,
        GeometricTolerance, GeometricToleranceData, GeometricToleranceModifier,
        GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData,
        ToleranceMagnitude,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::{DatumSystem, ShapeAspect};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let sa = model.shape_aspects.push(ShapeAspect {
        name: "feature".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let ds = model.datum_systems.push(DatumSystem {
        name: "DS".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        constituents: Vec::new(),
    });
    let mut pmi = PmiPool::default();
    let _datum = pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    let _gdr = pmi
        .general_datum_references
        .push(GeneralDatumReference::Compartment(
            GeneralDatumReferenceData {
                name: String::new(),
                description: String::new(),
                target: part_pid,
                product_definitional: false,
                base: GeneralDatumBase::Datum(step_io::ir::DatumId(0)),
            },
        ));
    let magnitude = || {
        ToleranceMagnitude::Measure(PropertyMeasure {
            name: String::new(),
            kind: MeasureKind::Length,
            value: 0.1,
            unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
        })
    };
    pmi.geometric_tolerance_with_datum_references.push(
        GeometricToleranceWithDatumReference::Position(GeometricToleranceWithDatumReferenceData {
            name: "P".into(),
            description: String::new(),
            magnitude: magnitude(),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            datum_system: vec![ds],
            modifiers: vec![GeometricToleranceModifier::MaximumMaterialRequirement],
            displacement: None,
        }),
    );
    pmi.geometric_tolerances
        .push(GeometricTolerance::Roundness(GeometricToleranceData {
            name: "R".into(),
            description: String::new(),
            magnitude: magnitude(),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            modifiers: vec![GeometricToleranceModifier::LeastMaterialRequirement],
            unit_size: None,
            defined_area_unit: None,
        }));
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("GEOMETRIC_TOLERANCE_WITH_MODIFIERS"),
        "expected WM part in complex MI output"
    );
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    // datum-ref Position with modifier survives the round-trip.
    let pos = re_pmi
        .geometric_tolerance_with_datum_references
        .iter()
        .find_map(|gt| match gt {
            GeometricToleranceWithDatumReference::Position(d) => Some(d),
            _ => None,
        })
        .expect("Position variant present after round-trip");
    assert_eq!(
        pos.modifiers,
        vec![GeometricToleranceModifier::MaximumMaterialRequirement],
        "Position modifier preserved"
    );
    // Form-tolerance Roundness with modifier survives.
    let round = re_pmi
        .geometric_tolerances
        .iter()
        .find_map(|gt| match gt {
            GeometricTolerance::Roundness(d) => Some(d),
            _ => None,
        })
        .expect("Roundness variant present after round-trip");
    assert_eq!(
        round.modifiers,
        vec![GeometricToleranceModifier::LeastMaterialRequirement],
        "Roundness modifier preserved"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn gt_defined_unit_area_unit_displacement_round_trip() {
    // Phase gt-defined-disposed — three new complex MI parts share the
    // GT base struct:
    // - WDU only (Flatness + unit_size)
    // - WDU + WDAU (Flatness + unit_size + rectangular area + second_unit_size)
    // - WDR + UD (SurfaceProfile + datum_system + displacement)
    use step_io::ir::pmi::{
        AreaUnitType, Datum, DefinedAreaUnit, GeneralDatumBase, GeneralDatumReference,
        GeneralDatumReferenceData, GeometricTolerance, GeometricToleranceData,
        GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData,
        ToleranceMagnitude,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    use step_io::ir::shape_rep::{DatumSystem, ShapeAspect};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    // Push an MWU into the units pool so unit_size / displacement refs
    // have a valid step id after emit (mwu_step_ids[0]).
    let mwu_id = {
        let pool = model.units_pool.as_mut().expect("units pool exists");
        pool.measure_with_units
            .push(step_io::ir::units::MeasureWithUnit::Length {
                value: 1.0,
                unit: length_unit,
            })
    };
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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
    let sa = model.shape_aspects.push(ShapeAspect {
        name: "feature".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });
    let ds = model.datum_systems.push(DatumSystem {
        name: "DS".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        constituents: Vec::new(),
    });
    let mut pmi = PmiPool::default();
    let _datum = pmi.datums.push(Datum {
        name: String::new(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
        identification: "A".into(),
    });
    let _gdr = pmi
        .general_datum_references
        .push(GeneralDatumReference::Compartment(
            GeneralDatumReferenceData {
                name: String::new(),
                description: String::new(),
                target: part_pid,
                product_definitional: false,
                base: GeneralDatumBase::Datum(step_io::ir::DatumId(0)),
            },
        ));
    let magnitude = || {
        ToleranceMagnitude::Measure(PropertyMeasure {
            name: String::new(),
            kind: MeasureKind::Length,
            value: 0.1,
            unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
        })
    };
    // Flatness + unit_size only
    pmi.geometric_tolerances
        .push(GeometricTolerance::Flatness(GeometricToleranceData {
            name: "F1".into(),
            description: String::new(),
            magnitude: magnitude(),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            modifiers: Vec::new(),
            unit_size: Some(mwu_id),
            defined_area_unit: None,
        }));
    // Flatness + unit_size + rectangular area + second_unit_size
    pmi.geometric_tolerances
        .push(GeometricTolerance::Flatness(GeometricToleranceData {
            name: "F2".into(),
            description: String::new(),
            magnitude: magnitude(),
            toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
            modifiers: Vec::new(),
            unit_size: Some(mwu_id),
            defined_area_unit: Some(DefinedAreaUnit {
                area_type: AreaUnitType::Rectangular,
                second_unit_size: Some(mwu_id),
            }),
        }));
    // SurfaceProfile + datum_system + displacement
    pmi.geometric_tolerance_with_datum_references.push(
        GeometricToleranceWithDatumReference::SurfaceProfile(
            GeometricToleranceWithDatumReferenceData {
                name: "SP".into(),
                description: String::new(),
                magnitude: magnitude(),
                toleranced_shape_aspect: ShapeAspectRef::ShapeAspect(sa),
                datum_system: vec![ds],
                modifiers: Vec::new(),
                displacement: Some(mwu_id),
            },
        ),
    );
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    assert!(
        text.contains("GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT"),
        "expected WDU part"
    );
    assert!(
        text.contains("GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT"),
        "expected WDAU part"
    );
    assert!(
        text.contains("UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE"),
        "expected UD part"
    );
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    // Two Flatness variants — first WDU-only, second WDU+WDAU.
    let flats: Vec<_> = re_pmi
        .geometric_tolerances
        .iter()
        .filter_map(|gt| match gt {
            GeometricTolerance::Flatness(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(flats.len(), 2);
    assert!(flats[0].unit_size.is_some(), "F1 unit_size preserved");
    assert!(flats[0].defined_area_unit.is_none(), "F1 has no area unit");
    assert!(flats[1].unit_size.is_some(), "F2 unit_size preserved");
    let area = flats[1]
        .defined_area_unit
        .as_ref()
        .expect("F2 area unit preserved");
    assert_eq!(area.area_type, AreaUnitType::Rectangular);
    assert!(
        area.second_unit_size.is_some(),
        "F2 second_unit_size preserved"
    );
    // SurfaceProfile with displacement.
    let sp = re_pmi
        .geometric_tolerance_with_datum_references
        .iter()
        .find_map(|gt| match gt {
            GeometricToleranceWithDatumReference::SurfaceProfile(d) => Some(d),
            _ => None,
        })
        .expect("SurfaceProfile present after round-trip");
    assert!(sp.displacement.is_some(), "SP displacement preserved");
}

#[test]
fn plus_minus_tolerance_round_trip() {
    // PLUS_MINUS_TOLERANCE — range = TOLERANCE_VALUE, toleranced_dimension =
    // DIMENSIONAL_SIZE. Exercises both SELECT reference enums.
    use step_io::ir::pmi::{
        DimensionalCharacteristic, DimensionalSize, DimensionalSizeKind, PlusMinusTolerance,
        ToleranceMagnitude, ToleranceMethodDefinition, ToleranceValue,
    };
    use step_io::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let length_unit = ctx.length;
    model.units.push(ctx);
    let solid_id = push_minimal_solid(&mut model);
    let identity_frame = model.geometry.identity_placement();

    let mut tree = AssemblyTree::default();
    let part_pid = tree.products.push(Product {
        id: "Part".into(),
        name: "Part".into(),
        description: None,
        geometry: Some(GeometryLeaf::Solid(SolidContent {
            ids: vec![solid_id],
        })),
        instances: vec![],
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

    let sa = model.shape_aspects.push(ShapeAspect {
        name: "sa".into(),
        description: String::new(),
        target: part_pid,
        product_definitional: false,
    });

    let bound = || {
        ToleranceMagnitude::Measure(PropertyMeasure {
            name: String::new(),
            kind: MeasureKind::Length,
            value: 0.02,
            unit_ref: Some(PropertyMeasureUnit::Named(length_unit)),
        })
    };
    let mut pmi = PmiPool::default();
    let ds = pmi.dimensional_sizes.push(DimensionalSize {
        applies_to: ShapeAspectRef::ShapeAspect(sa),
        name: "diameter".into(),
        kind: DimensionalSizeKind::Plain,
    });
    let tv = pmi.tolerance_values.push(ToleranceValue {
        lower_bound: bound(),
        upper_bound: bound(),
    });
    pmi.plus_minus_tolerances.push(PlusMinusTolerance {
        range: ToleranceMethodDefinition::Value(tv),
        toleranced_dimension: DimensionalCharacteristic::Size(ds),
    });
    model.pmi = Some(pmi);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_pmi = re.pmi.as_ref().expect("pmi pool");
    assert_eq!(re_pmi.tolerance_values.len(), 1);
    assert_eq!(re_pmi.plus_minus_tolerances.len(), 1);
    let pmt = re_pmi.plus_minus_tolerances.iter().next().unwrap();
    assert!(matches!(pmt.range, ToleranceMethodDefinition::Value(_)));
    assert!(matches!(
        pmt.toleranced_dimension,
        DimensionalCharacteristic::Size(_)
    ));
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
fn pre_defined_curve_font_family_round_trip() {
    // Both PreDefinedCurveFont variants — Plain (corpus 0 self variant) and
    // Draughting (corpus 104k) — share the visualization::pre_defined_curve_fonts
    // arena per the ir.toml blueprint.
    use step_io::ir::visualization::{
        DraughtingPreDefinedCurveFont, PreDefinedCurveFont, PreDefinedCurveFontData,
        VisualizationPool,
    };
    let mut model = empty_model();
    let mut viz = VisualizationPool::default();
    viz.pre_defined_curve_fonts
        .push(PreDefinedCurveFont::Plain(PreDefinedCurveFontData {
            name: "continuous".into(),
        }));
    viz.pre_defined_curve_fonts
        .push(PreDefinedCurveFont::Draughting(
            DraughtingPreDefinedCurveFont {
                name: "dashed".into(),
            },
        ));
    model.visualization = Some(viz);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.as_ref().expect("visualization pool");
    assert_eq!(re_viz.pre_defined_curve_fonts.len(), 2);
    let mut iter = re_viz.pre_defined_curve_fonts.iter();
    match iter.next().unwrap() {
        PreDefinedCurveFont::Plain(d) => assert_eq!(d.name, "continuous"),
        PreDefinedCurveFont::Draughting(_) => panic!("expected Plain first"),
    }
    match iter.next().unwrap() {
        PreDefinedCurveFont::Draughting(d) => assert_eq!(d.name, "dashed"),
        PreDefinedCurveFont::Plain(_) => panic!("expected Draughting second"),
    }
}

#[test]
fn pre_defined_symbol_family_round_trip() {
    // PreDefinedSymbol Plain (corpus 0) + Terminator (PRE_DEFINED_TERMINATOR_SYMBOL,
    // corpus 116) share the visualization::pre_defined_symbols arena.
    use step_io::ir::visualization::{
        PreDefinedSymbol, PreDefinedSymbolData, PreDefinedTerminatorSymbol, VisualizationPool,
    };
    let mut model = empty_model();
    let mut viz = VisualizationPool::default();
    viz.pre_defined_symbols
        .push(PreDefinedSymbol::Plain(PreDefinedSymbolData {
            name: "symbol".into(),
        }));
    viz.pre_defined_symbols
        .push(PreDefinedSymbol::Terminator(PreDefinedTerminatorSymbol {
            name: "filled arrow".into(),
        }));
    model.visualization = Some(viz);

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.as_ref().expect("visualization pool");
    assert_eq!(re_viz.pre_defined_symbols.len(), 2);
    let mut iter = re_viz.pre_defined_symbols.iter();
    match iter.next().unwrap() {
        PreDefinedSymbol::Plain(s) => assert_eq!(s.name, "symbol"),
        PreDefinedSymbol::Terminator(_) => panic!("expected Plain first"),
    }
    match iter.next().unwrap() {
        PreDefinedSymbol::Terminator(s) => assert_eq!(s.name, "filled arrow"),
        PreDefinedSymbol::Plain(_) => panic!("expected Terminator second"),
    }
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
    let Some(TessellatedItem::CoordinatesList(c)) = re.tessellated_items.iter().next() else {
        panic!("expected CoordinatesList");
    };
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

#[test]
fn tessellation_2_round_trip() {
    // COORDINATES_LIST + TESSELLATED_CURVE_SET + COMPLEX_TRIANGULATED_SURFACE_SET
    // — both new entities reference the shared coordinates list.
    use step_io::ir::tessellation::{
        ComplexTriangulatedSurfaceSet, CoordinatesList, TessellatedCurveSet, TessellatedItem,
    };
    let mut model = empty_model();
    let coords = model
        .tessellated_items
        .push(TessellatedItem::CoordinatesList(CoordinatesList {
            name: "pts".into(),
            npoints: 4,
            position_coords: vec![
                vec![0.0, 0.0, 0.0],
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![1.0, 1.0, 0.0],
            ],
        }));
    model
        .tessellated_items
        .push(TessellatedItem::TessellatedCurveSet(TessellatedCurveSet {
            name: "curves".into(),
            coordinates: coords,
            line_strips: vec![vec![1, 2], vec![3, 4, 1]],
        }));
    model
        .tessellated_surface_sets
        .push(ComplexTriangulatedSurfaceSet {
            name: "surf".into(),
            coordinates: coords,
            pnmax: 4,
            normals: vec![vec![0.0, 0.0, 1.0]],
            pnindex: vec![1, 2, 3, 4],
            triangle_strips: vec![vec![1, 2, 3]],
            triangle_fans: vec![],
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    assert_eq!(re.tessellated_items.len(), 2);
    assert_eq!(re.tessellated_surface_sets.len(), 1);

    let curve_set = re
        .tessellated_items
        .iter()
        .find_map(|item| match item {
            TessellatedItem::TessellatedCurveSet(t) => Some(t),
            TessellatedItem::CoordinatesList(_)
            | TessellatedItem::TessellatedGeometricSet(_)
            | TessellatedItem::TessellatedSolid(_)
            | TessellatedItem::TessellatedShell(_)
            | TessellatedItem::RepositionedTessellatedItem(_) => None,
        })
        .expect("curve set");
    assert_eq!(curve_set.name, "curves");
    assert_eq!(curve_set.line_strips, vec![vec![1, 2], vec![3, 4, 1]]);

    let s = re.tessellated_surface_sets.iter().next().unwrap();
    assert_eq!(s.name, "surf");
    assert_eq!(s.pnmax, 4);
    assert_eq!(s.pnindex, vec![1, 2, 3, 4]);
    assert_eq!(s.triangle_strips, vec![vec![1, 2, 3]]);
    assert!(s.triangle_fans.is_empty());
    assert!((s.normals[0][2] - 1.0).abs() < f64::EPSILON);
}

#[test]
fn tessellated_geometric_set_round_trip() {
    // TESSELLATED_GEOMETRIC_SET — children exercise all 3 TessellatedItemRef
    // variants (Item / Face / SurfaceSet).
    use step_io::ir::tessellation::{
        ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList,
        TessellatedCurveSet, TessellatedGeometricSet, TessellatedItem, TessellatedItemRef,
    };
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
    let curve = model
        .tessellated_items
        .push(TessellatedItem::TessellatedCurveSet(TessellatedCurveSet {
            name: "curve".into(),
            coordinates: coords,
            line_strips: vec![vec![1, 2, 3]],
        }));
    let face = model.tessellated_faces.push(ComplexTriangulatedFace {
        name: "face".into(),
        coordinates: coords,
        pnmax: 3,
        normals: vec![vec![0.0, 0.0, 1.0]],
        geometric_link: None,
        pnindex: vec![1, 2, 3],
        triangle_strips: vec![vec![1, 2, 3]],
        triangle_fans: vec![],
    });
    let ss = model
        .tessellated_surface_sets
        .push(ComplexTriangulatedSurfaceSet {
            name: "ss".into(),
            coordinates: coords,
            pnmax: 3,
            normals: vec![vec![0.0, 0.0, 1.0]],
            pnindex: vec![1, 2, 3],
            triangle_strips: vec![vec![1, 2, 3]],
            triangle_fans: vec![],
        });
    model
        .tessellated_items
        .push(TessellatedItem::TessellatedGeometricSet(
            TessellatedGeometricSet {
                name: "gset".into(),
                children: vec![
                    TessellatedItemRef::Item(curve),
                    TessellatedItemRef::Face(face),
                    TessellatedItemRef::SurfaceSet(ss),
                ],
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let gset = re
        .tessellated_items
        .iter()
        .find_map(|item| match item {
            TessellatedItem::TessellatedGeometricSet(g) => Some(g),
            _ => None,
        })
        .expect("geometric set round-trips");
    assert_eq!(gset.name, "gset");
    assert_eq!(gset.children.len(), 3);
    assert!(matches!(gset.children[0], TessellatedItemRef::Item(_)));
    assert!(matches!(gset.children[1], TessellatedItemRef::Face(_)));
    assert!(matches!(
        gset.children[2],
        TessellatedItemRef::SurfaceSet(_)
    ));
}

#[test]
fn psa_null_style_round_trip() {
    // PRESENTATION_STYLE_ASSIGNMENT((NULL_STYLE(.NULL.))) — corpus NIST AP242.
    use step_io::ir::visualization::{
        PresentationStyleAssignment, PresentationStyleAssignmentData, PsaStyle, VisualizationPool,
    };
    let mut model = empty_model();
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.presentation_style_assignments
        .push(PresentationStyleAssignment::Itself(
            PresentationStyleAssignmentData {
                styles: vec![PsaStyle::Null],
            },
        ));

    let text = model.write_to_string().expect("write");
    assert!(text.contains("NULL_STYLE(.NULL.)"));
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz");
    let psa = re_viz
        .presentation_style_assignments
        .iter()
        .next()
        .expect("psa");
    let PresentationStyleAssignment::Itself(data) = psa else {
        panic!("expected Itself");
    };
    assert_eq!(data.styles.len(), 1);
    assert!(matches!(data.styles[0], PsaStyle::Null));
}

#[test]
fn presentation_style_by_context_round_trip() {
    // PRESENTATION_STYLE_BY_CONTEXT — PSA SUBTYPE with style_context.
    use step_io::ir::shape_rep::{PlainRepr, Representation};
    use step_io::ir::visualization::{
        PresentationStyleAssignment, PresentationStyleByContext, StyleContext, VisualizationPool,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "ctx".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    let viz = model
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.presentation_style_assignments.push(
        PresentationStyleAssignment::PresentationStyleByContext(PresentationStyleByContext {
            styles: vec![],
            style_context: StyleContext::Representation(rep),
        }),
    );

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let re_viz = re.visualization.expect("viz");
    let psbc = re_viz
        .presentation_style_assignments
        .iter()
        .find_map(|p| match p {
            PresentationStyleAssignment::PresentationStyleByContext(c) => Some(c),
            PresentationStyleAssignment::Itself(_) => None,
        })
        .expect("psbc round-trips");
    assert!(matches!(
        psbc.style_context,
        StyleContext::Representation(_)
    ));
}

#[test]
fn surface_curve_subtypes_round_trip() {
    // BOUNDED_SURFACE_CURVE + INTERSECTION_CURVE — corpus 0 inst.
    // synthetic IR with surface + curve + associated_geometry → Surface.
    use step_io::ir::geometry::{
        Line3, PCurveOrSurface, Plane3, PreferredSurfaceCurveRepresentation, Surface, SurfaceCurve,
        SurfaceCurveData,
    };
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    let p1 = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let d1 = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let curve = model
        .geometry
        .curves
        .push(step_io::ir::geometry::Curve::Line(Line3 {
            point: p1,
            direction: d1,
            magnitude: 1.0,
        }));
    let plane = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position: frame }));
    let body = SurfaceCurveData {
        name: "sc".into(),
        curve_3d: curve,
        associated_geometry: vec![PCurveOrSurface::Surface(plane)],
        master_representation: PreferredSurfaceCurveRepresentation::Curve3d,
    };
    model
        .geometry
        .surface_curves
        .push(SurfaceCurve::BoundedSurfaceCurve(body.clone()));
    model
        .geometry
        .surface_curves
        .push(SurfaceCurve::IntersectionCurve(SurfaceCurveData {
            name: "ic".into(),
            ..body
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let mut iter = re.geometry.surface_curves.iter();
    let SurfaceCurve::BoundedSurfaceCurve(bsc) = iter.next().expect("bsc") else {
        panic!("expected BoundedSurfaceCurve first");
    };
    assert_eq!(bsc.name, "sc");
    let SurfaceCurve::IntersectionCurve(ic) = iter.next().expect("ic") else {
        panic!("expected IntersectionCurve second");
    };
    assert_eq!(ic.name, "ic");
}

#[test]
fn curve_bounded_surface_round_trip() {
    // CURVE_BOUNDED_SURFACE — bounded_surface SUBTYPE. corpus 0 inst —
    // synthetic IR only. references a basis surface + a generic Curve
    // (boundary_curve is not yet modelled in step-io).
    use step_io::ir::geometry::{CurveBoundedSurface, Line3, Plane3, Surface};
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    let plane = model
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position: frame }));
    let p1 = model.geometry.points.push(Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let d1 = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let boundary = model
        .geometry
        .curves
        .push(step_io::ir::geometry::Curve::Line(Line3 {
            point: p1,
            direction: d1,
            magnitude: 1.0,
        }));
    model
        .geometry
        .surfaces
        .push(Surface::CurveBounded(CurveBoundedSurface {
            name: "cbs".into(),
            basis_surface: plane,
            boundaries: vec![boundary],
            implicit_outer: true,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let cbs = re
        .geometry
        .surfaces
        .iter()
        .find_map(|s| match s {
            Surface::CurveBounded(c) => Some(c),
            _ => None,
        })
        .expect("cbs round-trips");
    assert_eq!(cbs.name, "cbs");
    assert!(cbs.implicit_outer);
    assert_eq!(cbs.boundaries.len(), 1);
}

#[test]
fn bounded_pcurve_round_trip() {
    // BOUNDED_PCURVE — pcurve SUBTYPE. Orphan. corpus 0 inst — synthetic
    // IR only. references a surface + a (definitional_)representation.
    use step_io::ir::geometry::{BoundedPCurve, ParameterSpaceCurve};
    use step_io::ir::shape_rep::{PlainRepr, Representation};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    let plane = model
        .geometry
        .surfaces
        .push(step_io::ir::geometry::Surface::Plane(
            step_io::ir::geometry::Plane3 { position: frame },
        ));
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "defrepr".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    model
        .geometry
        .parameter_space_curves
        .push(ParameterSpaceCurve::BoundedPCurve(BoundedPCurve {
            name: "bpc".into(),
            basis_surface: plane,
            reference_to_curve: rep,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let psc = re
        .geometry
        .parameter_space_curves
        .iter()
        .next()
        .expect("bpc round-trips");
    let ParameterSpaceCurve::BoundedPCurve(b) = psc;
    assert_eq!(b.name, "bpc");
}

#[test]
fn circular_area_round_trip() {
    // CIRCULAR_AREA — primitive_2d SUBTYPE. Orphan in step-io.
    use step_io::ir::geometry::CircularArea;
    let mut model = empty_model();
    let centre = model.geometry.points.push(Point3 {
        x: 1.0,
        y: 2.0,
        z: 0.0,
    });
    model.geometry.circular_areas.push(CircularArea {
        name: "testarea".into(),
        centre,
        radius: 2.0,
    });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let ca = re
        .geometry
        .circular_areas
        .iter()
        .next()
        .expect("cri round-trips");
    assert_eq!(ca.name, "testarea");
    assert!((ca.radius - 2.0).abs() < f64::EPSILON);
}

#[test]
fn compound_representation_item_round_trip() {
    // COMPOUND_REPRESENTATION_ITEM — typed wrapper carrying a child
    // DESCRIPTIVE_REPRESENTATION_ITEM (the dominant fixture pattern).
    use step_io::ir::shape_rep::{
        CompoundItem, CompoundItemElement, CompoundItemKind, CompoundRepresentationItem,
        DescriptiveItem,
    };
    let mut model = empty_model();
    model
        .compound_representation_items
        .push(CompoundRepresentationItem {
            name: "dimensional note".into(),
            item_element: CompoundItemElement {
                kind: CompoundItemKind::Set,
                items: vec![CompoundItem::Descriptive(DescriptiveItem {
                    name: "dimensional note".into(),
                    description: "controlled radius".into(),
                })],
            },
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let cri = re
        .compound_representation_items
        .iter()
        .next()
        .expect("cri round-trips");
    assert_eq!(cri.name, "dimensional note");
    assert_eq!(cri.item_element.kind, CompoundItemKind::Set);
    assert_eq!(cri.item_element.items.len(), 1);
    let CompoundItem::Descriptive(d) = &cri.item_element.items[0] else {
        panic!("expected Descriptive variant");
    };
    assert_eq!(d.description, "controlled radius");
}

#[test]
fn characterized_object_simple_emit() {
    // CharacterizedObject::Itself writes as simple
    // `CHARACTERIZED_OBJECT(name, $)`. Reader rejects this form (it
    // only accepts the complex MI form) — this test checks one-way
    // write only.
    use step_io::ir::shape_rep::{CharacterizedObject, CharacterizedObjectData};
    let mut model = empty_model();
    model
        .characterized_objects
        .push(CharacterizedObject::Itself(CharacterizedObjectData {
            name: "Back".into(),
            description: None,
        }));

    let text = model.write_to_string().expect("write");
    assert!(text.contains("CHARACTERIZED_OBJECT('Back',$)"));
}

#[test]
fn srwp_round_trip() {
    // SHAPE_REPRESENTATION_WITH_PARAMETERS — representation SUBTYPE
    // with partial-narrow items SELECT (Direction / Placement /
    // Descriptive only).
    use step_io::ir::shape_rep::{
        DescriptiveItem, Representation, ShapeRepresentationWithParameters, SrwpItem,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    let dir = model.geometry.directions.push(Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    model
        .representations
        .push(Representation::ShapeRepresentationWithParameters(
            ShapeRepresentationWithParameters {
                name: "srwp".into(),
                items: vec![
                    SrwpItem::Placement(frame),
                    SrwpItem::Direction(dir),
                    SrwpItem::Descriptive(DescriptiveItem {
                        name: "param".into(),
                        description: "v".into(),
                    }),
                ],
                context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let srwp = re
        .representations
        .iter()
        .find_map(|r| match r {
            Representation::ShapeRepresentationWithParameters(s) => Some(s),
            _ => None,
        })
        .expect("srwp round-trips");
    assert_eq!(srwp.name, "srwp");
    assert_eq!(srwp.items.len(), 3);
}

#[test]
fn iiru_round_trip() {
    // ITEM_IDENTIFIED_REPRESENTATION_USAGE — concrete base entity with
    // 5 attrs. definition resolves to ShapeAspect; identified_item to
    // a typed SET_REPRESENTATION_ITEM wrapper.
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{
        CompoundItemKind, IiruDefinition, IiruIdentifiedItem, ItemIdentifiedRepresentationUsage,
        PlainRepr, Representation,
    };
    let (mut model, sa, _, _, _) = shape_aspect_relationship_fixture();
    let frame = model.geometry.identity_placement();
    let rep = model.representations.push(Representation::Plain(PlainRepr {
        name: "used".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(
            step_io::ir::UnitContextId(0),
        )),
        frame: None,
    }));
    model
        .item_identified_representation_usages
        .push(ItemIdentifiedRepresentationUsage {
            name: "iiru".into(),
            description: Some("GDT".into()),
            definition: IiruDefinition::ShapeAspect(sa),
            used_representation: rep,
            identified_item: IiruIdentifiedItem::Compound {
                kind: CompoundItemKind::Set,
                items: vec![RepresentationItemRef::Placement3d(frame)],
            },
        });

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let iiru = re
        .item_identified_representation_usages
        .iter()
        .next()
        .expect("iiru round-trips");
    assert_eq!(iiru.name, "iiru");
    assert_eq!(iiru.description.as_deref(), Some("GDT"));
    assert!(matches!(iiru.definition, IiruDefinition::ShapeAspect(_)));
}

#[test]
fn mddr_round_trip() {
    // MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP — pairs two
    // representations (DM | MDGPR | SR per mddr_select).
    use step_io::ir::shape_rep::{
        MechanicalDesignAndDraughtingRelationship, PlainRepr, Representation,
        RepresentationRelationship,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let sr1 = model.representations.push(Representation::Plain(PlainRepr {
        name: "sr1".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    let sr2 = model.representations.push(Representation::Plain(PlainRepr {
        name: "sr2".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    model.representation_relationships.push(
        RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(
            MechanicalDesignAndDraughtingRelationship {
                name: "mddr".into(),
                description: "test".into(),
                rep_1: sr1,
                rep_2: sr2,
            },
        ),
    );

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let mddr = re
        .representation_relationships
        .iter()
        .find_map(|r| match r {
            RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(m) => Some(m),
            RepresentationRelationship::ConstructiveGeometryRepresentationRelationship(_)
            | RepresentationRelationship::ShapeRepresentationRelationship(_) => None,
        })
        .expect("mddr round-trips");
    assert_eq!(mddr.name, "mddr");
    assert_eq!(mddr.description, "test");
}

#[test]
fn cgr_relationship_round_trip() {
    // CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP — pairs an SR
    // (rep_1) with a CGR (rep_2). Exercises the new
    // representation_relationships arena + delayed emit.
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{
        ConstructiveGeometryRepr, ConstructiveGeometryRepresentationRelationship, PlainRepr,
        Representation, RepresentationRelationship,
    };
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    let sr = model.representations.push(Representation::Plain(PlainRepr {
        name: "sr".into(),
        context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
        frame: None,
    }));
    let cgr = model
        .representations
        .push(Representation::ConstructiveGeometry(
            ConstructiveGeometryRepr {
                name: "cgr".into(),
                items: vec![RepresentationItemRef::Placement3d(frame)],
                context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
            },
        ));
    model.representation_relationships.push(
        RepresentationRelationship::ConstructiveGeometryRepresentationRelationship(
            ConstructiveGeometryRepresentationRelationship {
                name: "supplemental geometry".into(),
                description: String::new(),
                rep_1: sr,
                rep_2: cgr,
            },
        ),
    );

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let cgrr = re
        .representation_relationships
        .iter()
        .find_map(|r| match r {
            RepresentationRelationship::ConstructiveGeometryRepresentationRelationship(c) => {
                Some(c)
            }
            RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(_)
            | RepresentationRelationship::ShapeRepresentationRelationship(_) => None,
        })
        .expect("cgrr round-trips");
    assert_eq!(cgrr.name, "supplemental geometry");
}

#[test]
fn constructive_geometry_representation_round_trip() {
    // CONSTRUCTIVE_GEOMETRY_REPRESENTATION — representation SUBTYPE with
    // a SET of geometry items. Exercises the delayed-emit pathway
    // through emit_constructive_geometry_representations.
    use step_io::ir::representation_item::RepresentationItemRef;
    use step_io::ir::shape_rep::{ConstructiveGeometryRepr, Representation};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
    let frame = model.geometry.identity_placement();
    model
        .representations
        .push(Representation::ConstructiveGeometry(
            ConstructiveGeometryRepr {
                name: "supplemental geometry".into(),
                items: vec![RepresentationItemRef::Placement3d(frame)],
                context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let cgr = re
        .representations
        .iter()
        .find_map(|r| match r {
            Representation::ConstructiveGeometry(c) => Some(c),
            _ => None,
        })
        .expect("cgr round-trips");
    assert_eq!(cgr.name, "supplemental geometry");
    assert_eq!(cgr.items.len(), 1);
    assert!(matches!(
        cgr.items[0],
        RepresentationItemRef::Placement3d(_)
    ));
}

#[test]
fn tessellated_shape_representation_round_trip() {
    // TESSELLATED_SHAPE_REPRESENTATION — representation SUBTYPE whose
    // items are tessellated_item refs. Exercises the delayed-emit pathway
    // through emit_tessellated_shape_representations.
    use step_io::ir::shape_rep::{Representation, TessellatedShapeRepresentation};
    use step_io::ir::tessellation::{CoordinatesList, TessellatedItem, TessellatedItemRef};
    let mut model = empty_model();
    let ctx = mm_radian_steradian(&mut model);
    let uc = model.units.push(ctx);
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
    model
        .representations
        .push(Representation::TessellatedShapeRepresentation(
            TessellatedShapeRepresentation {
                name: "tsr".into(),
                items: vec![TessellatedItemRef::Item(coords)],
                context: Some(step_io::ir::RepresentationContextRef::Unitful(uc)),
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let tsr = re
        .representations
        .iter()
        .find_map(|r| match r {
            Representation::TessellatedShapeRepresentation(t) => Some(t),
            _ => None,
        })
        .expect("tsr round-trips");
    assert_eq!(tsr.name, "tsr");
    assert_eq!(tsr.items.len(), 1);
    assert!(matches!(tsr.items[0], TessellatedItemRef::Item(_)));
}

#[test]
fn repositioned_tessellated_item_round_trip() {
    // REPOSITIONED_TESSELLATED_ITEM — tessellated_item subtype carrying
    // a per-instance axis2_placement_3d frame.
    use step_io::ir::tessellation::{RepositionedTessellatedItem, TessellatedItem};
    let mut model = empty_model();
    let frame = model.geometry.identity_placement();
    model
        .tessellated_items
        .push(TessellatedItem::RepositionedTessellatedItem(
            RepositionedTessellatedItem {
                name: "rti".into(),
                location: frame,
            },
        ));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let rti = re
        .tessellated_items
        .iter()
        .find_map(|item| match item {
            TessellatedItem::RepositionedTessellatedItem(r) => Some(r),
            _ => None,
        })
        .expect("repositioned tessellated item round-trips");
    assert_eq!(rti.name, "rti");
}

#[test]
fn tessellated_solid_shell_round_trip() {
    // TESSELLATED_SOLID + TESSELLATED_SHELL — items reference structured
    // tessellated items (face / surface set) via TessellatedItemRef.
    use step_io::ir::tessellation::{
        ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList, TessellatedItem,
        TessellatedItemRef, TessellatedShell, TessellatedSolid,
    };
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
    let face = model.tessellated_faces.push(ComplexTriangulatedFace {
        name: "f".into(),
        coordinates: coords,
        pnmax: 3,
        normals: vec![vec![0.0, 0.0, 1.0]],
        geometric_link: None,
        pnindex: vec![1, 2, 3],
        triangle_strips: vec![vec![1, 2, 3]],
        triangle_fans: vec![],
    });
    let ss = model
        .tessellated_surface_sets
        .push(ComplexTriangulatedSurfaceSet {
            name: "ss".into(),
            coordinates: coords,
            pnmax: 3,
            normals: vec![vec![0.0, 0.0, 1.0]],
            pnindex: vec![1, 2, 3],
            triangle_strips: vec![vec![1, 2, 3]],
            triangle_fans: vec![],
        });
    model
        .tessellated_items
        .push(TessellatedItem::TessellatedSolid(TessellatedSolid {
            name: "solid".into(),
            items: vec![
                TessellatedItemRef::Face(face),
                TessellatedItemRef::SurfaceSet(ss),
            ],
            geometric_link: None,
        }));
    model
        .tessellated_items
        .push(TessellatedItem::TessellatedShell(TessellatedShell {
            name: "shell".into(),
            items: vec![TessellatedItemRef::Face(face)],
            topological_link: None,
        }));

    let text = model.write_to_string().expect("write");
    let re = reconvert(&text);
    let solid = re
        .tessellated_items
        .iter()
        .find_map(|i| match i {
            TessellatedItem::TessellatedSolid(s) => Some(s),
            _ => None,
        })
        .expect("solid round-trips");
    assert_eq!(solid.name, "solid");
    assert_eq!(solid.items.len(), 2);
    let shell = re
        .tessellated_items
        .iter()
        .find_map(|i| match i {
            TessellatedItem::TessellatedShell(s) => Some(s),
            _ => None,
        })
        .expect("shell round-trips");
    assert_eq!(shell.name, "shell");
    assert_eq!(shell.items.len(), 1);
}
