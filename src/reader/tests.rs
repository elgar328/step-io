use super::*;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Surface};
use crate::ir::id::PointId;
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit, UnitContext};
use crate::ir::topology::Orientation;

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
fn convert_2d_point_produces_dimension_mismatch_warning() {
    let result = convert_source(&minimal_step("#1 = CARTESIAN_POINT('',(10.,20.));"));
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::DimensionMismatch {
            expected: 3,
            actual: 2,
            ..
        }
    ));
    assert!(result.model.geometry.points.is_empty());
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
            assert!(n.weights.is_some());
            let ws = n.weights.as_ref().unwrap();
            assert_eq!(ws.len(), 3);
            assert!((ws[1] - 0.707).abs() < 0.001);
        }
        Curve::Line(_)
        | Curve::Circle(_)
        | Curve::Ellipse(_)
        | Curve::Trimmed(_)
        | Curve::Composite(_) => panic!("expected Nurbs"),
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
    assert_eq!(solid.name.as_deref(), Some("Test"));
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
    assert_eq!(solid.name, None);
}

#[test]
fn convert_face_bound_is_outer_false() {
    let result = convert_source(&full_topology_step());
    let wire = &result.model.topology.wires[crate::WireId(0)];
    assert!(!wire.is_outer);
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
    assert!(wire.is_outer);
}

// ---------------------------------------------------------------------------
// Unit context (Pass 0)
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
    assert_eq!(
        result.model.units.iter().next().cloned(),
        Some(UnitContext {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        }),
    );
}

#[test]
fn unit_centimetre_mapping() {
    let result = convert_source(&minimal_step(&unit_data(".CENTI.")));
    assert!(result.warnings.is_empty());
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Centimetre),
    );
}

#[test]
fn unit_plain_metre_mapping() {
    let result = convert_source(&minimal_step(&unit_data("$")));
    assert!(result.warnings.is_empty());
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Metre),
    );
}

#[test]
fn unit_unsupported_prefix_produces_warning_and_none() {
    let result = convert_source(&minimal_step(&unit_data(".KILO.")));
    // Two warnings expected: (1) the leaf flagged .KILO. as unsupported,
    // (2) the global context couldn't fill the length slot.
    assert_eq!(result.warnings.len(), 2, "{:#?}", result.warnings);
    assert!(matches!(
        &result.warnings[0],
        ConvertError::UnexpectedEntityForm { detail, .. }
            if detail.contains("unsupported SI length unit")
    ));
    assert!(result.model.units.is_empty());
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Inch),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Inch),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Inch),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Foot),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.plane_angle),
        Some(AngleUnit::Degree),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.plane_angle),
        Some(AngleUnit::Degree),
    );
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
    assert_eq!(
        result.model.units.iter().next().map(|u| u.length),
        Some(LengthUnit::Millimetre),
    );
}

#[test]
fn reads_unrecognized_cbu_name_warns() {
    // An unknown name on a LENGTH_UNIT-flavoured CBU: reader should produce
    // an UnexpectedEntityForm warning (not a silent skip), and leave units=None
    // because the GUAC ref can't be resolved.
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
    assert!(result.model.units.is_empty());
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
    assert!(result.warnings.is_empty(), "{:#?}", result.warnings);
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
