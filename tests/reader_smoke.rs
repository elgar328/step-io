//! Integration tests for the reader: parse fixtures → convert → verify IR.

use step_io::ir::geometry::{Curve, Surface};
use step_io::ir::id::PointId;
use step_io::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};
use step_io::ir::units::NamedUnit;
use step_io::reader::ReaderContext;

// ------------------------------------------------------------------
// Box fixtures
// ------------------------------------------------------------------

const BOX_FIXTURES: &[(&str, &str)] = &[
    ("box_ap203", include_str!("fixtures/box_ap203.step")),
    ("box_ap214_cd", include_str!("fixtures/box_ap214_cd.step")),
    ("box_ap214_dis", include_str!("fixtures/box_ap214_dis.step")),
    ("box_ap214_is", include_str!("fixtures/box_ap214_is.step")),
    ("box_ap242_dis", include_str!("fixtures/box_ap242_dis.step")),
];

#[test]
fn box_fixtures_convert_without_warnings() {
    for (name, source) in BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn box_fixtures_geometry_pool_counts() {
    for (name, source) in BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 27, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 26, "fixture {name}: directions");
        assert_eq!(geo.curves.len(), 12, "fixture {name}: curves");
        assert_eq!(geo.surfaces.len(), 6, "fixture {name}: surfaces");
    }
}

/// Spot-check coordinate values from `box_ap214_is`.
/// Arena index 0 corresponds to STEP `#12` (the lowest-numbered `CARTESIAN_POINT`,
/// due to `BTreeMap`'s ascending iteration order).
#[test]
fn box_ap214_is_spot_check_coordinates() {
    let source = include_str!("fixtures/box_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    // #12 = CARTESIAN_POINT('',(0.,0.,0.)) → PointId(0)
    let pt0 = &geo.points[PointId(0)];
    assert!((pt0.x).abs() < f64::EPSILON);
    assert!((pt0.y).abs() < f64::EPSILON);
    assert!((pt0.z).abs() < f64::EPSILON);

    // #162 = CARTESIAN_POINT('',(50.,50.,100.))
    // It is the 27th (last) point → PointId(26), since box_ap214_is has 27 points
    // and #162 is the highest-numbered CARTESIAN_POINT.
    let pt_last = &geo.points[PointId(26)];
    assert!((pt_last.x - 50.0).abs() < f64::EPSILON);
    assert!((pt_last.y - 50.0).abs() < f64::EPSILON);
    assert!((pt_last.z - 100.0).abs() < f64::EPSILON);
}

#[test]
fn box_fixtures_all_surfaces_are_planes() {
    for (name, source) in BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        for surface in result.model.geometry.surfaces.iter() {
            assert!(
                matches!(surface, Surface::Plane(_)),
                "fixture {name}: expected all surfaces to be Plane"
            );
        }
    }
}

#[test]
fn box_fixtures_all_curves_are_lines() {
    for (name, source) in BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        for curve in result.model.geometry.curves.iter() {
            assert!(
                matches!(curve, Curve::Line(_)),
                "fixture {name}: expected all curves to be Line"
            );
        }
    }
}

// ------------------------------------------------------------------
// Cylinder fixtures
// ------------------------------------------------------------------

const CYLINDER_FIXTURES: &[(&str, &str)] = &[(
    "cylinder_ap214_is",
    include_str!("fixtures/cylinder_ap214_is.step"),
)];

#[test]
fn cylinder_fixtures_convert_without_warnings() {
    for (name, source) in CYLINDER_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn cylinder_fixtures_geometry_pool_counts() {
    for (name, source) in CYLINDER_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 9, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 13, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            3,
            "fixture {name}: curves (1 line + 2 circles)"
        );
        assert_eq!(
            geo.surfaces.len(),
            3,
            "fixture {name}: surfaces (2 planes + 1 cylinder)"
        );
    }
}

/// Spot-check cylinder-specific entities from `cylinder_ap214_is`.
#[test]
fn cylinder_ap214_is_spot_check_radius() {
    let source = include_str!("fixtures/cylinder_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    // Verify CYLINDRICAL_SURFACE with radius 50.
    let has_cylinder_50 = geo
        .surfaces
        .iter()
        .any(|s| matches!(s, Surface::Cylinder(c) if (c.radius - 50.0).abs() < f64::EPSILON));
    assert!(
        has_cylinder_50,
        "expected a cylindrical surface with radius 50"
    );

    // Verify CIRCLE with radius 50.
    let circle_count = geo
        .curves
        .iter()
        .filter(|c| matches!(c, Curve::Circle(c) if (c.radius - 50.0).abs() < f64::EPSILON))
        .count();
    assert_eq!(circle_count, 2, "expected 2 circles with radius 50");
}

// ------------------------------------------------------------------
// Topology pool counts
// ------------------------------------------------------------------

#[test]
fn box_fixtures_topology_pool_counts() {
    for (name, source) in BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            8,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 12, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 6, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 6, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn cylinder_fixtures_topology_pool_counts() {
    for (name, source) in CYLINDER_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            2,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 3, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 3, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 3, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

/// Spot-check: box solid has 1 shell with 6 faces.
#[test]
fn box_ap214_is_solid_structure() {
    let source = include_str!("fixtures/box_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let topo = &result.model.topology;

    assert_eq!(topo.solids.len(), 1);
    let solid = &topo.solids[step_io::SolidId(0)];
    assert_eq!(solid.shells.len(), 1);

    let shell = &topo.shells[solid.shells[0]];
    assert_eq!(shell.faces.len(), 6);
}

// ------------------------------------------------------------------
// Fillet box fixtures
// ------------------------------------------------------------------

const FILLET_BOX_FIXTURES: &[(&str, &str)] = &[(
    "fillet_box_ap214_is",
    include_str!("fixtures/fillet_box_ap214_is.step"),
)];

#[test]
fn fillet_box_fixtures_convert_without_warnings() {
    for (name, source) in FILLET_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn fillet_box_fixtures_geometry_pool_counts() {
    for (name, source) in FILLET_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 99, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 126, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            48,
            "fixture {name}: curves (24 lines + 24 circles)"
        );
        assert_eq!(
            geo.surfaces.len(),
            26,
            "fixture {name}: surfaces (6 planes + 12 cylinders + 8 spheres)"
        );
    }
}

#[test]
fn fillet_box_fixtures_topology_pool_counts() {
    for (name, source) in FILLET_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            24,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 48, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 26, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 26, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

/// Spot-check: fillet box has spherical surfaces with radius 10.
#[test]
fn fillet_box_ap214_is_spot_check_spherical_surface() {
    let source = include_str!("fixtures/fillet_box_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    let sphere_count = geo
        .surfaces
        .iter()
        .filter(|s| matches!(s, Surface::Sphere(s) if (s.radius - 10.0).abs() < f64::EPSILON))
        .count();
    assert_eq!(
        sphere_count, 8,
        "expected 8 spherical surfaces with radius 10"
    );
}

// ------------------------------------------------------------------
// Cone fixtures
// ------------------------------------------------------------------

const CONE_FIXTURES: &[(&str, &str)] =
    &[("cone_ap214_is", include_str!("fixtures/cone_ap214_is.step"))];

#[test]
fn cone_fixtures_convert_without_warnings() {
    for (name, source) in CONE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn cone_fixtures_pool_counts() {
    for (name, source) in CONE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;
        let topo = &result.model.topology;

        assert_eq!(geo.points.len(), 7, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 9, "fixture {name}: directions");
        assert_eq!(geo.curves.len(), 2, "fixture {name}: curves");
        assert_eq!(geo.surfaces.len(), 2, "fixture {name}: surfaces");
        assert_eq!(
            result.model.geometry.vertices.len(),
            2,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 2, "fixture {name}: edges");
        assert_eq!(topo.faces.len(), 2, "fixture {name}: faces");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn cone_ap214_is_spot_check_conical_surface() {
    let source = include_str!("fixtures/cone_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);

    let has_cone = result
        .model
        .geometry
        .surfaces
        .iter()
        .any(|s| matches!(s, Surface::Cone(c) if c.radius.abs() < f64::EPSILON && (c.semi_angle - 0.4636).abs() < 0.001));
    assert!(
        has_cone,
        "expected a conical surface with radius=0 and semi_angle≈0.464"
    );
}

// ------------------------------------------------------------------
// Torus fixtures
// ------------------------------------------------------------------

const TORUS_FIXTURES: &[(&str, &str)] = &[(
    "torus_ap214_is",
    include_str!("fixtures/torus_ap214_is.step"),
)];

#[test]
fn torus_fixtures_convert_without_warnings() {
    for (name, source) in TORUS_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn torus_fixtures_pool_counts() {
    for (name, source) in TORUS_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;
        let topo = &result.model.topology;

        assert_eq!(geo.points.len(), 5, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 8, "fixture {name}: directions");
        assert_eq!(geo.curves.len(), 2, "fixture {name}: curves");
        assert_eq!(geo.surfaces.len(), 1, "fixture {name}: surfaces");
        assert_eq!(
            result.model.geometry.vertices.len(),
            1,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 2, "fixture {name}: edges");
        assert_eq!(topo.faces.len(), 1, "fixture {name}: faces");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn torus_ap214_is_spot_check() {
    let source = include_str!("fixtures/torus_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);

    let has_torus = result.model.geometry.surfaces.iter().any(|s| {
        matches!(s, Surface::Torus(t) if (t.major_radius - 37.5).abs() < f64::EPSILON && (t.minor_radius - 12.5).abs() < f64::EPSILON)
    });
    assert!(
        has_torus,
        "expected toroidal surface with major=37.5, minor=12.5"
    );
}

// ------------------------------------------------------------------
// Revolution fixtures
// ------------------------------------------------------------------

const REVOLUTION_FIXTURES: &[(&str, &str)] = &[(
    "revolution_ap214_is",
    include_str!("fixtures/revolution_ap214_is.step"),
)];

#[test]
fn revolution_fixtures_convert_without_warnings() {
    for (name, source) in REVOLUTION_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn revolution_fixtures_pool_counts() {
    for (name, source) in REVOLUTION_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;
        let topo = &result.model.topology;

        assert_eq!(geo.points.len(), 36, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 11, "fixture {name}: directions");
        assert_eq!(geo.curves.len(), 4, "fixture {name}: curves");
        assert_eq!(geo.surfaces.len(), 3, "fixture {name}: surfaces");
        assert_eq!(
            result.model.geometry.vertices.len(),
            2,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 3, "fixture {name}: edges");
        assert_eq!(topo.faces.len(), 3, "fixture {name}: faces");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn revolution_ap214_is_spot_check() {
    let source = include_str!("fixtures/revolution_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    let has_revolution = geo
        .surfaces
        .iter()
        .any(|s| matches!(s, Surface::Revolution(_)));
    assert!(has_revolution, "expected a surface of revolution");

    // Verify 2 rational B-spline curves (weights: Some, not all 1.0).
    let rational_count = geo
        .curves
        .iter()
        .filter(|c| matches!(c, Curve::Nurbs(n) if n.weights().is_some()))
        .count();
    assert_eq!(rational_count, 2, "expected 2 rational NURBS curves");

    // Verify at least one weight is not 1.0 (true rational).
    let has_non_unit_weight = geo.curves.iter().any(|c| match c {
        Curve::Nurbs(n) => n
            .weights()
            .is_some_and(|ws| ws.iter().any(|&w| (w - 1.0).abs() > f64::EPSILON)),
        _ => false,
    });
    assert!(
        has_non_unit_weight,
        "expected at least one weight != 1.0 (truly rational)"
    );
}

// ------------------------------------------------------------------
// Loft fixtures
// ------------------------------------------------------------------

const LOFT_FIXTURES: &[(&str, &str)] =
    &[("loft_ap214_is", include_str!("fixtures/loft_ap214_is.step"))];

#[test]
fn loft_fixtures_convert_without_warnings() {
    for (name, source) in LOFT_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn loft_fixtures_geometry_pool_counts() {
    for (name, source) in LOFT_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 125, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 21, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            15,
            "fixture {name}: curves (5 lines + 5 circles + 5 nurbs)"
        );
        assert_eq!(
            geo.surfaces.len(),
            7,
            "fixture {name}: surfaces (2 planes + 5 nurbs)"
        );
    }
}

#[test]
fn loft_fixtures_topology_pool_counts() {
    for (name, source) in LOFT_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            10,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 15, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 7, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 7, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

// ------------------------------------------------------------------
// Tapered box fixtures
// ------------------------------------------------------------------

const TAPERED_BOX_FIXTURES: &[(&str, &str)] = &[(
    "tapered_box_ap214_is",
    include_str!("fixtures/tapered_box_ap214_is.step"),
)];

#[test]
fn tapered_box_fixtures_convert_without_warnings() {
    for (name, source) in TAPERED_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn tapered_box_fixtures_geometry_pool_counts() {
    for (name, source) in TAPERED_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 56, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 13, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            12,
            "fixture {name}: curves (7 lines + 3 simple nurbs + 2 rational nurbs)"
        );
        assert_eq!(
            geo.surfaces.len(),
            6,
            "fixture {name}: surfaces (2 planes + 3 simple nurbs + 1 rational nurbs)"
        );
    }
}

#[test]
fn tapered_box_fixtures_topology_pool_counts() {
    for (name, source) in TAPERED_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            8,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 12, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 6, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 6, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

/// Spot-check `RATIONAL_B_SPLINE_SURFACE` plus regression for simple
/// `NurbsSurface`, both produced by the loft between two rectangle profiles.
#[test]
fn tapered_box_ap214_is_spot_check() {
    let source = include_str!("fixtures/tapered_box_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    // Surface NURBS breakdown: 1 rational (weights: Some) + 2 simple (weights: None).
    let rational_surface_count = geo
        .surfaces
        .iter()
        .filter(|s| matches!(s, Surface::Nurbs(n) if n.weights().is_some()))
        .count();
    assert_eq!(
        rational_surface_count, 1,
        "expected 1 rational NURBS surface"
    );
    let simple_surface_count = geo
        .surfaces
        .iter()
        .filter(|s| matches!(s, Surface::Nurbs(n) if n.weights().is_none()))
        .count();
    assert_eq!(
        simple_surface_count, 3,
        "expected 3 simple (non-rational) NURBS surfaces"
    );

    // Validate the rational surface's weights grid dimensions match control points.
    let rational = geo
        .surfaces
        .iter()
        .find_map(|s| match s {
            Surface::Nurbs(n) if n.weights().is_some() => Some(n),
            _ => None,
        })
        .expect("rational NURBS surface missing");
    let weights = rational.weights().unwrap();
    assert_eq!(weights.len(), rational.control_points.len());
    for (w_row, cp_row) in weights.iter().zip(rational.control_points.iter()) {
        assert_eq!(w_row.len(), cp_row.len());
    }
    // Truly rational — at least one weight ≠ 1.0.
    let has_non_unit_weight = weights
        .iter()
        .any(|row| row.iter().any(|&w| (w - 1.0).abs() > f64::EPSILON));
    assert!(
        has_non_unit_weight,
        "expected at least one weight != 1.0 in the rational surface"
    );
}

/// Spot-check NURBS curve and surface properties from `loft_ap214_is`.
#[test]
fn loft_ap214_is_spot_check_nurbs() {
    let source = include_str!("fixtures/loft_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    // Count NURBS curves (should be 5, all degree 1).
    let nurbs_curves: Vec<_> = geo
        .curves
        .iter()
        .filter_map(|c| match c {
            Curve::Nurbs(n) => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(nurbs_curves.len(), 5, "expected 5 NURBS curves");
    for nc in &nurbs_curves {
        assert_eq!(nc.degree, 1);
        assert_eq!(nc.control_points.len(), 2);
        assert!(nc.weights().is_none());
        assert_eq!(nc.knots.len(), nc.knot_multiplicities.len());
    }

    // Count NURBS surfaces (should be 5).
    let nurbs_surfaces: Vec<_> = geo
        .surfaces
        .iter()
        .filter_map(|s| match s {
            Surface::Nurbs(n) => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(nurbs_surfaces.len(), 5, "expected 5 NURBS surfaces");
    for ns in &nurbs_surfaces {
        assert!(ns.u_degree >= 7);
        assert_eq!(ns.v_degree, 1);
        assert!(ns.weights().is_none());
        assert_eq!(ns.u_knots.len(), ns.u_knot_multiplicities.len());
        assert_eq!(ns.v_knots.len(), ns.v_knot_multiplicities.len());
    }
}

// ------------------------------------------------------------------
// Ellipse fixtures
// ------------------------------------------------------------------

const ELLIPSE_FIXTURES: &[(&str, &str)] = &[(
    "ellipse_ap214_is",
    include_str!("fixtures/ellipse_ap214_is.step"),
)];

#[test]
fn ellipse_fixtures_convert_without_warnings() {
    for (name, source) in ELLIPSE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn ellipse_fixtures_geometry_pool_counts() {
    for (name, source) in ELLIPSE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 9, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 14, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            4,
            "fixture {name}: curves (1 line + 3 ellipse)"
        );
        assert_eq!(
            geo.surfaces.len(),
            3,
            "fixture {name}: surfaces (2 planes + 1 linear extrusion)"
        );
    }
}

#[test]
fn ellipse_fixtures_topology_pool_counts() {
    for (name, source) in ELLIPSE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            2,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 3, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 3, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 3, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn ellipse_ap214_is_spot_check() {
    let source = include_str!("fixtures/ellipse_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    let ellipses: Vec<_> = geo
        .curves
        .iter()
        .filter_map(|c| match c {
            Curve::Ellipse(e) => Some(e),
            _ => None,
        })
        .collect();
    assert_eq!(ellipses.len(), 3, "expected 3 Curve::Ellipse");
    for e in &ellipses {
        assert!(
            (e.semi_axis_1 - 50.0).abs() < f64::EPSILON,
            "semi_axis_1 should be 50.0, got {}",
            e.semi_axis_1,
        );
        assert!(
            (e.semi_axis_2 - 30.0).abs() < f64::EPSILON,
            "semi_axis_2 should be 30.0, got {}",
            e.semi_axis_2,
        );
    }

    // Exactly 1 Surface::Extrusion (from the pad side).
    let extrusion_count = geo
        .surfaces
        .iter()
        .filter(|s| matches!(s, Surface::Extrusion(_)))
        .count();
    assert_eq!(extrusion_count, 1, "expected 1 Surface::Extrusion");

    // Extrusion depth must be positive (VECTOR magnitude).
    let extrusion = geo
        .surfaces
        .iter()
        .find_map(|s| match s {
            Surface::Extrusion(e) => Some(e),
            _ => None,
        })
        .expect("extrusion surface missing");
    assert!(
        extrusion.depth > 0.0,
        "extrusion depth should be positive, got {}",
        extrusion.depth
    );
}

// ------------------------------------------------------------------
// Hollow box fixtures — exercise BREP_WITH_VOIDS + ORIENTED_CLOSED_SHELL
// ------------------------------------------------------------------

const HOLLOW_BOX_FIXTURES: &[(&str, &str)] = &[(
    "hollow_box_ap214_is",
    include_str!("fixtures/hollow_box_ap214_is.step"),
)];

#[test]
fn hollow_box_fixtures_convert_without_warnings() {
    for (name, source) in HOLLOW_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn hollow_box_fixtures_topology_pool_counts() {
    for (name, source) in HOLLOW_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        // outer box + inner cylinder void; exact face count depends on how
        // FreeCAD split the cylindrical surface — see the per-AP fixture.
        assert_eq!(
            topo.shells.len(),
            2,
            "fixture {name}: shells (outer + inner void)"
        );
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

#[test]
fn hollow_box_solid_has_outer_plus_one_void() {
    use step_io::Orientation;
    for (name, source) in HOLLOW_BOX_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;
        let solid = topo.solids.iter().next().expect("one solid");
        assert_eq!(solid.shells.len(), 2, "fixture {name}: 1 outer + 1 void");

        let outer = topo.shells.iter().nth(solid.shells[0].0 as usize).unwrap();
        let inner = topo.shells.iter().nth(solid.shells[1].0 as usize).unwrap();
        assert_eq!(
            outer.orientation,
            Orientation::Forward,
            "fixture {name}: outer Forward"
        );
        assert_eq!(
            inner.orientation,
            Orientation::Reversed,
            "fixture {name}: void Reversed"
        );
    }
}

// ------------------------------------------------------------------
// Face surface fixtures — exercise FaceKind::General (FACE_SURFACE)
// ------------------------------------------------------------------

const FACE_SURFACE_FIXTURES: &[(&str, &str)] = &[(
    "face_surface_ap214_is",
    include_str!("fixtures/face_surface_ap214_is.step"),
)];

#[test]
fn face_surface_fixtures_convert_without_warnings() {
    for (name, source) in FACE_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn face_surface_fixtures_geometry_pool_counts() {
    for (name, source) in FACE_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 17, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 2, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            4,
            "fixture {name}: curves (4 simple nurbs edge curves)"
        );
        assert_eq!(geo.surfaces.len(), 1, "fixture {name}: surfaces (1 plane)");
    }
}

#[test]
fn face_surface_fixtures_topology_pool_counts() {
    for (name, source) in FACE_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            4,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 4, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 1, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 1, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 0, "fixture {name}: solids");
    }
}

/// Spot-check `FACE_SURFACE` round-trip — verifies `FaceKind::General` is
/// preserved on read and emitted back as `FACE_SURFACE(`, not downgraded to
/// `ADVANCED_FACE(`.
#[test]
fn face_surface_ap214_is_spot_check() {
    use step_io::ir::topology::FaceKind;

    let source = include_str!("fixtures/face_surface_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let faces: Vec<_> = result.model.topology.faces.iter().collect();

    assert_eq!(faces.len(), 1, "expected exactly 1 face");
    assert_eq!(
        faces[0].kind,
        FaceKind::General,
        "face kind should be General (FACE_SURFACE origin)"
    );

    // Round-trip the IR through the writer and check the emitted text.
    let written = result
        .model
        .write_to_string()
        .expect("writer produced output");
    let face_surface_count = written.matches("FACE_SURFACE(").count();
    let advanced_face_count = written.matches("ADVANCED_FACE(").count();
    assert_eq!(
        face_surface_count, 1,
        "expected 1 FACE_SURFACE( occurrence in round-trip output"
    );
    assert_eq!(
        advanced_face_count, 0,
        "expected 0 ADVANCED_FACE( occurrences in round-trip output"
    );
}

// ------------------------------------------------------------------
// Offset surface fixtures — exercise Surface::Offset (OFFSET_SURFACE)
// ------------------------------------------------------------------

const OFFSET_SURFACE_FIXTURES: &[(&str, &str)] = &[(
    "offset_surface_ap214_is",
    include_str!("fixtures/offset_surface_ap214_is.step"),
)];

#[test]
fn offset_surface_fixtures_convert_without_warnings() {
    for (name, source) in OFFSET_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn offset_surface_fixtures_geometry_pool_counts() {
    for (name, source) in OFFSET_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let geo = &result.model.geometry;

        assert_eq!(geo.points.len(), 246, "fixture {name}: points");
        assert_eq!(geo.directions.len(), 39, "fixture {name}: directions");
        assert_eq!(
            geo.curves.len(),
            25,
            "fixture {name}: curves (18 lines + 7 simple nurbs)"
        );
        assert_eq!(
            geo.surfaces.len(),
            12,
            "fixture {name}: surfaces (9 planes + 1 simple nurbs + 1 extrusion + 1 offset)"
        );
    }
}

#[test]
fn offset_surface_fixtures_topology_pool_counts() {
    for (name, source) in OFFSET_SURFACE_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let topo = &result.model.topology;

        assert_eq!(
            result.model.geometry.vertices.len(),
            16,
            "fixture {name}: vertices"
        );
        assert_eq!(topo.edges.len(), 24, "fixture {name}: edges");
        assert_eq!(topo.wires.len(), 12, "fixture {name}: wires");
        assert_eq!(topo.faces.len(), 11, "fixture {name}: faces");
        assert_eq!(topo.shells.len(), 1, "fixture {name}: shells");
        assert_eq!(topo.solids.len(), 1, "fixture {name}: solids");
    }
}

/// Spot-check `OFFSET_SURFACE` round-trip — `Surface::Offset` preserved,
/// basis resolves to an already-interned surface, and writer emits the
/// entity name back verbatim.
#[test]
fn offset_surface_ap214_is_spot_check() {
    let source = include_str!("fixtures/offset_surface_ap214_is.step");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let geo = &result.model.geometry;

    let offset_count = geo
        .surfaces
        .iter()
        .filter(|s| matches!(s, Surface::Offset(_)))
        .count();
    assert_eq!(offset_count, 1, "expected 1 Surface::Offset");

    let offset = geo
        .surfaces
        .iter()
        .find_map(|s| match s {
            Surface::Offset(o) => Some(o),
            _ => None,
        })
        .expect("offset surface missing");

    assert!(
        (offset.distance.abs() - 5.0).abs() < f64::EPSILON,
        "expected |distance| == 5.0, got {}",
        offset.distance,
    );

    let basis = &geo.surfaces[offset.basis];
    assert!(
        matches!(basis, Surface::Nurbs(_)),
        "expected basis to resolve to a Surface::Nurbs"
    );

    // Round-trip: emitted text must preserve OFFSET_SURFACE entity name.
    let written = result
        .model
        .write_to_string()
        .expect("writer produced output");
    let offset_surface_count = written.matches("OFFSET_SURFACE(").count();
    assert_eq!(
        offset_surface_count, 1,
        "expected 1 OFFSET_SURFACE( occurrence in round-trip output"
    );
}

// ------------------------------------------------------------------
// Unit context — every fixture exports mm/radian/steradian
// ------------------------------------------------------------------

const ALL_FIXTURE_GROUPS: &[&[(&str, &str)]] = &[
    BOX_FIXTURES,
    CYLINDER_FIXTURES,
    FILLET_BOX_FIXTURES,
    CONE_FIXTURES,
    TORUS_FIXTURES,
    REVOLUTION_FIXTURES,
    LOFT_FIXTURES,
    TAPERED_BOX_FIXTURES,
    ELLIPSE_FIXTURES,
    HOLLOW_BOX_FIXTURES,
    ASSEMBLY_FIXTURES,
    FACE_SURFACE_FIXTURES,
    OFFSET_SURFACE_FIXTURES,
];

// ------------------------------------------------------------------
// Assembly fixtures (Phase A: products parsed, tree root not yet wired)
// ------------------------------------------------------------------

const ASSEMBLY_FIXTURES: &[(&str, &str)] = &[(
    "assembly_ap214_is",
    include_str!("fixtures/assembly_ap214_is.step"),
)];

#[test]
fn assembly_fixtures_convert_without_warnings() {
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: expected no warnings, got {:#?}",
            result.warnings,
        );
    }
}

#[test]
fn assembly_fixtures_have_seven_products() {
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result
            .model
            .assembly
            .as_ref()
            .unwrap_or_else(|| panic!("fixture {name}: assembly should be Some"));
        assert_eq!(tree.products.len(), 7, "fixture {name}: product count");
        assert!(!tree.roots.is_empty(), "fixture {name}: root resolved");
    }
}

#[test]
fn assembly_fixtures_content_variants_split_three_solids_four_groups() {
    use step_io::ProductContent;
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        let (solid_count, group_count) =
            tree.products
                .iter()
                .fold((0_usize, 0_usize), |(sol, grp), p| match &p.content {
                    ProductContent::Solid(_)
                    | ProductContent::SurfaceBody(_)
                    | ProductContent::Wireframe(_) => (sol + 1, grp),
                    ProductContent::Group(_) => (sol, grp + 1),
                });
        assert_eq!(solid_count, 3, "fixture {name}: leaf solids");
        assert_eq!(group_count, 4, "fixture {name}: groups");
        // Every Solid's SolidId must live in the topology pool.
        for product in tree.products.iter() {
            if let ProductContent::Solid(solid) = &product.content {
                let total = u32::try_from(result.model.topology.solids.len()).unwrap();
                for sid in &solid.ids {
                    assert!(
                        sid.0 < total,
                        "fixture {name}: product solid id out of range"
                    );
                }
            }
        }
        // Expected names all present.
        let names: Vec<&str> = tree.products.iter().map(|p| p.name.as_str()).collect();
        for expected in [
            "Assembly", "Cube", "Cylinder", "Sphere", "Part", "Part001", "Part002",
        ] {
            assert!(
                names.contains(&expected),
                "fixture {name}: missing product '{expected}'"
            );
        }
    }
}

#[test]
fn assembly_fixtures_preserve_sphere_vertex_loop() {
    // Each assembly fixture has exactly one sphere, whose face uses a
    // degenerate VERTEX_LOOP boundary. Exactly one Wire should carry
    // `vertex.is_some()` with no edges; the rest are edge loops.
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let vertex_wire_count = result
            .model
            .topology
            .wires
            .iter()
            .filter(|w| w.vertex.is_some() && w.edges.is_empty())
            .count();
        assert_eq!(
            vertex_wire_count, 1,
            "fixture {name}: expected exactly one VERTEX_LOOP-backed Wire"
        );
    }
}

#[test]
fn single_part_fixtures_have_one_product() {
    use step_io::ProductContent;
    // Even single-part STEP files carry one PRODUCT; the assembly tree is
    // always Some(...) with `products.len() == 1` and the sole product
    // classified as a Solid leaf (root stays None until Phase B).
    for group in &[
        BOX_FIXTURES,
        CYLINDER_FIXTURES,
        FILLET_BOX_FIXTURES,
        CONE_FIXTURES,
        TORUS_FIXTURES,
        REVOLUTION_FIXTURES,
        LOFT_FIXTURES,
        TAPERED_BOX_FIXTURES,
        ELLIPSE_FIXTURES,
    ] {
        for (name, source) in *group {
            let graph = step_io::parse(source).expect(name);
            let result = ReaderContext::convert(&graph);
            let tree =
                result.model.assembly.as_ref().unwrap_or_else(|| {
                    panic!("fixture {name}: single-part should have Some(tree)")
                });
            assert_eq!(tree.products.len(), 1, "fixture {name}: products");
            // The sole product — having never appeared as an Instance child
            // — is the automatic root.
            assert_eq!(
                tree.roots,
                vec![step_io::ProductId(0)],
                "fixture {name}: the only product is the sole root"
            );
            let product = tree.products.iter().next().unwrap();
            assert!(
                matches!(product.content, ProductContent::Solid(_)),
                "fixture {name}: single-part product should be Solid(_)"
            );
        }
    }
}

#[test]
fn every_fixture_has_expected_units() {
    for group in ALL_FIXTURE_GROUPS {
        for (name, source) in *group {
            let graph = step_io::parse(source)
                .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
            let result = ReaderContext::convert(&graph);
            let units = result
                .model
                .units
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| panic!("fixture {name}: units missing"));
            let pool = result
                .model
                .units_pool
                .as_ref()
                .unwrap_or_else(|| panic!("fixture {name}: units pool missing"));
            let length = match pool.named_units[units.length] {
                NamedUnit::Length(f) => f.unit,
                _ => panic!("fixture {name}: length slot is not Length"),
            };
            let plane_angle = match pool.named_units[units.plane_angle] {
                NamedUnit::PlaneAngle(f) => f.unit,
                _ => panic!("fixture {name}: plane_angle slot is not PlaneAngle"),
            };
            let solid_angle = match pool.named_units[units.solid_angle] {
                NamedUnit::SolidAngle(f) => f.unit,
                _ => panic!("fixture {name}: solid_angle slot is not SolidAngle"),
            };
            let expected_length = if *name == "fillet_box_ap214_is" {
                LengthUnit::Inch
            } else {
                LengthUnit::Millimetre
            };
            assert_eq!(length, expected_length, "fixture {name}: length");
            assert_eq!(
                plane_angle,
                AngleUnit::Radian,
                "fixture {name}: plane_angle"
            );
            assert_eq!(
                solid_angle,
                SolidAngleUnit::Steradian,
                "fixture {name}: solid_angle",
            );
            // Every FreeCAD fixture carries a length uncertainty. Values
            // vary per-fixture (1e-7 vs 2e-7), so just assert presence.
            assert!(
                units.length_uncertainty.is_some(),
                "fixture {name}: length_uncertainty missing",
            );
        }
    }
}

// ------------------------------------------------------------------
// Phase B tree-shape assertions
// ------------------------------------------------------------------

#[test]
fn assembly_fixtures_tree_root_is_assembly_product() {
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        let root = tree
            .roots
            .first()
            .copied()
            .expect("root should be resolved");
        let root_product = &tree.products[root];
        assert_eq!(
            root_product.name, "Assembly",
            "fixture {name}: root should be the top-level Assembly product"
        );
    }
}

#[test]
fn assembly_fixtures_root_has_four_instances() {
    use step_io::ProductContent;
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        let root = tree
            .roots
            .first()
            .copied()
            .expect("root should be resolved");
        match &tree.products[root].content {
            ProductContent::Group(group) => {
                assert_eq!(
                    group.instances.len(),
                    4,
                    "fixture {name}: root should hold 4 instances"
                );
            }
            ProductContent::Solid(_)
            | ProductContent::SurfaceBody(_)
            | ProductContent::Wireframe(_) => {
                panic!("fixture {name}: root should be a Group")
            }
        }
    }
}

#[test]
fn assembly_fixtures_cube_wrapper_is_shared() {
    use step_io::ProductContent;
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        // The Cube wrapper is a Group with a single inner Cube leaf.
        let Some((idx, _)) = tree.products.iter().enumerate().find(|(_, p)| {
            p.name == "Part"
                && matches!(&p.content, ProductContent::Group(v) if v.instances.len() == 1)
        }) else {
            panic!("fixture {name}: Cube wrapper 'Part' not found");
        };
        let cube_wrapper = step_io::ProductId(u32::try_from(idx).unwrap());

        let root = tree
            .roots
            .first()
            .copied()
            .expect("root should be resolved");
        let ProductContent::Group(root_group) = &tree.products[root].content else {
            panic!("fixture {name}: root not Group");
        };
        let shared_count = root_group
            .instances
            .iter()
            .filter(|inst| inst.child == cube_wrapper)
            .count();
        assert_eq!(
            shared_count, 2,
            "fixture {name}: Cube wrapper should be referenced twice from root"
        );

        // The two shared instances must have different transforms.
        let mut targets = root_group
            .instances
            .iter()
            .filter(|inst| inst.child == cube_wrapper)
            .map(|inst| inst.transform.target);
        let t1 = targets.next().unwrap();
        let t2 = targets.next().unwrap();
        assert_ne!(
            t1, t2,
            "fixture {name}: shared instances should have distinct transforms"
        );
    }
}

#[test]
fn assembly_fixtures_wrapper_holds_single_inner() {
    use step_io::ProductContent;
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        // Each non-root Group (Part / Part001 / Part002 wrapper) holds
        // exactly one inner Instance.
        let wrapper_count = tree
            .products
            .iter()
            .filter(|p| matches!(&p.content, ProductContent::Group(v) if v.instances.len() == 1))
            .count();
        assert_eq!(
            wrapper_count, 3,
            "fixture {name}: exactly three single-instance wrappers"
        );
    }
}

#[test]
fn assembly_fixtures_transform_target_is_non_origin() {
    use step_io::ProductContent;
    for (name, source) in ASSEMBLY_FIXTURES {
        let graph = step_io::parse(source)
            .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"));
        let result = ReaderContext::convert(&graph);
        let tree = result.model.assembly.as_ref().unwrap();
        // At least one instance should place its child away from origin —
        // fixtures place cylinder, sphere and the extra cube at non-zero
        // coordinates to exercise the transform path.
        let mut found_non_origin = false;
        for product in tree.products.iter() {
            if let ProductContent::Group(group) = &product.content {
                for inst in &group.instances {
                    let target = result.model.geometry.placements[inst.transform.target];
                    let pt = &result.model.geometry.points[target.location];
                    if pt.x.abs() + pt.y.abs() + pt.z.abs() > f64::EPSILON {
                        found_non_origin = true;
                    }
                }
            }
        }
        assert!(
            found_non_origin,
            "fixture {name}: expected at least one instance at a non-origin transform"
        );
    }
}

// -------------------------------------------------------------------------
// PCURVE / SURFACE_CURVE collection (W-C.1)
// -------------------------------------------------------------------------

#[test]
fn cylinder_ap214_is_collects_pcurves() {
    let src = include_str!("fixtures/cylinder_ap214_is.step");
    let graph = step_io::parse(src).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    assert!(
        result.warnings.is_empty(),
        "unexpected warnings: {:#?}",
        result.warnings
    );
    let model = result.model;

    // Cylinder fixture has 6 PCURVE entities (2 on the seam, 2 on each of
    // two surface curves). All six should land in Edge.pcurves.
    let total_pcurves: usize = model.topology.edges.iter().map(|e| e.pcurves.len()).sum();
    assert_eq!(total_pcurves, 6, "cylinder_ap214_is total pcurves");

    // 2D arenas populated.
    assert!(!model.geometry.curves_2d.is_empty(), "curves_2d empty");
    assert!(!model.geometry.points_2d.is_empty(), "points_2d empty");
    assert!(
        !model.geometry.directions_2d.is_empty(),
        "directions_2d empty"
    );

    // Sanity: at least one edge has pcurves, and the basis_surface is a
    // valid SurfaceId inside the 3D surface arena.
    let first = model
        .topology
        .edges
        .iter()
        .find(|e| !e.pcurves.is_empty())
        .expect("at least one edge has pcurves");
    let basis = first.pcurves[0].basis_surface;
    assert!(
        (basis.0 as usize) < model.geometry.surfaces.len(),
        "basis_surface out of range"
    );
}

#[test]
fn loft_ap214_is_collects_pcurves() {
    let src = include_str!("fixtures/loft_ap214_is.step");
    let graph = step_io::parse(src).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    assert!(
        result.warnings.is_empty(),
        "unexpected warnings: {:#?}",
        result.warnings
    );
    let model = result.model;

    // Loft fixture is much richer; we just require "many" pcurves and a
    // non-empty 2D curve arena including NURBS 2D entries.
    let total_pcurves: usize = model.topology.edges.iter().map(|e| e.pcurves.len()).sum();
    assert!(
        total_pcurves > 20,
        "loft should have many pcurves; got {total_pcurves}"
    );
    assert!(!model.geometry.curves_2d.is_empty());

    let has_2d_nurbs = model
        .geometry
        .curves_2d
        .iter()
        .any(|c| matches!(c, step_io::ir::geometry::Curve2d::Nurbs(_)));
    assert!(has_2d_nurbs, "loft should include 2D NURBS curves");
}

#[test]
fn external_temp_screw_parses() {
    // Sourced from an external sample (OCCT screw.step) as an
    // `external_temp_` placeholder per the test-fixture policy. Replace
    // with a hand-crafted fixture once one is available.
    use step_io::ir::geometry::{Curve2d, NurbsKind2d};

    let src = include_str!("fixtures/external_temp_screw.step");
    let graph = step_io::parse(src).expect("external_temp_screw.step parses");
    let result = ReaderContext::convert(&graph);

    // Pass4aRational populates curves_2d with at least one rational NURBS.
    let rational_2d_count = result
        .model
        .geometry
        .curves_2d
        .iter()
        .filter(|c| {
            matches!(
                c,
                Curve2d::Nurbs(n) if matches!(n.kind, NurbsKind2d::Rational { .. })
            )
        })
        .count();
    assert!(
        rational_2d_count > 0,
        "expected at least one 2D rational NURBS, got {rational_2d_count}"
    );
}

#[test]
fn pcurve_fixtures_convert_without_warnings() {
    const FIXTURES: &[(&str, &str)] = &[
        (
            "cylinder_ap214_is",
            include_str!("fixtures/cylinder_ap214_is.step"),
        ),
        ("loft_ap214_is", include_str!("fixtures/loft_ap214_is.step")),
    ];
    for (name, src) in FIXTURES {
        let graph =
            step_io::parse(src).unwrap_or_else(|e| panic!("fixture {name} parse failed: {e}"));
        let result = ReaderContext::convert(&graph);
        assert!(
            result.warnings.is_empty(),
            "fixture {name}: unexpected warnings: {:#?}",
            result.warnings
        );
        let total: usize = result
            .model
            .topology
            .edges
            .iter()
            .map(|e| e.pcurves.len())
            .sum();
        assert!(
            total > 0,
            "fixture {name}: expected at least one pcurve collected"
        );
    }
}

/// `DATUM_FEATURE` reads into the `pmi` pool as a `shape_aspect` subtype.
/// The NIST property fixture carries 4 `DATUM_FEATURE` and 6 `DATUM`, all
/// with an `of_shape` that resolves through the product chain.
#[test]
fn nist_property_def_datum_feature_pool() {
    let source = include_str!("fixtures/external_temp_nist_property_def.stp");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let pmi = result
        .model
        .pmi
        .as_ref()
        .expect("fixture carries PMI content");
    assert_eq!(pmi.datum_features.len(), 4, "DATUM_FEATURE count");
    assert_eq!(pmi.datums.len(), 6, "DATUM count");
}

/// `geometric_tolerance` form tolerances read into the `pmi` pool. The NIST
/// property fixture carries 3 `FLATNESS_TOLERANCE`, each with a plain
/// `LENGTH_MEASURE_WITH_UNIT` magnitude and a `DATUM_FEATURE` target — both
/// resolvable, so all three round-trip into the arena.
#[test]
fn nist_property_def_geometric_tolerances() {
    use step_io::ir::pmi::{GeometricTolerance, ToleranceMagnitude};
    let source = include_str!("fixtures/external_temp_nist_property_def.stp");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let pmi = result
        .model
        .pmi
        .as_ref()
        .expect("fixture carries PMI content");
    assert_eq!(
        pmi.geometric_tolerances.len(),
        3,
        "FLATNESS_TOLERANCE count"
    );
    for gt in pmi.geometric_tolerances.iter() {
        let GeometricTolerance::Flatness(data) = gt else {
            panic!("expected Flatness variant, got {gt:?}");
        };
        assert!(
            matches!(data.magnitude, ToleranceMagnitude::MeasureWithUnit(_)),
            "NIST flatness magnitude is a plain LENGTH_MEASURE_WITH_UNIT"
        );
    }
}

/// `general_datum_reference` form entities read into the `pmi` pool. The
/// NIST property fixture carries 19 `DATUM_REFERENCE_COMPARTMENT`, each with
/// a `base` pointing at a `DATUM` and an empty `modifiers` set.
#[test]
fn nist_property_def_general_datum_references() {
    use step_io::ir::pmi::{GeneralDatumBase, GeneralDatumReference};
    let source = include_str!("fixtures/external_temp_nist_property_def.stp");
    let graph = step_io::parse(source).expect("parse failed");
    let result = ReaderContext::convert(&graph);
    let pmi = result
        .model
        .pmi
        .as_ref()
        .expect("fixture carries PMI content");
    assert_eq!(
        pmi.general_datum_references.len(),
        19,
        "DATUM_REFERENCE_COMPARTMENT count"
    );
    for gdr in pmi.general_datum_references.iter() {
        let GeneralDatumReference::Compartment(data) = gdr else {
            panic!("expected Compartment variant, got {gdr:?}");
        };
        let GeneralDatumBase::Datum(_) = data.base;
    }
}
