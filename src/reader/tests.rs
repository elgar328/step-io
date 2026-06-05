use super::*;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Surface};
use crate::ir::id::{Point2dId, PointId};
use crate::ir::model::StepModel;
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};
use crate::ir::topology::Orientation;
use crate::ir::units::NamedUnit;

/// units-2 helpers: resolve the first `UnitContext`'s `length / plane_angle
/// / solid_angle` `NamedUnitId` to its enum value via the units pool.
fn first_length(model: &StepModel) -> Option<LengthUnit> {
    let ctx = model.units.iter().next()?;
    let pool = model.units_pool.as_ref()?;
    match pool.named_units[ctx.length] {
        NamedUnit::Length(f) => Some(f.unit),
        _ => None,
    }
}
fn first_plane_angle(model: &StepModel) -> Option<AngleUnit> {
    let ctx = model.units.iter().next()?;
    let pool = model.units_pool.as_ref()?;
    match pool.named_units[ctx.plane_angle] {
        NamedUnit::PlaneAngle(f) => Some(f.unit),
        _ => None,
    }
}
fn first_solid_angle(model: &StepModel) -> Option<SolidAngleUnit> {
    let ctx = model.units.iter().next()?;
    let pool = model.units_pool.as_ref()?;
    match pool.named_units[ctx.solid_angle] {
        NamedUnit::SolidAngle(f) => Some(f.unit),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// HEADER extraction
// ---------------------------------------------------------------------------

fn step_with_full_header(fd_impl_level: &str, fn_author: &str, fn_org: &str) -> String {
    format!(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION(('FreeCAD Model'),'{fd_impl_level}');\n\
         FILE_NAME('Open CASCADE Shape Model','2026-04-15T10:25:49',('{fn_author}'),('{fn_org}'),\n\
         'Open CASCADE STEP processor 7.8','FreeCAD','Unknown');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
         ENDSEC;\n\
         DATA;\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n"
    )
}

#[test]
fn reads_file_header_from_fixture_pattern() {
    let src = step_with_full_header("2;1", "Author", "Acme");
    let result = convert_source(&src);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    let h = result.model.header.expect("header preserved");
    assert_eq!(h.description.as_slice(), &["FreeCAD Model".to_string()]);
    assert_eq!(h.implementation_level.as_str(), "2;1");
    assert_eq!(h.name, "Open CASCADE Shape Model");
    assert_eq!(h.time_stamp, "2026-04-15T10:25:49");
    assert_eq!(h.author.as_slice(), &["Author".to_string()]);
    assert_eq!(h.organization.as_slice(), &["Acme".to_string()]);
    assert_eq!(h.preprocessor_version, "Open CASCADE STEP processor 7.8");
    assert_eq!(h.originating_system, "FreeCAD");
    assert_eq!(h.authorization, "Unknown");
}

#[test]
fn reads_file_header_empty_description_list_returns_none() {
    let src = "ISO-10303-21;\n\
               HEADER;\n\
               FILE_DESCRIPTION((),'2;1');\n\
               FILE_NAME('','',(''),(''),'','','');\n\
               FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
               ENDSEC;\n\
               DATA;\n\
               ENDSEC;\n\
               END-ISO-10303-21;\n";
    let result = convert_source(src);
    assert!(result.model.header.is_none());
    assert!(
        result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::UnexpectedEntityForm { detail, .. }
                if detail.contains("description")
        )),
        "{:#?}",
        result.warnings,
    );
}

#[test]
fn reads_file_header_empty_implementation_level_returns_none() {
    let src = step_with_full_header("", "Author", "Acme");
    let result = convert_source(&src);
    assert!(result.model.header.is_none());
    assert!(
        result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::UnexpectedEntityForm { detail, .. }
                if detail.contains("implementation_level")
        )),
        "{:#?}",
        result.warnings,
    );
}

#[test]
fn preserves_ap203_ed2_schema_raw_through_convert() {
    use crate::parser::SchemaClass;
    // Long-form MIM_LF name used by CATIA/Creo/NX/Inventor — distinct from
    // the FreeCAD ed1 short form. Reader must carry the raw text inside
    // StepSchema so the writer can emit it verbatim rather than
    // normalising to ed1.
    let src = "ISO-10303-21;\n\
               HEADER;\n\
               FILE_DESCRIPTION((''), '2;1');\n\
               FILE_NAME('','',(''),(''),'','','');\n\
               FILE_SCHEMA(('AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 3 1 4 }'));\n\
               ENDSEC;\n\
               DATA;\n\
               ENDSEC;\n\
               END-ISO-10303-21;\n";
    let result = convert_source(src);
    assert_eq!(result.model.schema.class(), Some(SchemaClass::Ap203));
    let raw = result.model.schema.raw().expect("raw preserved");
    assert_eq!(
        raw.as_slice(),
        &[
            "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF \
             { 1 0 10303 403 3 1 4 }"
                .to_string(),
        ],
    );
}

/// Parse a minimal STEP source and convert it.
fn convert_source(source: &str) -> ConvertResult {
    let graph = crate::parse(source).expect("parse failed");
    ReaderContext::convert(&graph)
}

fn minimal_step(data_lines: &str) -> String {
    format!(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION((''), '2;1');\n\
         FILE_NAME('test', '2024-01-01', (''), (''), '', '', '');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
         ENDSEC;\n\
         DATA;\n\
         {data_lines}\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n"
    )
}

// --- Empty graph ---

#[test]
fn convert_empty_graph_produces_empty_model() {
    let result = convert_source(&minimal_step(""));
    assert!(result.warnings.is_empty());
    assert!(result.model.geometry.points.is_empty());
    assert!(result.model.geometry.directions.is_empty());
    assert!(result.model.geometry.surfaces.is_empty());
    assert!(result.model.geometry.curves.is_empty());
    assert!(result.model.geometry.vertices.is_empty());
    assert!(result.model.topology.edges.is_empty());
    assert!(result.model.topology.wires.is_empty());
    assert!(result.model.topology.faces.is_empty());
    assert!(result.model.topology.shells.is_empty());
    assert!(result.model.topology.solids.is_empty());
}

// --- CARTESIAN_POINT ---

#[test]
fn convert_single_cartesian_point() {
    let result = convert_source(&minimal_step("#1 = CARTESIAN_POINT('',(10.,20.,30.));"));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.points.len(), 1);
    let pt = &result.model.geometry.points[PointId(0)];
    assert!((pt.x - 10.0).abs() < f64::EPSILON);
    assert!((pt.y - 20.0).abs() < f64::EPSILON);
    assert!((pt.z - 30.0).abs() < f64::EPSILON);
}

#[test]
fn convert_top_level_2d_point_lands_in_points_2d_arena() {
    // A 2-coord CARTESIAN_POINT at the top level (no enclosing
    // DEFINITIONAL_REPRESENTATION) is classified by coordinate count:
    // the 3D handler skips silently, the 2D handler pushes to
    // `points_2d`. Round-trip preservation depends on this.
    let result = convert_source(&minimal_step("#1 = CARTESIAN_POINT('',(10.,20.));"));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert!(result.model.geometry.points.is_empty());
    assert_eq!(result.model.geometry.points_2d.len(), 1);
    let pt = &result.model.geometry.points_2d[Point2dId(0)];
    assert!((pt.x - 10.0).abs() < f64::EPSILON);
    assert!((pt.y - 20.0).abs() < f64::EPSILON);
}

#[test]
fn convert_malformed_cartesian_point_coord_count_emits_warning() {
    // STEP allows only 2- or 3-coord CARTESIAN_POINT. Anything else is
    // genuinely malformed; the 3D handler surfaces it as a warning so
    // it does not vanish silently between the 2D and 3D sister
    // handlers (both of which would otherwise return Ok(()) for an
    // unrecognised arity).
    let result = convert_source(&minimal_step("#1 = CARTESIAN_POINT('',(1.,2.,3.,4.));"));
    assert!(result.model.geometry.points.is_empty());
    assert!(result.model.geometry.points_2d.is_empty());
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::UnexpectedEntityForm { entity_id: 1, detail } if detail.contains("got 4")
    ));
}

#[test]
fn convert_top_level_2d_curve_chain_lands_in_curves_2d_arena() {
    // A full 2D LINE chain (point + direction + vector + line) sits at
    // the top level — no DEFINITIONAL_REPRESENTATION wraps it. Each
    // handler discriminates 2D vs 3D by coordinate count or first
    // cross-reference and pushes into the 2D arenas regardless of
    // pcurve-subtree membership. Without this, an orphan 2D curve
    // produced by a CAD kernel would be dropped on re-read, shifting
    // arena IDs and breaking round-trip equality (see plan #5).
    let src = minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.));\n\
         #2 = DIRECTION('',(1.,0.));\n\
         #3 = VECTOR('',#2,1.);\n\
         #4 = LINE('',#1,#3);",
    );
    let result = convert_source(&src);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(result.model.geometry.points_2d.len(), 1);
    assert_eq!(result.model.geometry.directions_2d.len(), 1);
    assert_eq!(result.model.geometry.curves_2d.len(), 1);
}

// --- DIRECTION ---

#[test]
fn convert_single_direction() {
    let result = convert_source(&minimal_step("#1 = DIRECTION('',(0.,0.,1.));"));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.directions.len(), 1);
    let dir = &result.model.geometry.directions[crate::DirectionId(0)];
    assert!((dir.z - 1.0).abs() < f64::EPSILON);
}

// --- VECTOR ---

#[test]
fn convert_vector() {
    let result = convert_source(&minimal_step(
        "#1 = DIRECTION('',(0.,0.,1.));\n\
         #2 = VECTOR('',#1,2.5);",
    ));
    assert!(result.warnings.is_empty());
    // VECTOR is not in an arena, but verify no warnings.
}

// --- AXIS2_PLACEMENT_3D ---

#[test]
fn convert_placement_with_all_fields() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = AXIS2_PLACEMENT_3D('',#1,#2,#3);",
    ));
    assert!(result.warnings.is_empty());
}

#[test]
fn convert_placement_with_optional_fields() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = AXIS2_PLACEMENT_3D('',#1,$,$);",
    ));
    assert!(result.warnings.is_empty());
}

// --- LINE ---

#[test]
fn convert_line() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = VECTOR('',#2,1.);\n\
         #4 = LINE('',#1,#3);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.curves.len(), 1);
    match &result.model.geometry.curves[crate::CurveId(0)] {
        Curve::Line(line) => {
            assert!((line.magnitude - 1.0).abs() < f64::EPSILON);
        }
        _ => panic!("expected Line"),
    }
}

// --- PLANE ---

#[test]
fn convert_plane() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,1.,0.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #5 = PLANE('',#4);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.surfaces.len(), 1);
}

// --- CYLINDRICAL_SURFACE ---

#[test]
fn convert_cylindrical_surface() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #5 = CYLINDRICAL_SURFACE('',#4,50.);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.surfaces.len(), 1);
    match &result.model.geometry.surfaces[crate::SurfaceId(0)] {
        Surface::Cylinder(cyl) => {
            assert!((cyl.radius - 50.0).abs() < f64::EPSILON);
        }
        _ => panic!("expected Cylinder"),
    }
}

// --- SPHERICAL_SURFACE ---

#[test]
fn convert_spherical_surface() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #5 = SPHERICAL_SURFACE('',#4,10.);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.surfaces.len(), 1);
    match &result.model.geometry.surfaces[crate::SurfaceId(0)] {
        Surface::Sphere(s) => {
            assert!((s.radius - 10.0).abs() < f64::EPSILON);
        }
        _ => panic!("expected Sphere"),
    }
}

// --- CIRCLE ---

#[test]
fn convert_circle() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #5 = CIRCLE('',#4,25.);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.curves.len(), 1);
    match &result.model.geometry.curves[crate::CurveId(0)] {
        Curve::Circle(c) => {
            assert!((c.radius - 25.0).abs() < f64::EPSILON);
        }
        _ => panic!("expected Circle"),
    }
}

// --- Error cases ---

#[test]
fn missing_reference_produces_warning() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = LINE('',#1,#99);",
    ));
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::MissingReference {
            from: 2,
            to: 99,
            field_name: "dir",
        }
    ));
}

#[test]
fn complex_entity_silently_skipped() {
    let result = convert_source(&minimal_step(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
    ));
    assert!(result.warnings.is_empty());
    assert!(result.model.geometry.points.is_empty());
}

// --- Rational B-Spline Curve (complex entity) ---

#[test]
fn convert_rational_bspline_curve_complex() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = ( BOUNDED_CURVE() \
                B_SPLINE_CURVE(2,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.) \
                B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.UNSPECIFIED.) \
                CURVE() \
                GEOMETRIC_REPRESENTATION_ITEM() \
                RATIONAL_B_SPLINE_CURVE((1.,0.707,1.)) \
                REPRESENTATION_ITEM('') );",
    ));
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got {:#?}",
        result.warnings
    );
    assert_eq!(result.model.geometry.curves.len(), 1);
    match &result.model.geometry.curves[crate::CurveId(0)] {
        Curve::Nurbs(n) => {
            assert_eq!(n.degree, 2);
            assert_eq!(n.control_points.len(), 3);
            assert!(n.weights().is_some());
            let ws = n.weights().unwrap();
            assert_eq!(ws.len(), 3);
            assert!((ws[1] - 0.707).abs() < 0.001);
        }
        Curve::Line(_)
        | Curve::Circle(_)
        | Curve::Ellipse(_)
        | Curve::Trimmed(_)
        | Curve::Composite(_)
        | Curve::Polyline(_)
        | Curve::Hyperbola(_)
        | Curve::Parabola(_)
        | Curve::OffsetCurve3d(_) => panic!("expected Nurbs"),
    }
}

#[test]
fn rational_bspline_curve_weight_count_mismatch_warning() {
    // 3 control points but only 2 weights — should produce DimensionMismatch.
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = ( BOUNDED_CURVE() \
                B_SPLINE_CURVE(2,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.) \
                B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.UNSPECIFIED.) \
                CURVE() \
                GEOMETRIC_REPRESENTATION_ITEM() \
                RATIONAL_B_SPLINE_CURVE((1.,0.707)) \
                REPRESENTATION_ITEM('') );",
    ));
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::DimensionMismatch {
            field_name: "weights_data",
            ..
        }
    ));
}

// --- Quasi-uniform B-Spline curve (simple, derived knots) ---

#[test]
fn convert_quasi_uniform_curve_simple() {
    // QUASI_UNIFORM_CURVE has 6 attrs (no knot list — derived).
    // degree=3, 4 control points → mults=[4,4], knots=[0.,1.].
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = CARTESIAN_POINT('',(3.,0.,0.));\n\
         #5 = QUASI_UNIFORM_CURVE('',3,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.F.);",
    ));
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got {:#?}",
        result.warnings
    );
    assert_eq!(result.model.geometry.curves.len(), 1);
    match &result.model.geometry.curves[crate::CurveId(0)] {
        Curve::Nurbs(n) => {
            assert_eq!(n.degree, 3);
            assert_eq!(n.control_points.len(), 4);
            assert!(n.weights().is_none());
            assert_eq!(n.knot_multiplicities, vec![4, 4]);
            assert_eq!(n.knots, vec![0.0, 1.0]);
        }
        _ => panic!("expected Nurbs"),
    }
}

#[test]
fn convert_rational_quasi_uniform_curve_complex() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = CARTESIAN_POINT('',(3.,0.,0.));\n\
         #5 = ( BOUNDED_CURVE() \
                B_SPLINE_CURVE(3,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.F.) \
                CURVE() \
                GEOMETRIC_REPRESENTATION_ITEM() \
                QUASI_UNIFORM_CURVE() \
                RATIONAL_B_SPLINE_CURVE((1.,0.5,0.5,1.)) \
                REPRESENTATION_ITEM('') );",
    ));
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got {:#?}",
        result.warnings
    );
    assert_eq!(result.model.geometry.curves.len(), 1);
    match &result.model.geometry.curves[crate::CurveId(0)] {
        Curve::Nurbs(n) => {
            assert_eq!(n.degree, 3);
            assert!(n.weights().is_some());
            assert_eq!(n.knot_multiplicities, vec![4, 4]);
            assert_eq!(n.knots, vec![0.0, 1.0]);
        }
        _ => panic!("expected Nurbs"),
    }
}

// --- Quasi-uniform B-Spline surface (simple, derived knots) ---

#[test]
fn convert_quasi_uniform_surface_simple() {
    // QUASI_UNIFORM_SURFACE has 8 attrs (no knot lists — derived).
    // u_degree=3, v_degree=1, 4x2 control points (matches fixture shape).
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = CARTESIAN_POINT('',(3.,0.,0.));\n\
         #5 = CARTESIAN_POINT('',(0.,1.,0.));\n\
         #6 = CARTESIAN_POINT('',(1.,1.,0.));\n\
         #7 = CARTESIAN_POINT('',(2.,1.,0.));\n\
         #8 = CARTESIAN_POINT('',(3.,1.,0.));\n\
         #9 = QUASI_UNIFORM_SURFACE('',3,1,((#1,#5),(#2,#6),(#3,#7),(#4,#8)),.UNSPECIFIED.,.F.,.F.,.U.);",
    ));
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got {:#?}",
        result.warnings
    );
    assert_eq!(result.model.geometry.surfaces.len(), 1);
    match &result.model.geometry.surfaces[crate::SurfaceId(0)] {
        Surface::Nurbs(s) => {
            assert_eq!(s.u_degree, 3);
            assert_eq!(s.v_degree, 1);
            assert_eq!(s.control_points.len(), 4);
            assert_eq!(s.control_points[0].len(), 2);
            assert!(s.weights().is_none());
            assert_eq!(s.u_knot_multiplicities, vec![4, 4]);
            assert_eq!(s.v_knot_multiplicities, vec![2, 2]);
            assert_eq!(s.u_knots, vec![0.0, 1.0]);
            assert_eq!(s.v_knots, vec![0.0, 1.0]);
        }
        _ => panic!("expected Nurbs"),
    }
}

#[test]
fn convert_rational_quasi_uniform_surface_complex() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = CARTESIAN_POINT('',(2.,0.,0.));\n\
         #4 = CARTESIAN_POINT('',(3.,0.,0.));\n\
         #5 = CARTESIAN_POINT('',(0.,1.,0.));\n\
         #6 = CARTESIAN_POINT('',(1.,1.,0.));\n\
         #7 = CARTESIAN_POINT('',(2.,1.,0.));\n\
         #8 = CARTESIAN_POINT('',(3.,1.,0.));\n\
         #9 = ( BOUNDED_SURFACE() \
                B_SPLINE_SURFACE(3,1,((#1,#5),(#2,#6),(#3,#7),(#4,#8)),.UNSPECIFIED.,.F.,.F.,.U.) \
                GEOMETRIC_REPRESENTATION_ITEM() \
                QUASI_UNIFORM_SURFACE() \
                RATIONAL_B_SPLINE_SURFACE(((1.,1.),(0.5,0.5),(0.5,0.5),(1.,1.))) \
                REPRESENTATION_ITEM('') \
                SURFACE() );",
    ));
    assert!(
        result.warnings.is_empty(),
        "expected no warnings, got {:#?}",
        result.warnings
    );
    assert_eq!(result.model.geometry.surfaces.len(), 1);
    match &result.model.geometry.surfaces[crate::SurfaceId(0)] {
        Surface::Nurbs(s) => {
            assert_eq!(s.u_degree, 3);
            assert_eq!(s.v_degree, 1);
            assert!(s.weights().is_some());
            assert_eq!(s.u_knot_multiplicities, vec![4, 4]);
            assert_eq!(s.v_knot_multiplicities, vec![2, 2]);
        }
        _ => panic!("expected Nurbs"),
    }
}

// --- Topology: VERTEX_POINT ---

#[test]
fn convert_vertex_point() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(1.,2.,3.));\n\
         #2 = VERTEX_POINT('',#1);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.vertices.len(), 1);
    let v = &result.model.geometry.vertices[crate::VertexId(0)];
    assert_eq!(v.point, PointId(0));
}

// --- Topology: EDGE_CURVE ---

#[test]
fn convert_edge_curve() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = VECTOR('',#3,1.);\n\
         #5 = LINE('',#1,#4);\n\
         #6 = VERTEX_POINT('',#1);\n\
         #7 = VERTEX_POINT('',#2);\n\
         #8 = EDGE_CURVE('',#6,#7,#5,.T.);",
    ));
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.topology.edges.len(), 1);
    let e = &result.model.topology.edges[crate::EdgeId(0)];
    assert_eq!(e.orientation, Orientation::Forward);
    assert!((e.trim.0).abs() < f64::EPSILON);
}

#[test]
fn convert_edge_curve_reversed() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(1.,0.,0.));\n\
         #3 = VECTOR('',#2,1.);\n\
         #4 = LINE('',#1,#3);\n\
         #5 = VERTEX_POINT('',#1);\n\
         #6 = EDGE_CURVE('',#5,#5,#4,.F.);",
    ));
    assert!(result.warnings.is_empty());
    let e = &result.model.topology.edges[crate::EdgeId(0)];
    assert_eq!(e.orientation, Orientation::Reversed);
}

// --- Topology: ORIENTED_EDGE ---

#[test]
fn convert_oriented_edge_with_derived_attrs() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(1.,0.,0.));\n\
         #3 = VECTOR('',#2,1.);\n\
         #4 = LINE('',#1,#3);\n\
         #5 = VERTEX_POINT('',#1);\n\
         #6 = EDGE_CURVE('',#5,#5,#4,.T.);\n\
         #7 = ORIENTED_EDGE('',*,*,#6,.F.);",
    ));
    assert!(result.warnings.is_empty());
}

// --- Topology: full chain to MANIFOLD_SOLID_BREP ---

fn full_topology_step() -> String {
    minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = VECTOR('',#2,1.);\n\
         #5 = LINE('',#1,#4);\n\
         #6 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #7 = PLANE('',#6);\n\
         #10 = VERTEX_POINT('',#1);\n\
         #11 = EDGE_CURVE('',#10,#10,#5,.T.);\n\
         #12 = ORIENTED_EDGE('',*,*,#11,.T.);\n\
         #13 = EDGE_LOOP('',(#12));\n\
         #14 = FACE_BOUND('',#13,.T.);\n\
         #15 = ADVANCED_FACE('',(#14),#7,.T.);\n\
         #16 = CLOSED_SHELL('',(#15));\n\
         #17 = MANIFOLD_SOLID_BREP('Test',#16);",
    )
}

#[test]
fn convert_full_topology_chain() {
    let result = convert_source(&full_topology_step());
    assert!(result.warnings.is_empty());
    assert_eq!(result.model.geometry.vertices.len(), 1);
    assert_eq!(result.model.topology.edges.len(), 1);
    assert_eq!(result.model.topology.wires.len(), 1);
    assert_eq!(result.model.topology.faces.len(), 1);
    assert_eq!(result.model.topology.shells.len(), 1);
    assert_eq!(result.model.topology.solids.len(), 1);
}

#[test]
fn convert_solid_name_preserved() {
    let result = convert_source(&full_topology_step());
    let solid = &result.model.topology.solids[crate::SolidId(0)];
    assert_eq!(solid.name(), Some("Test"));
}

#[test]
fn convert_solid_empty_name_is_none() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = VECTOR('',#2,1.);\n\
         #5 = LINE('',#1,#4);\n\
         #6 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #7 = PLANE('',#6);\n\
         #10 = VERTEX_POINT('',#1);\n\
         #11 = EDGE_CURVE('',#10,#10,#5,.T.);\n\
         #12 = ORIENTED_EDGE('',*,*,#11,.T.);\n\
         #13 = EDGE_LOOP('',(#12));\n\
         #14 = FACE_BOUND('',#13,.T.);\n\
         #15 = ADVANCED_FACE('',(#14),#7,.T.);\n\
         #16 = CLOSED_SHELL('',(#15));\n\
         #17 = MANIFOLD_SOLID_BREP('',#16);",
    ));
    let solid = &result.model.topology.solids[crate::SolidId(0)];
    assert_eq!(solid.name(), None);
}

#[test]
fn convert_face_bound_is_outer_false() {
    let result = convert_source(&full_topology_step());
    let wire = &result.model.topology.wires[crate::WireId(0)];
    assert!(matches!(wire, crate::ir::Wire::FaceBound(_)));
}

#[test]
fn convert_face_outer_bound_sets_is_outer() {
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = DIRECTION('',(0.,0.,1.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = VECTOR('',#2,1.);\n\
         #5 = LINE('',#1,#4);\n\
         #6 = AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
         #7 = PLANE('',#6);\n\
         #10 = VERTEX_POINT('',#1);\n\
         #11 = EDGE_CURVE('',#10,#10,#5,.T.);\n\
         #12 = ORIENTED_EDGE('',*,*,#11,.T.);\n\
         #13 = EDGE_LOOP('',(#12));\n\
         #14 = FACE_OUTER_BOUND('',#13,.T.);\n\
         #15 = ADVANCED_FACE('',(#14),#7,.T.);\n\
         #16 = CLOSED_SHELL('',(#15));\n\
         #17 = MANIFOLD_SOLID_BREP('',#16);",
    ));
    let wire = &result.model.topology.wires[crate::WireId(0)];
    assert!(matches!(wire, crate::ir::Wire::FaceOuterBound(_)));
}

// ---------------------------------------------------------------------------
// Unit context
// ---------------------------------------------------------------------------

/// Build the canonical mm / radian / steradian unit entities plus the
/// `GLOBAL_UNIT_ASSIGNED_CONTEXT` that ties them together. `length_prefix`
/// is injected into the length unit's `SI_UNIT` so tests can swap it.
fn unit_data(length_prefix: &str) -> String {
    format!(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT({length_prefix},.METRE.) );\n\
         #2 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #3 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #4 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3))\n\
         \t\tREPRESENTATION_CONTEXT('','') );"
    )
}

#[test]
fn unit_millimetre_radian_steradian() {
    let result = convert_source(&minimal_step(&unit_data(".MILLI.")));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Millimetre));
    assert_eq!(first_plane_angle(&result.model), Some(AngleUnit::Radian));
    assert_eq!(
        first_solid_angle(&result.model),
        Some(SolidAngleUnit::Steradian)
    );
    let ctx = result.model.units.iter().next().expect("ctx");
    assert!(ctx.length_uncertainty.is_none());
    assert!(ctx.plane_angle_uncertainty.is_none());
    assert!(ctx.solid_angle_uncertainty.is_none());
}

#[test]
fn unit_centimetre_mapping() {
    let result = convert_source(&minimal_step(&unit_data(".CENTI.")));
    assert!(result.warnings.is_empty());
    assert_eq!(first_length(&result.model), Some(LengthUnit::Centimetre),);
}

#[test]
fn unit_plain_metre_mapping() {
    let result = convert_source(&minimal_step(&unit_data("$")));
    assert!(result.warnings.is_empty());
    assert_eq!(first_length(&result.model), Some(LengthUnit::Metre),);
}

#[test]
fn unit_unsupported_prefix_produces_warning_and_default_length() {
    let result = convert_source(&minimal_step(&unit_data(".KILO.")));
    // Two warnings expected: (1) the leaf flagged .KILO. as unsupported,
    // (2) the global context filled the missing length slot with the SI default.
    assert_eq!(result.warnings.len(), 2, "{:#?}", result.warnings);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::UnexpectedEntityForm { detail, .. }
            if detail.contains("unsupported SI length unit")
    ));
    assert!(matches!(
        &result.warnings[1],
        ConvertError::UnexpectedEntityForm { detail, .. }
            if detail.contains("incomplete unit context")
    ));
    // GUAC fallback pushed a unit context with SI defaults so downstream
    // PRODUCT chain emit isn't silently lost.
    let unit = result
        .model
        .units
        .iter()
        .next()
        .expect("fallback context pushed");
    let _ = unit;
    assert_eq!(first_length(&result.model), Some(LengthUnit::Millimetre));
}

// ---------------------------------------------------------------------------
// CONVERSION_BASED_UNIT (inch / foot / degree + CBU-wrapped metric)
// ---------------------------------------------------------------------------

/// Build a full unit context that mixes one CBU length unit, one SI angle,
/// and SI steradian. `length_cbu_block` contains the CBU chain for the
/// length unit; the caller produces entities #1..#4 and the chain's final
/// entity id goes into the GUAC.
fn cbu_unit_step(length_cbu_block: &str, length_unit_ref: u32) -> String {
    minimal_step(&format!(
        "{length_cbu_block}\n\
         #20 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #21 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #22 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#{length_unit_ref},#20,#21))\n\
         \t\tREPRESENTATION_CONTEXT('','') );"
    ))
}

#[test]
fn reads_inch_milli_untyped_convention() {
    // Fillet_box (FreeCAD) convention: MILLI-METRE base + untyped 25.4.
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(25.4,#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('INCH',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Inch),);
}

#[test]
fn reads_inch_centi_typed_convention() {
    // dm1-id-214 (STEPcode) convention: CENTI-METRE base + typed LENGTH_MEASURE(2.54).
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.CENTI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(2.54),#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('INCH',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Inch),);
}

#[test]
fn reads_inch_lowercase_name() {
    // Observed in AP242 NIST fixtures: some files emit 'inch' in lowercase.
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(25.4,#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('inch',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Inch),);
}

#[test]
fn reads_foot_standard() {
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(304.8,#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('FOOT',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Foot),);
}

#[test]
fn reads_degree_uppercase() {
    // Degree CBU wraps the SI radian base; assertion checks plane_angle only.
    let step = minimal_step(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #10 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #11 = DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
         #12 = PLANE_ANGLE_MEASURE_WITH_UNIT(0.017453292519943295,#10);\n\
         #13 = ( CONVERSION_BASED_UNIT('DEGREE',#12) NAMED_UNIT(#11) PLANE_ANGLE_UNIT() );\n\
         #14 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #15 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#13,#14))\n\
         \t\tREPRESENTATION_CONTEXT('','') );",
    );
    let result = convert_source(&step);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_plane_angle(&result.model), Some(AngleUnit::Degree),);
}

#[test]
fn reads_degree_lowercase() {
    let step = minimal_step(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #10 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #11 = DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
         #12 = PLANE_ANGLE_MEASURE_WITH_UNIT(0.017453292519943295,#10);\n\
         #13 = ( CONVERSION_BASED_UNIT('degree',#12) NAMED_UNIT(#11) PLANE_ANGLE_UNIT() );\n\
         #14 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #15 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#13,#14))\n\
         \t\tREPRESENTATION_CONTEXT('','') );",
    );
    let result = convert_source(&step);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_plane_angle(&result.model), Some(AngleUnit::Degree),);
}

#[test]
fn reads_millimetre_cbu_wrap() {
    // AP242 NIST files sometimes wrap an SI MILLIMETRE in a CONVERSION_BASED_UNIT
    // named 'MILLIMETRE'. Reader must still resolve it to LengthUnit::Millimetre.
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(1.,#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('MILLIMETRE',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_length(&result.model), Some(LengthUnit::Millimetre),);
}

#[test]
fn reads_degrees_plural_cbu_name() {
    // Some exporters (Rhino + ST-Developer observed in the wild) emit
    // `CONVERSION_BASED_UNIT('DEGREES', ...)` — plural form. Reader should
    // accept it as AngleUnit::Degree just like the singular `'DEGREE'`.
    let source = minimal_step(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
         #3 = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.01745329252),#4);\n\
         #4 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #5 = ( CONVERSION_BASED_UNIT('DEGREES',#3) NAMED_UNIT(#2) PLANE_ANGLE_UNIT() );\n\
         #6 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #7 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#5,#6))\n\
         \t\tREPRESENTATION_CONTEXT('','') );",
    );
    let result = convert_source(&source);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(first_plane_angle(&result.model), Some(AngleUnit::Degree));
}

#[test]
fn reads_unrecognized_cbu_name_warns() {
    // An unknown name on a LENGTH_UNIT-flavoured CBU: reader emits an
    // UnexpectedEntityForm warning AND a second warning when the GUAC falls
    // back to SI defaults to keep the unit context alive (so the PRODUCT
    // chain doesn't silently disappear downstream).
    let block = "\
         #1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
         #3 = LENGTH_MEASURE_WITH_UNIT(1.,#1);\n\
         #4 = ( CONVERSION_BASED_UNIT('CUBIT',#3) LENGTH_UNIT() NAMED_UNIT(#2) );";
    let result = convert_source(&cbu_unit_step(block, 4));
    assert!(
        result.warnings.iter().any(
            |w| matches!(w, ConvertError::UnexpectedEntityForm { detail, .. }
                if detail.contains("CONVERSION_BASED_UNIT")
                && detail.contains("CUBIT"))
        ),
        "{:#?}",
        result.warnings,
    );
    assert!(
        result.warnings.iter().any(
            |w| matches!(w, ConvertError::UnexpectedEntityForm { detail, .. }
                if detail.contains("incomplete unit context"))
        ),
        "{:#?}",
        result.warnings,
    );
    let unit = result
        .model
        .units
        .iter()
        .next()
        .expect("fallback context pushed");
    let _ = unit;
    assert_eq!(first_length(&result.model), Some(LengthUnit::Millimetre));
}

#[test]
fn garbage_angle_cbu_recovered_as_degree_by_factor() {
    // Anonymised fixtures occasionally carry a CONVERSION_BASED_UNIT with a
    // nonsense angle name (the `interior-vehicle-hvac` grabcad fixtures
    // replaced every string with the placeholder 'MIAU' before upload). The
    // name is unrecognisable, but the conversion factor (0.01745329252 = π/180,
    // relative to the only SI plane-angle base radian) unambiguously identifies
    // the unit as a degree. The reader recovers it by factor and records a
    // `NonStandardInput` normalization (not a defect) — so the plane-angle slot
    // is filled, no "incomplete unit context" fallback fires, and the unit
    // round-trips as a standard 'DEGREE'.
    let source = minimal_step(
        "#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #2 = DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
         #3 = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.01745329252),#4);\n\
         #4 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #5 = ( CONVERSION_BASED_UNIT('NONSENSE',#3) NAMED_UNIT(#2) PLANE_ANGLE_UNIT() );\n\
         #6 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #7 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#5,#6))\n\
         \t\tREPRESENTATION_CONTEXT('','') );",
    );
    let result = convert_source(&source);
    // The non-standard name is surfaced as a normalization, not a defect.
    assert!(
        result.warnings.iter().any(
            |w| matches!(w, ConvertError::NonStandardInput { field, normalized_to, .. }
            if field.contains("CONVERSION_BASED_UNIT.name") && field.contains("NONSENSE")
                && normalized_to == "DEGREE")
        ),
        "{:#?}",
        result.warnings,
    );
    // No "unsupported name" defect and no incomplete-context fallback: the unit
    // was recovered, so the plane-angle slot is filled.
    assert!(
        !result.warnings.iter().any(
            |w| matches!(w, ConvertError::UnexpectedEntityForm { detail, .. }
            if detail.contains("incomplete unit context")
                || (detail.contains("CONVERSION_BASED_UNIT") && detail.contains("NONSENSE")))
        ),
        "{:#?}",
        result.warnings,
    );
    // The recovered unit is a degree (not the old radian fallback).
    assert_eq!(first_plane_angle(&result.model), Some(AngleUnit::Degree));
    assert_eq!(first_length(&result.model), Some(LengthUnit::Millimetre));
}

#[test]
fn unit_no_global_context_is_silent() {
    // A minimal STEP file with no GLOBAL_UNIT_ASSIGNED_CONTEXT at all — a
    // lone CARTESIAN_POINT. Conversion should leave `units` as None and
    // emit no warnings (silent skip).
    let result = convert_source(&minimal_step("#1 = CARTESIAN_POINT('',(0.,0.,0.));"));
    assert!(result.warnings.is_empty());
    assert!(result.model.units.is_empty());
}

#[test]
fn empty_prrpc_and_its_relationship_dropped_as_normalization() {
    // `PRODUCT_RELATED_PRODUCT_CATEGORY.products` is SET[1:?] in every schema;
    // some CATIA / Autodesk exports emit an empty `()`. The reader drops the
    // empty PRRPC (and the PRODUCT_CATEGORY_RELATIONSHIP that references it) as
    // a NonStandardInput normalization, not a MissingReference defect. A
    // PRRPC with real products is preserved.
    let result = convert_source(&minimal_step(
        "#1 = PRODUCT('P','P',' ',(#2));\n\
         #2 = PRODUCT_CONTEXT('',#3,'mechanical');\n\
         #3 = APPLICATION_CONTEXT('core');\n\
         #4 = PRODUCT_CATEGORY('part','');\n\
         #5 = PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,());\n\
         #6 = PRODUCT_CATEGORY_RELATIONSHIP('','',#4,#5);\n\
         #7 = PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#1));",
    ));
    // Both `$`-empty entities surface as normalizations; no defect.
    let norms = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::NonStandardInput { normalized_to, .. }
                if normalized_to.starts_with("dropped"))
        })
        .count();
    assert_eq!(norms, 2, "{:#?}", result.warnings);
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| matches!(w, ConvertError::MissingReference { .. })),
        "{:#?}",
        result.warnings
    );
    // The empty PRRPC + its relationship are not in the IR; the real PRRPC is.
    let asm = result.model.assembly.as_ref().expect("assembly");
    assert!(
        asm.product_category_relationships.is_empty(),
        "the relationship to the empty PRRPC is dropped"
    );
    let prpc_count = asm
        .product_categories
        .iter()
        .filter(|pc| {
            matches!(
                pc,
                crate::ir::assembly::ProductCategory::ProductRelatedProductCategory(_)
            )
        })
        .count();
    assert_eq!(prpc_count, 1, "only the non-empty PRRPC survives");
}

#[test]
fn empty_invisibility_dropped_as_normalization() {
    // `INVISIBILITY.invisible_items` is SET[1:?] in every schema; some grabcad
    // exports emit an empty `()` (hides nothing). The reader drops it as a
    // NonStandardInput normalization, not a MissingReference defect. INVISIBILITY
    // is a leaf, so there is no cascade.
    let result = convert_source(&minimal_step("#1 = INVISIBILITY(());"));
    let norms = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::NonStandardInput { field, normalized_to, .. }
                if field == "INVISIBILITY" && normalized_to.starts_with("dropped"))
        })
        .count();
    assert_eq!(norms, 1, "{:#?}", result.warnings);
    assert!(
        !result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::MissingReference { .. } | ConvertError::UnexpectedEntityForm { .. }
        )),
        "{:#?}",
        result.warnings
    );
    // No empty invisibility entity is materialised.
    assert!(
        result
            .model
            .visualization
            .as_ref()
            .is_none_or(|v| v.invisibilities.is_empty()),
        "the empty INVISIBILITY is not in the IR"
    );
}

#[test]
fn invisibility_with_all_items_unresolved_surfaces_a_warning() {
    // A non-empty INVISIBILITY whose items resolve to no modelled
    // styled_item / representation / draughting_callout (here a CARTESIAN_POINT,
    // outside all three id_maps — the handler treats any non-target ref the same)
    // is dropped, but surfaced as a defect warning rather than silently. These
    // entities already count as missing, so this adds visibility, not loss.
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = INVISIBILITY((#1));",
    ));
    let unresolved = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::UnexpectedEntityForm { detail, .. }
                if detail.contains("INVISIBILITY") && detail.contains("did not resolve"))
        })
        .count();
    assert_eq!(unresolved, 1, "{:#?}", result.warnings);
    // It is not a NonStandardInput normalization (the set was non-empty).
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| matches!(w, ConvertError::NonStandardInput { field, .. }
                if field == "INVISIBILITY")),
        "{:#?}",
        result.warnings
    );
    // No invisibility entity is materialised.
    assert!(
        result
            .model
            .visualization
            .as_ref()
            .is_none_or(|v| v.invisibilities.is_empty()),
        "the unresolved INVISIBILITY is not in the IR"
    );
}

#[test]
fn dangling_person_and_organization_and_cascade_dropped_as_normalization() {
    // `PERSON_AND_ORGANIZATION.the_person` is a required ref; some anonymizers
    // (e.g. the GrabCAD badland-winch / fairlead fixtures) scrub the person and
    // leave a dangling sentinel (#18446744073709551615 = u64::MAX) that points
    // to no defined entity. The reader drops such a P&O — and the
    // CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT / APPROVAL_PERSON_ORGANIZATION
    // that reference it — as NonStandardInput normalizations, not defects. A P&O
    // with real PERSON / ORGANIZATION refs is preserved.
    let result = convert_source(&minimal_step(
        "#1 = PERSON('id','last','first',$,$,$);\n\
         #2 = ORGANIZATION($,'org','');\n\
         #3 = PERSON_AND_ORGANIZATION(#1,#2);\n\
         #4 = PERSON_AND_ORGANIZATION(#18446744073709551615,#2);\n\
         #5 = PERSON_AND_ORGANIZATION_ROLE('creator');\n\
         #6 = CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#4,#5,());\n\
         #7 = APPROVAL_PERSON_ORGANIZATION(#4,#9998,#5);",
    ));
    // The dangling P&O and its two referencing entities each surface as a
    // "dropped" normalization; no defect.
    let dropped: Vec<&str> = result
        .warnings
        .iter()
        .filter_map(|w| match w {
            ConvertError::NonStandardInput {
                field,
                normalized_to,
                ..
            } if normalized_to.starts_with("dropped") => Some(field.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(dropped.len(), 3, "{:#?}", result.warnings);
    assert!(dropped.contains(&"PERSON_AND_ORGANIZATION"));
    assert!(dropped.contains(&"CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT"));
    assert!(dropped.contains(&"APPROVAL_PERSON_ORGANIZATION"));
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| matches!(w, ConvertError::MissingReference { .. })),
        "{:#?}",
        result.warnings
    );
    // Only the P&O with real person + organization refs survives in the IR.
    let plm = result.model.plm.as_ref().expect("plm pool");
    assert_eq!(plm.person_and_organizations.iter().count(), 1);
    assert!(plm.person_and_organization_assignments.iter().count() == 0);
    assert!(plm.approval_person_organizations.iter().count() == 0);
}

#[test]
fn file_name_unset_string_fields_normalized_to_empty() {
    // Part 21 (ISO 10303-21) defines FILE_NAME scalar fields as required
    // STRING; `$` is non-standard (`''` denotes unspecified). Some exporters
    // (e.g. the SO14 sensor / centrifugal-fan grabcad fixtures) emit `$` for
    // originating_system / authorization. The reader normalizes `$` to `''`
    // and keeps the header rather than discarding it.
    let source = "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION((''), '2;1');\n\
         FILE_NAME('n', 't', (''), (''), 'pp', $, $);\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
         ENDSEC;\n\
         DATA;\n\
         #1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n";
    let result = convert_source(source);
    let header = result
        .model
        .header
        .as_ref()
        .expect("header kept, not discarded");
    assert_eq!(header.originating_system, "");
    assert_eq!(header.authorization, "");
    // Both `$` fields surface as normalizations, and no defect warning.
    let norm = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::NonStandardInput { field, .. }
                if field.contains("FILE_NAME") && field.contains("Unset"))
        })
        .count();
    assert_eq!(norm, 2, "{:#?}", result.warnings);
    assert!(
        !result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::AttributeType { .. } | ConvertError::UnexpectedEntityForm { .. }
        )),
        "{:#?}",
        result.warnings
    );

    // Re-read of the written output keeps the header with no new normalization.
    let text = result.model.write_to_string().expect("write");
    let re = convert_source(&text);
    assert!(re.model.header.is_some(), "header survives round-trip");
}

#[test]
fn curve_style_unset_curve_font_round_trips_as_none() {
    // CURVE_STYLE.curve_font is OPTIONAL in AP242 (required in AP203/AP214);
    // Rhino 8 omits it as `$`. The reader preserves the omission as `None`
    // (rather than dropping the whole CURVE_STYLE) and the writer re-emits `$`.
    use crate::ir::visualization::CurveWidth;
    let source = minimal_step(
        "#1 = COLOUR_RGB('',0.,1.,1.);\n\
         #2 = CURVE_STYLE('',$,POSITIVE_LENGTH_MEASURE(0.02),#1);",
    );
    let result = convert_source(&source);
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    let pool = result.model.visualization.as_ref().expect("viz pool");
    assert_eq!(
        pool.curve_styles.len(),
        1,
        "CURVE_STYLE preserved, not dropped on $ curve_font"
    );
    let cs = pool.curve_styles.iter().next().unwrap();
    assert_eq!(cs.curve_font, None, "$ curve_font preserved as None");
    assert!(matches!(
        cs.curve_width,
        CurveWidth::PositiveLengthMeasure(_)
    ));

    // Writer re-emits `$`, and re-reading yields the same `None` (idempotent).
    let text = result.model.write_to_string().expect("write");
    assert!(text.contains("CURVE_STYLE("), "CURVE_STYLE emitted: {text}");
    let re = convert_source(&text);
    let re_cs = re
        .model
        .visualization
        .as_ref()
        .expect("viz pool")
        .curve_styles
        .iter()
        .next()
        .unwrap();
    assert_eq!(re_cs.curve_font, None, "round-trip keeps None");
}

// ---------------------------------------------------------------------------
// PCURVE skip-set + SURFACE_CURVE alias
// ---------------------------------------------------------------------------

#[test]
fn pcurve_subtree_is_silently_skipped() {
    // 3D CARTESIAN_POINT (to be converted) + DEFINITIONAL_REPRESENTATION subtree
    // containing a 2D CARTESIAN_POINT that must be skipped without warning.
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(-0.,0.));\n\
         #3 = DEFINITIONAL_REPRESENTATION('',(#2),#4);\n\
         #4 = ( GEOMETRIC_REPRESENTATION_CONTEXT(2)\n\
                PARAMETRIC_REPRESENTATION_CONTEXT()\n\
                REPRESENTATION_CONTEXT('','') );",
    ));
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
    assert_eq!(
        result.model.geometry.points.len(),
        1,
        "only #1 should survive"
    );
}

#[test]
fn surface_curve_aliases_to_inner_curve_3d() {
    // SURFACE_CURVE wraps a LINE as its curve_3d and references a PCURVE
    // sitting under a DEFINITIONAL_REPRESENTATION subtree. The EDGE_CURVE
    // should resolve to the LINE via the alias; the 2D subtree is skipped.
    let result = convert_source(&minimal_step(
        "#1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2 = CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3 = DIRECTION('',(1.,0.,0.));\n\
         #4 = VECTOR('',#3,1.);\n\
         #5 = LINE('',#1,#4);\n\
         #6 = DIRECTION('',(0.,0.,1.));\n\
         #7 = AXIS2_PLACEMENT_3D('',#1,#6,#3);\n\
         #8 = PLANE('',#7);\n\
         #9 = PCURVE('',#8,#10);\n\
         #10 = DEFINITIONAL_REPRESENTATION('',(#11),#12);\n\
         #11 = CARTESIAN_POINT('',(0.,0.));\n\
         #12 = ( GEOMETRIC_REPRESENTATION_CONTEXT(2)\n\
                 PARAMETRIC_REPRESENTATION_CONTEXT()\n\
                 REPRESENTATION_CONTEXT('','') );\n\
         #13 = SURFACE_CURVE('',#5,(#9),.CURVE_3D.);\n\
         #14 = VERTEX_POINT('',#1);\n\
         #15 = VERTEX_POINT('',#2);\n\
         #16 = EDGE_CURVE('',#14,#15,#13,.T.);",
    ));
    // The synthetic fixture uses a CARTESIAN_POINT as PCURVE.reference_to_curve
    // for brevity, so `resolve_pcurve` legitimately reports it as unresolvable
    // (phase pcurve-pass-order made the previously-silent drop into a warning).
    assert_eq!(
        result.warnings.len(),
        1,
        "expected exactly the PCURVE-unresolved warning, got {:#?}",
        result.warnings
    );
    // Only the 3D LINE should be in the curves pool.
    assert_eq!(result.model.geometry.curves.len(), 1);
    // The edge must point at that LINE via the SURFACE_CURVE alias.
    assert_eq!(result.model.topology.edges.len(), 1);
    let edge = &result.model.topology.edges[crate::EdgeId(0)];
    match &result.model.geometry.curves[edge.curve] {
        Curve::Line(_) => {}
        _ => panic!("edge.curve should resolve to the aliased LINE"),
    }
}

// ---------------------------------------------------------------------------
// PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS (AP203 / AP242 subtype)
// ---------------------------------------------------------------------------

#[test]
fn pdef_with_associated_documents_is_recognised_as_product_definition() {
    // ashtray (grabcad) and similar AP203 fixtures emit
    //   PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS(id, desc, formation, ctx, documentation_ids)
    // in the PRODUCT chain instead of plain PRODUCT_DEFINITION. The reader
    // must accept the subtype: the entity dispatch builds pdef_to_product,
    // and the PDS classification (pdef_shape_to_pdef map) treats the subtype
    // as a valid PDEF target. Without this, the SDR handler skips silently
    // and the product ends up with `geometry_context = None` plus empty
    // content - exactly the ashtray failure mode.
    let source = minimal_step(
        "#1 = APPLICATION_CONTEXT('test');\n\
         #2 = PRODUCT_CONTEXT('',#1,'mechanical');\n\
         #3 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');\n\
         #4 = PRODUCT('P','P','',(#2));\n\
         #5 = PRODUCT_DEFINITION_FORMATION('1','',#4);\n\
         #16 = DOCUMENT_TYPE('');\n\
         #6 = DOCUMENT('','','',#16);\n\
         #7 = PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS('design','',#5,#3,(#6));\n\
         #8 = PRODUCT_DEFINITION_SHAPE('','',#7);\n\
         #10 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #11 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #12 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #13 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11,#12))\n\
         \t\tREPRESENTATION_CONTEXT('','') );\n\
         #14 = SHAPE_REPRESENTATION('',(),#13);\n\
         #15 = SHAPE_DEFINITION_REPRESENTATION(#8,#14);",
    );
    let result = convert_source(&source);
    // No warnings about missing PDEF or unresolved SDR - PDWAD must be
    // accepted just like PRODUCT_DEFINITION.
    assert!(
        result
            .warnings
            .iter()
            .all(|w| !matches!(w, ConvertError::MissingReference { .. })),
        "no MissingReference warnings expected: {:#?}",
        result.warnings,
    );
    let assembly = result.model.assembly.as_ref().expect("assembly present");
    assert_eq!(assembly.products.len(), 1);
    let product = assembly
        .products
        .iter()
        .next()
        .expect("at least one product");
    assert!(
        product.geometry_context.is_some(),
        "PDWAD product must get geometry_context bound via the SDR chain"
    );
    // The subtype's documentation_ids must be captured onto the product so the
    // writer can re-emit the subtype rather than downgrading to plain PD.
    assert_eq!(
        product.associated_documents.len(),
        1,
        "documentation_ids must be recorded on the product"
    );
    // Write-back re-emits the subtype (not a downgraded plain PRODUCT_DEFINITION)
    // and a full round-trip preserves the documentation_ids.
    let out = result.model.write_to_string().expect("write");
    assert!(
        out.contains("PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS"),
        "writer must re-emit the subtype, got:\n{out}"
    );
    let re = convert_source(&out);
    let re_product = re
        .model
        .assembly
        .as_ref()
        .expect("round-tripped assembly")
        .products
        .iter()
        .next()
        .expect("round-tripped product");
    assert_eq!(
        re_product.associated_documents.len(),
        1,
        "documentation_ids survive a full round-trip"
    );
}

#[test]
fn product_definition_id_description_materialised_in_arena() {
    // PRODUCT_DEFINITION.id / .description were dropped on read and hardcoded
    // to 'design' / '' by the writer. They are now preserved in the canonical
    // `product_definition` arena (Commit A — reader side; the writer still
    // synthesises until Commit B, so this asserts the arena on first read).
    let source = minimal_step(
        "#1 = APPLICATION_CONTEXT('test');\n\
         #2 = PRODUCT_CONTEXT('',#1,'mechanical');\n\
         #3 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');\n\
         #4 = PRODUCT('P','P','',(#2));\n\
         #5 = PRODUCT_DEFINITION_FORMATION('1','',#4);\n\
         #6 = PRODUCT_DEFINITION('MyPart','rev A',#5,#3);\n\
         #8 = PRODUCT_DEFINITION_SHAPE('','',#6);\n\
         #10 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #11 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #12 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #13 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11,#12))\n\
         \t\tREPRESENTATION_CONTEXT('','') );\n\
         #14 = SHAPE_REPRESENTATION('',(),#13);\n\
         #15 = SHAPE_DEFINITION_REPRESENTATION(#8,#14);",
    );
    let result = convert_source(&source);
    let assembly = result.model.assembly.as_ref().expect("assembly present");
    assert_eq!(assembly.product_definitions.iter().count(), 1);
    let pd = assembly.product_definitions.iter().next().unwrap();
    assert_eq!(pd.id, "MyPart");
    assert_eq!(pd.description, "rev A");
    assert!(pd.formation.is_some(), "formation resolved into the arena");
    assert!(
        pd.context.is_some(),
        "context resolved via the resolve_product_contexts post-pass"
    );
    let product = assembly.products.iter().next().unwrap();
    assert!(
        product.pdef.is_some(),
        "Product links to its canonical PD arena entry"
    );

    // Round-trip: the writer now emits id/description from the arena (Commit B),
    // so they survive write -> read (the legacy hardcoded 'design'/'' would
    // lose them).
    let out = result.model.write_to_string().expect("write");
    let re = convert_source(&out);
    let re_pd = re
        .model
        .assembly
        .as_ref()
        .expect("round-tripped assembly")
        .product_definitions
        .iter()
        .next()
        .expect("round-tripped PD");
    assert_eq!(re_pd.id, "MyPart", "PD id survives the round-trip");
    assert_eq!(re_pd.description, "rev A", "PD description survives");
}

#[test]
fn gisu_unset_used_representation_derived_from_identified_item() {
    // `GEOMETRIC_ITEM_SPECIFIC_USAGE.used_representation` is required, but CATIA
    // emits `$` for "Solid" GISUs. The reader derives it from the representation
    // that contains `identified_item` (the schema's WHERE rule) and recovers the
    // GISU instead of dropping it, surfacing a NonStandardInput normalization.
    let source = minimal_step(
        "#1 = APPLICATION_CONTEXT('test');\n\
         #2 = PRODUCT_CONTEXT('',#1,'mechanical');\n\
         #3 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');\n\
         #4 = PRODUCT('P','P','',(#2));\n\
         #5 = PRODUCT_DEFINITION_FORMATION('1','',#4);\n\
         #6 = PRODUCT_DEFINITION('part','',#5,#3);\n\
         #7 = PRODUCT_DEFINITION_SHAPE('','',#6);\n\
         #10 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
         #11 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
         #12 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );\n\
         #13 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)\n\
         \t\tGLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11,#12))\n\
         \t\tREPRESENTATION_CONTEXT('','') );\n\
         #20 = CARTESIAN_POINT('',(0.,0.,0.));\n\
         #21 = DIRECTION('',(0.,0.,1.));\n\
         #22 = DIRECTION('',(1.,0.,0.));\n\
         #23 = VECTOR('',#21,1.);\n\
         #24 = LINE('',#20,#23);\n\
         #25 = AXIS2_PLACEMENT_3D('',#20,#21,#22);\n\
         #26 = PLANE('',#25);\n\
         #27 = VERTEX_POINT('',#20);\n\
         #28 = EDGE_CURVE('',#27,#27,#24,.T.);\n\
         #29 = ORIENTED_EDGE('',*,*,#28,.T.);\n\
         #30 = EDGE_LOOP('',(#29));\n\
         #31 = FACE_BOUND('',#30,.T.);\n\
         #32 = ADVANCED_FACE('',(#31),#26,.T.);\n\
         #33 = CLOSED_SHELL('',(#32));\n\
         #34 = MANIFOLD_SOLID_BREP('',#33);\n\
         #35 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#34),#13);\n\
         #36 = SHAPE_ASPECT('','Solid',#7,.F.);\n\
         #37 = GEOMETRIC_ITEM_SPECIFIC_USAGE('','Solid',#36,$,#34);",
    );
    let result = convert_source(&source);

    // The GISU is recovered (not dropped); its used_representation resolves to
    // the only representation in the model (the ABSR containing the solid).
    assert_eq!(
        result.model.geometric_item_specific_usages.iter().count(),
        1,
        "$-used_representation GISU recovered"
    );
    let gisu = result
        .model
        .geometric_item_specific_usages
        .iter()
        .next()
        .unwrap();
    assert_eq!(
        gisu.used_representation,
        crate::ir::RepresentationId(0),
        "derived used_representation points at the ABSR"
    );

    // Surfaced as a NonStandardInput normalization (LOSS-exempt), not a defect.
    let norm = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::NonStandardInput { field, .. }
                if field.contains("GEOMETRIC_ITEM_SPECIFIC_USAGE.used_representation"))
        })
        .count();
    assert_eq!(norm, 1, "{:#?}", result.warnings);
    assert!(
        !result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::MissingReference { .. } | ConvertError::UnexpectedEntityForm { .. }
        )),
        "{:#?}",
        result.warnings
    );
}

#[test]
fn shape_aspect_of_shape_product_definition_normalised() {
    // SHAPE_ASPECT.of_shape is required to be a PRODUCT_DEFINITION_SHAPE, but the
    // C3D kernel emits a PRODUCT_DEFINITION directly (#6 below, not the PDS #7).
    // The reader accepts it, resolves to the product, and surfaces a
    // NonStandardInput normalization; the writer re-emits the standard PDS form.
    let source = minimal_step(
        "#1 = APPLICATION_CONTEXT('test');\n\
         #2 = PRODUCT_CONTEXT('',#1,'mechanical');\n\
         #3 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');\n\
         #4 = PRODUCT('P','P','',(#2));\n\
         #5 = PRODUCT_DEFINITION_FORMATION('1','',#4);\n\
         #6 = PRODUCT_DEFINITION('part','',#5,#3);\n\
         #7 = PRODUCT_DEFINITION_SHAPE('','',#6);\n\
         #8 = SHAPE_ASPECT('feat','',#6,.F.);",
    );
    let result = convert_source(&source);

    // The shape_aspect is recovered (target = the product), not dropped.
    assert_eq!(
        result.model.shape_aspects.iter().count(),
        1,
        "of_shape=PRODUCT_DEFINITION shape_aspect recovered"
    );

    // Surfaced as a NonStandardInput normalization (LOSS-exempt), no defect.
    let norm = result
        .warnings
        .iter()
        .filter(|w| {
            matches!(w, ConvertError::NonStandardInput { field, .. }
                if field == "SHAPE_ASPECT.of_shape")
        })
        .count();
    assert_eq!(norm, 1, "{:#?}", result.warnings);
    assert!(
        !result.warnings.iter().any(|w| matches!(
            w,
            ConvertError::MissingReference { .. } | ConvertError::UnexpectedEntityForm { .. }
        )),
        "{:#?}",
        result.warnings
    );
    // The target resolves to the single product (the writer re-emits the
    // standard of_shape=PDS form from it). Standard-form round-trip idempotency
    // is covered on real C3D data (input-shaft) by the reference-check run.
    let sa = result.model.shape_aspects.iter().next().unwrap();
    assert_eq!(
        sa.target,
        crate::ProductId(0),
        "of_shape resolves to product"
    );
}
