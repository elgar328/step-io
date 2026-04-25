//! Real-fixture round-trip tests for the writer.
//!
//! W-B.2 covers the six single-part `ap214_is` fixtures that don't include
//! PCURVE-family entities: `box`, `cone`, `ellipse`, `fillet_box`,
//! `revolution`, `torus`, `tapered_box`. `loft` and `cylinder` are
//! W-C (PCURVE).

#![allow(clippy::too_many_lines)]

use step_io::ir::assembly::ProductContent;
use step_io::parse;
use step_io::reader::ReaderContext;

fn assert_fixture_round_trip(name: &str, src: &str) {
    let original = {
        let graph = parse(src).unwrap_or_else(|e| panic!("{name}: fixture parses: {e}"));
        ReaderContext::convert(&graph).model
    };
    let text = original
        .write_to_string()
        .unwrap_or_else(|e| panic!("{name}: write failed: {e}"));
    let graph2 = parse(&text).unwrap_or_else(|e| panic!("{name}: writer output parses: {e}"));
    let result = ReaderContext::convert(&graph2);

    assert!(
        result.warnings.is_empty(),
        "{name}: reader warnings on writer output: {:#?}",
        result.warnings
    );
    let re = result.model;

    // Topology counts — IR-authoritative structures.
    assert_eq!(
        re.topology.solids.len(),
        original.topology.solids.len(),
        "{name}: solids count"
    );
    assert_eq!(
        re.topology.shells.len(),
        original.topology.shells.len(),
        "{name}: shells count"
    );
    assert_eq!(
        re.topology.faces.len(),
        original.topology.faces.len(),
        "{name}: faces count"
    );
    assert_eq!(
        re.topology.wires.len(),
        original.topology.wires.len(),
        "{name}: wires count"
    );
    assert_eq!(
        re.topology.edges.len(),
        original.topology.edges.len(),
        "{name}: edges count"
    );
    assert_eq!(
        re.topology.vertices.len(),
        original.topology.vertices.len(),
        "{name}: vertices count"
    );

    // Geometry counts.
    assert_eq!(
        re.geometry.points.len(),
        original.geometry.points.len(),
        "{name}: points count"
    );
    assert_eq!(
        re.geometry.directions.len(),
        original.geometry.directions.len(),
        "{name}: directions count"
    );
    assert_eq!(
        re.geometry.curves.len(),
        original.geometry.curves.len(),
        "{name}: curves count"
    );
    assert_eq!(
        re.geometry.surfaces.len(),
        original.geometry.surfaces.len(),
        "{name}: surfaces count"
    );

    // 2D geometry (PCURVE parametric space) counts
    assert_eq!(
        re.geometry.points_2d.len(),
        original.geometry.points_2d.len(),
        "{name}: points_2d count"
    );
    assert_eq!(
        re.geometry.directions_2d.len(),
        original.geometry.directions_2d.len(),
        "{name}: directions_2d count"
    );
    assert_eq!(
        re.geometry.curves_2d.len(),
        original.geometry.curves_2d.len(),
        "{name}: curves_2d count"
    );

    // Per-edge pcurves
    for (eidx, (oe, re_edge)) in original
        .topology
        .edges
        .iter()
        .zip(re.topology.edges.iter())
        .enumerate()
    {
        assert_eq!(
            oe.pcurves.len(),
            re_edge.pcurves.len(),
            "{name}: edge[{eidx}] pcurves len"
        );
        for (pidx, (op, rp)) in oe.pcurves.iter().zip(re_edge.pcurves.iter()).enumerate() {
            assert_eq!(
                op.basis_surface, rp.basis_surface,
                "{name}: edge[{eidx}].pcurve[{pidx}] basis_surface"
            );
            assert_eq!(
                op.curve_2d, rp.curve_2d,
                "{name}: edge[{eidx}].pcurve[{pidx}] curve_2d"
            );
        }
    }

    assert_eq!(re.units, original.units, "{name}: units");
    assert_eq!(re.schema, original.schema, "{name}: schema preserved");
    assert_eq!(re.header, original.header, "{name}: header");

    // Product metadata preserved.
    let o_asm = original
        .assembly
        .as_ref()
        .unwrap_or_else(|| panic!("{name}: original has assembly"));
    let r_asm = re
        .assembly
        .as_ref()
        .unwrap_or_else(|| panic!("{name}: round-tripped has assembly"));
    assert_eq!(
        o_asm.products.len(),
        r_asm.products.len(),
        "{name}: products count"
    );
    assert_eq!(o_asm.root, r_asm.root, "{name}: root");
    for (pidx, (op, rp)) in o_asm.products.iter().zip(r_asm.products.iter()).enumerate() {
        assert_eq!(op.id, rp.id, "{name}: product[{pidx}] id");
        assert_eq!(op.name, rp.name, "{name}: product[{pidx}] name");
        assert_eq!(
            op.description, rp.description,
            "{name}: product[{pidx}] description"
        );
        match (&op.content, &rp.content) {
            (ProductContent::Solid(_), ProductContent::Solid(_)) => {}
            (ProductContent::SurfaceBody(o), ProductContent::SurfaceBody(r)) => {
                assert_eq!(
                    o.len(),
                    r.len(),
                    "{name}: product[{pidx}] surface body shells"
                );
            }
            (ProductContent::Group(oi), ProductContent::Group(ri)) => {
                assert_eq!(oi.len(), ri.len(), "{name}: product[{pidx}] instance count");
                for (iidx, (o, r)) in oi.iter().zip(ri.iter()).enumerate() {
                    assert_eq!(
                        o.child, r.child,
                        "{name}: product[{pidx}].instance[{iidx}].child"
                    );
                    assert_eq!(
                        o.occurrence_id, r.occurrence_id,
                        "{name}: product[{pidx}].instance[{iidx}].occurrence_id"
                    );
                    assert_eq!(
                        o.occurrence_name, r.occurrence_name,
                        "{name}: product[{pidx}].instance[{iidx}].occurrence_name"
                    );
                    assert_eq!(
                        o.transform, r.transform,
                        "{name}: product[{pidx}].instance[{iidx}].transform"
                    );
                }
            }
            _ => panic!("{name}: product[{pidx}] content variant mismatch"),
        }
    }

    // Sanity sample: first point coordinates preserved bit-for-bit.
    let o_pt = original.geometry.points.iter().next().unwrap();
    let r_pt = re.geometry.points.iter().next().unwrap();
    assert!(
        (o_pt.x - r_pt.x).abs() < f64::EPSILON,
        "{name}: first point x"
    );
    assert!(
        (o_pt.y - r_pt.y).abs() < f64::EPSILON,
        "{name}: first point y"
    );
    assert!(
        (o_pt.z - r_pt.z).abs() < f64::EPSILON,
        "{name}: first point z"
    );

    // Sanity sample: solid name round-trips. Skipped for surface-body fixtures
    // where the solids arena is empty.
    if let (Some(o_sol), Some(r_sol)) = (
        original.topology.solids.iter().next(),
        re.topology.solids.iter().next(),
    ) {
        assert_eq!(o_sol.name, r_sol.name, "{name}: solid name");
    }
}

#[test]
fn box_ap214_is_round_trips() {
    assert_fixture_round_trip("box", include_str!("fixtures/box_ap214_is.step"));
}

#[test]
fn cone_ap214_is_round_trips() {
    assert_fixture_round_trip("cone", include_str!("fixtures/cone_ap214_is.step"));
}

#[test]
fn ellipse_ap214_is_round_trips() {
    assert_fixture_round_trip("ellipse", include_str!("fixtures/ellipse_ap214_is.step"));
}

#[test]
fn fillet_box_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "fillet_box",
        include_str!("fixtures/fillet_box_ap214_is.step"),
    );
}

#[test]
fn hemisphere_tube_ap242_dis_round_trips() {
    assert_fixture_round_trip(
        "hemisphere_tube",
        include_str!("fixtures/hemisphere_tube_ap242_dis.stp"),
    );
}

/// Guards the Fusion 360 / CATIA `SDR → plain SR → SRR → MSSR` indirection:
/// the reader must resolve the SRR so the product becomes a `SurfaceBody`
/// and capture the plain SR's frame in `outer_sr_frame`; the writer must in
/// turn emit the MSSR chain together with the plain SR + SRR wrapper.
#[test]
fn hemisphere_tube_emits_surface_body_chain() {
    let src = include_str!("fixtures/hemisphere_tube_ap242_dis.stp");
    let graph = parse(src).expect("parse fixture");
    let model = ReaderContext::convert(&graph).model;

    let tree = model.assembly.as_ref().expect("assembly present");
    let product = tree.products.iter().next().expect("one product");
    assert!(
        matches!(product.content, ProductContent::SurfaceBody(_)),
        "expected SurfaceBody, got {:?}",
        product.content,
    );
    assert!(
        product.outer_sr_frame.is_some(),
        "indirect SR pattern not preserved in IR",
    );

    let out = model.write_to_string().expect("write");
    assert!(
        out.contains("MANIFOLD_SURFACE_SHAPE_REPRESENTATION"),
        "writer output missing MSSR",
    );
    assert!(
        out.contains("SHELL_BASED_SURFACE_MODEL"),
        "writer output missing SBSM",
    );
    assert!(
        out.contains("OPEN_SHELL"),
        "writer output missing OPEN_SHELL",
    );
    assert!(
        out.contains("SHAPE_REPRESENTATION_RELATIONSHIP"),
        "writer output missing SRR (indirect SR pattern lost)",
    );
}

#[test]
fn revolution_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "revolution",
        include_str!("fixtures/revolution_ap214_is.step"),
    );
}

#[test]
fn torus_ap214_is_round_trips() {
    assert_fixture_round_trip("torus", include_str!("fixtures/torus_ap214_is.step"));
}

#[test]
fn tapered_box_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "tapered_box",
        include_str!("fixtures/tapered_box_ap214_is.step"),
    );
}

#[test]
fn face_surface_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "face_surface",
        include_str!("fixtures/face_surface_ap214_is.step"),
    );
}

#[test]
fn offset_surface_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "offset_surface",
        include_str!("fixtures/offset_surface_ap214_is.step"),
    );
}

// -------------------------------------------------------------------------
// box AP coverage — the only shape kept across all 5 AP versions so that the
// writer's AP-specific header / schema / product-chain paths stay exercised.
// -------------------------------------------------------------------------

#[test]
fn box_ap203_round_trips() {
    assert_fixture_round_trip("box_ap203", include_str!("fixtures/box_ap203.step"));
}

#[test]
fn box_ap214_cd_round_trips() {
    assert_fixture_round_trip("box_ap214_cd", include_str!("fixtures/box_ap214_cd.step"));
}

#[test]
fn box_ap214_dis_round_trips() {
    assert_fixture_round_trip("box_ap214_dis", include_str!("fixtures/box_ap214_dis.step"));
}

#[test]
fn box_ap242_dis_round_trips() {
    assert_fixture_round_trip("box_ap242_dis", include_str!("fixtures/box_ap242_dis.step"));
}

// -------------------------------------------------------------------------
// Edge cases (assembly / PCURVE / BREP_WITH_VOIDS) — ap214_is each.
// -------------------------------------------------------------------------

#[test]
fn assembly_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "assembly_ap214_is",
        include_str!("fixtures/assembly_ap214_is.step"),
    );
}

#[test]
fn cylinder_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "cylinder_ap214_is",
        include_str!("fixtures/cylinder_ap214_is.step"),
    );
}

#[test]
fn loft_ap214_is_round_trips() {
    assert_fixture_round_trip("loft_ap214_is", include_str!("fixtures/loft_ap214_is.step"));
}

#[test]
fn hollow_box_ap214_is_round_trips() {
    assert_fixture_round_trip(
        "hollow_box_ap214_is",
        include_str!("fixtures/hollow_box_ap214_is.step"),
    );
}

#[test]
fn hollow_box_ap214_is_preserves_void_orientation() {
    use step_io::Orientation;
    let src = include_str!("fixtures/hollow_box_ap214_is.step");
    let original = ReaderContext::convert(&parse(src).unwrap()).model;
    let text = original.write_to_string().expect("write");
    let re = ReaderContext::convert(&parse(&text).unwrap()).model;

    assert_eq!(re.topology.solids.len(), 1);
    let solid = re.topology.solids.iter().next().unwrap();
    assert_eq!(solid.shells.len(), 2, "1 outer + 1 void");

    let outer = &re
        .topology
        .shells
        .iter()
        .nth(solid.shells[0].0 as usize)
        .unwrap();
    let inner = &re
        .topology
        .shells
        .iter()
        .nth(solid.shells[1].0 as usize)
        .unwrap();
    assert_eq!(outer.orientation, Orientation::Forward);
    assert_eq!(inner.orientation, Orientation::Reversed);
}
