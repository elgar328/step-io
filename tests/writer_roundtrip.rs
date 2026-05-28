//! Real-fixture round-trip tests for the writer.
//!
//! W-B.2 covers the six single-part `ap214_is` fixtures that don't include
//! PCURVE-family entities: `box`, `cone`, `ellipse`, `fillet_box`,
//! `revolution`, `torus`, `tapered_box`. `loft` and `cylinder` are
//! W-C (PCURVE).

#![allow(clippy::too_many_lines)]

use step_io::ir::assembly::{GeometryLeaf, WireframeReprKind};
use step_io::ir::property::{CharacterizedDefinition, PropertyDefinition};
use step_io::ir::units::NamedUnit;
use step_io::parse;
use step_io::reader::ReaderContext;

/// units-2: `NamedUnitId` values aren't stable across round-trip because
/// the writer's 2-pass emit may not preserve the original arena ordering
/// (e.g. CBU(METRE) self-wrap pulls its base SI in front of the outer).
/// Compare the *resolved* unit values + uncertainty via arena lookup
/// instead of the raw IDs.
fn assert_unit_contexts_equivalent(
    name: &str,
    lhs: &step_io::ir::model::StepModel,
    rhs: &step_io::ir::model::StepModel,
) {
    assert_eq!(lhs.units.len(), rhs.units.len(), "{name}: units count");
    let lpool = lhs.units_pool.as_ref().expect("lhs units pool");
    let rpool = rhs.units_pool.as_ref().expect("rhs units pool");
    for (lc, rc) in lhs.units.iter().zip(rhs.units.iter()) {
        // Compare semantic content (unit enum + flag bits); skip `cbu_base`
        // since it's an arena ref whose absolute value depends on emit order.
        let l_len = match lpool.named_units[lc.length] {
            NamedUnit::Length(f) => (f.unit, f.cbu_base.is_some()),
            _ => panic!("{name}: lhs length slot is not Length"),
        };
        let r_len = match rpool.named_units[rc.length] {
            NamedUnit::Length(f) => (f.unit, f.cbu_base.is_some()),
            _ => panic!("{name}: rhs length slot is not Length"),
        };
        assert_eq!(l_len, r_len, "{name}: length");
        let l_pa = match lpool.named_units[lc.plane_angle] {
            NamedUnit::PlaneAngle(f) => (f.unit, f.cbu_base.is_some()),
            _ => panic!("{name}: lhs plane_angle slot is not PlaneAngle"),
        };
        let r_pa = match rpool.named_units[rc.plane_angle] {
            NamedUnit::PlaneAngle(f) => (f.unit, f.cbu_base.is_some()),
            _ => panic!("{name}: rhs plane_angle slot is not PlaneAngle"),
        };
        assert_eq!(l_pa, r_pa, "{name}: plane_angle");
        let l_solid = match lpool.named_units[lc.solid_angle] {
            NamedUnit::SolidAngle(f) => f.unit,
            _ => panic!("{name}: lhs solid_angle slot is not SolidAngle"),
        };
        let r_solid = match rpool.named_units[rc.solid_angle] {
            NamedUnit::SolidAngle(f) => f.unit,
            _ => panic!("{name}: rhs solid_angle slot is not SolidAngle"),
        };
        assert_eq!(l_solid, r_solid, "{name}: solid_angle");
        assert_eq!(
            lc.length_uncertainty, rc.length_uncertainty,
            "{name}: length_uncertainty"
        );
        assert_eq!(
            lc.plane_angle_uncertainty, rc.plane_angle_uncertainty,
            "{name}: plane_angle_uncertainty"
        );
        assert_eq!(
            lc.solid_angle_uncertainty, rc.solid_angle_uncertainty,
            "{name}: solid_angle_uncertainty"
        );
    }
}

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
        re.geometry.vertices.len(),
        original.geometry.vertices.len(),
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

    assert_unit_contexts_equivalent(name, &re, &original);
    assert_eq!(re.schema, original.schema, "{name}: schema preserved");
    assert_eq!(re.header, original.header, "{name}: header");
    assert_eq!(
        re.visualization, original.visualization,
        "{name}: visualization"
    );
    assert_eq!(re.properties, original.properties, "{name}: properties");
    assert_eq!(
        re.shape_aspects.iter().collect::<Vec<_>>(),
        original.shape_aspects.iter().collect::<Vec<_>>(),
        "{name}: shape_aspects"
    );

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
    assert_eq!(o_asm.roots, r_asm.roots, "{name}: roots");
    for (pidx, (op, rp)) in o_asm.products.iter().zip(r_asm.products.iter()).enumerate() {
        assert_eq!(op.id, rp.id, "{name}: product[{pidx}] id");
        assert_eq!(op.name, rp.name, "{name}: product[{pidx}] name");
        assert_eq!(
            op.description, rp.description,
            "{name}: product[{pidx}] description"
        );
        assert_eq!(op.category, rp.category, "{name}: product[{pidx}] category");
        assert_eq!(
            op.formation_with_source, rp.formation_with_source,
            "{name}: product[{pidx}] formation_with_source"
        );
        match (&op.geometry, &rp.geometry) {
            (Some(GeometryLeaf::Solid(_)), Some(GeometryLeaf::Solid(_))) => {}
            (Some(GeometryLeaf::SurfaceBody(o)), Some(GeometryLeaf::SurfaceBody(r))) => {
                assert_eq!(
                    o.ids.len(),
                    r.ids.len(),
                    "{name}: product[{pidx}] surface body shells"
                );
            }
            (Some(GeometryLeaf::Wireframe(o)), Some(GeometryLeaf::Wireframe(r))) => {
                assert_eq!(
                    o.repr_kind, r.repr_kind,
                    "{name}: product[{pidx}] wireframe repr_kind"
                );
                assert_eq!(
                    o.curves.len(),
                    r.curves.len(),
                    "{name}: product[{pidx}] wireframe curves count"
                );
                assert_eq!(
                    o.points.len(),
                    r.points.len(),
                    "{name}: product[{pidx}] wireframe points count"
                );
            }
            (None, None) => {
                assert_eq!(
                    op.instances.len(),
                    rp.instances.len(),
                    "{name}: product[{pidx}] instance count"
                );
                for (iidx, (o, r)) in op.instances.iter().zip(rp.instances.iter()).enumerate() {
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
        matches!(product.geometry, Some(GeometryLeaf::SurfaceBody(_))),
        "expected SurfaceBody, got {:?}",
        product.geometry,
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

#[test]
fn wire1_ap214_is_round_trips() {
    assert_fixture_round_trip("wire1", include_str!("fixtures/wire1_ap214_is.stp"));
}

#[test]
fn wire2_ap214_is_round_trips() {
    assert_fixture_round_trip("wire2", include_str!("fixtures/wire2_ap214_is.step"));
}

/// CATIA wire1 fixture exercises the `..._SURFACE_...` flavour (GBSSR) plus
/// `COMPOSITE_CURVE` / `TRIMMED_CURVE` chains. The reader must classify the
/// product as Wireframe with `repr_kind == Surface`, the writer must round-
/// trip the GBSSR + `GEOMETRIC_SET` pair, and the indirect SR pattern (SDR ->
/// plain SR -> SRR -> GBSSR) must be preserved via `outer_sr_frame`.
#[test]
fn wire1_emits_geometric_set_and_gbssr() {
    let src = include_str!("fixtures/wire1_ap214_is.stp");
    let graph = parse(src).expect("parse fixture");
    let model = ReaderContext::convert(&graph).model;

    let tree = model.assembly.as_ref().expect("assembly present");
    let product = tree.products.iter().next().expect("one product");
    let Some(GeometryLeaf::Wireframe(wf)) = &product.geometry else {
        panic!("expected Wireframe, got {:?}", product.geometry);
    };
    assert_eq!(wf.repr_kind, WireframeReprKind::Surface);
    assert!(!wf.curves.is_empty(), "expected wireframe curves");
    assert!(
        product.outer_sr_frame.is_some(),
        "wire1 uses indirect SR pattern; outer_sr_frame must be set"
    );

    let out = model.write_to_string().expect("write");
    assert!(out.contains("GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION"));
    assert!(out.contains("GEOMETRIC_SET"));
    assert!(out.contains("COMPOSITE_CURVE"));
    assert!(out.contains("TRIMMED_CURVE"));
    assert!(
        out.contains("SHAPE_REPRESENTATION_RELATIONSHIP"),
        "indirect SR wrapper must round-trip"
    );
}

/// `FreeCAD` wire2 fixture exercises the `..._WIREFRAME_...` flavour (GBWSR)
/// inside a multi-product assembly. Two of the three leaves are wireframe
/// products; one is a `SurfaceBody`. Verifies the writer emits GBWSR with
/// `GEOMETRIC_CURVE_SET` and that PARAMETER-master `TRIMMED_CURVE` round-trips.
#[test]
fn wire2_emits_gbwsr_in_assembly() {
    let src = include_str!("fixtures/wire2_ap214_is.step");
    let graph = parse(src).expect("parse fixture");
    let model = ReaderContext::convert(&graph).model;

    let tree = model.assembly.as_ref().expect("assembly present");
    assert_eq!(tree.products.len(), 4, "root + 3 leaves");
    let mut wireframe_count = 0_usize;
    let mut surface_body_count = 0_usize;
    let mut group_count = 0_usize;
    for p in tree.products.iter() {
        match &p.geometry {
            Some(GeometryLeaf::Wireframe(wf)) => {
                assert_eq!(wf.repr_kind, WireframeReprKind::Wireframe);
                wireframe_count += 1;
            }
            Some(GeometryLeaf::SurfaceBody(_)) => surface_body_count += 1,
            None => group_count += 1,
            Some(GeometryLeaf::Solid(_)) => {}
        }
    }
    assert_eq!(wireframe_count, 2);
    assert_eq!(surface_body_count, 1);
    assert_eq!(group_count, 1);

    let out = model.write_to_string().expect("write");
    assert!(out.contains("GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION"));
    assert!(out.contains("GEOMETRIC_CURVE_SET"));
    assert!(out.contains("TRIMMED_CURVE"));
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

/// AP203 fixture pairs `PRPC.name = "detail"` with a supertype
/// `PC.name = "part"` — verifies the IR keeps both names so the writer
/// reproduces the chain faithfully. Also confirms the AP203
/// `_WITH_SPECIFIED_SOURCE` formation subtype is preserved via the
/// loyalty flag.
#[test]
fn box_ap203_preserves_product_category_chain() {
    use step_io::ir::assembly::ProductCategory;
    let src = include_str!("fixtures/box_ap203.step");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    let tree = model.assembly.as_ref().expect("assembly present");
    let product = tree.products.iter().next().expect("one product");
    let prpc = tree
        .product_categories
        .iter()
        .find_map(|pc| match pc {
            ProductCategory::ProductRelatedProductCategory(d) => Some(d),
            ProductCategory::Itself(_) => None,
        })
        .expect("AP203 fixture must carry a PRPC entry");
    assert_eq!(prpc.name, "detail");
    let pc = tree
        .product_categories
        .iter()
        .find_map(|pc| match pc {
            ProductCategory::Itself(d) => Some(d),
            ProductCategory::ProductRelatedProductCategory(_) => None,
        })
        .expect("AP203 fixture has PCR + PC");
    assert_eq!(pc.name, "part");
    assert_eq!(pc.description, None);
    assert!(
        product.formation_with_source,
        "AP203 mandates _WITH_SPECIFIED_SOURCE"
    );
}

/// `box_ap214_is.step` has a single MDGPR with one `STYLED_ITEM` bound to
/// the `MANIFOLD_SOLID_BREP` plus a single `COLOUR_RGB`. Verifies the
/// passive tree-inline IR captures the chain correctly down to the RGB
/// triple.
#[test]
fn box_ap214_is_preserves_visualization() {
    use step_io::ir::RepresentationItemRef;
    use step_io::ir::visualization::SurfaceSideStyleEntry;
    let model =
        ReaderContext::convert(&parse(include_str!("fixtures/box_ap214_is.step")).expect("parse"))
            .model;
    let viz = model.visualization.expect("visualization present");
    assert_eq!(viz.mdgprs.len(), 1);
    let mdgpr = &viz.mdgprs[0];
    assert_eq!(mdgpr.items.len(), 1);
    let step_io::ir::visualization::StyledItem::Plain(si) = &viz.styled_items[mdgpr.items[0]]
    else {
        panic!("expected Plain StyledItem variant");
    };
    assert!(
        matches!(si.item, RepresentationItemRef::Solid(_)),
        "STYLED_ITEM should bind to a Solid, got {:?}",
        si.item
    );
    assert_eq!(si.styles.len(), 1);
    let step_io::ir::visualization::PresentationStyleAssignment::Itself(psa_data) =
        &viz.presentation_style_assignments[si.styles[0]]
    else {
        panic!("expected Itself PSA variant");
    };
    // The PSA carries a SurfaceStyleUsage plus a CurveStyle reference; pick
    // the surface entry out by variant.
    let ssu_id = psa_data
        .styles
        .iter()
        .find_map(|s| match s {
            step_io::ir::visualization::PsaStyle::Surface(id) => Some(*id),
            step_io::ir::visualization::PsaStyle::Curve(_)
            | step_io::ir::visualization::PsaStyle::Null => None,
        })
        .expect("PSA carries a SurfaceStyleUsage");
    let step_io::ir::visualization::FoundedItem::SurfaceStyleUsage(ssu) =
        &viz.founded_items[ssu_id]
    else {
        panic!("expected SurfaceStyleUsage founded-item variant");
    };
    let step_io::ir::visualization::FoundedItem::SurfaceSideStyle(sss) =
        &viz.founded_items[ssu.style]
    else {
        panic!("expected SurfaceSideStyle founded-item variant");
    };
    let entry = &sss.styles[0];
    let SurfaceSideStyleEntry::FillArea(ssfa_id) = entry else {
        panic!("expected FillArea entry, got {entry:?}");
    };
    let step_io::ir::visualization::FoundedItem::SurfaceStyleFillArea(ssfa) =
        &viz.founded_items[*ssfa_id]
    else {
        panic!("expected SurfaceStyleFillArea founded-item variant");
    };
    let step_io::ir::visualization::FoundedItem::FillAreaStyle(fas) =
        &viz.founded_items[ssfa.fill_area]
    else {
        panic!("expected FillAreaStyle founded-item variant");
    };
    let colour_id = fas.fill_styles[0].colour;
    let step_io::ir::visualization::Colour::Rgb(color) = &viz.colours[colour_id] else {
        panic!("expected Rgb colour variant");
    };
    assert!((color.red - 0.678).abs() < 0.01);
    assert!((color.green - 0.710).abs() < 0.01);
    assert!((color.blue - 0.741).abs() < 0.01);
}

/// ABC-tier fixture (temporarily borrowed from
/// `step-io-reference-check/fixtures/abc/00009954_*.step` — the user plans
/// to replace it with a hand-curated minimal fixture later, hence the
/// `external_temp_` prefix). ABC files emit explicit `DIMENSIONAL_EXPONENTS`
/// references in plain SI unit complexes' `NAMED_UNIT.dimensions` slot;
/// units-3b normalises that surface form to `*` Derived on emit so the
/// surface DE entities disappear. The IR-level round-trip must still
/// produce the same `LengthUnit` / `AngleUnit` / `SolidAngleUnit` in the context.
#[test]
fn external_temp_abc_explicit_de_round_trip() {
    let src = include_str!("fixtures/external_temp_abc_explicit_de.step");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    let text = model.write_to_string().expect("write");
    let back = ReaderContext::convert(&parse(&text).expect("re-parse")).model;
    assert_unit_contexts_equivalent("abc_explicit_de", &model, &back);
}

/// `STEPCode` `AP214e3` fixture (`io1-cm-214.stp`, temporarily borrowed —
/// `external_temp_` prefix) carrying three `POLYLINE` entities, each with
/// two `CARTESIAN_POINT` refs. Verifies the 3D polyline handler round-trips
/// the IR `Curve::Polyline` variant.
#[test]
fn external_temp_polyline_round_trip() {
    let src = include_str!("fixtures/external_temp_polyline.step");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    let polyline_count = model
        .geometry
        .curves
        .iter()
        .filter(|c| matches!(c, step_io::ir::geometry::Curve::Polyline(_)))
        .count();
    assert_eq!(polyline_count, 3, "fixture must yield three polylines");
    let text = model.write_to_string().expect("write");
    let back = ReaderContext::convert(&parse(&text).expect("re-parse")).model;
    let back_count = back
        .geometry
        .curves
        .iter()
        .filter(|c| matches!(c, step_io::ir::geometry::Curve::Polyline(_)))
        .count();
    assert_eq!(back_count, 3, "round-trip preserves polyline count");
}

/// Multi-context Fusion 360 fixture (temporarily borrowed from
/// `step-io-reference-check/fixtures/fusion360/32879_49552f2f_3.stp` — the
/// user plans to replace it with a hand-curated minimal fixture later,
/// hence the `external_temp_` filename prefix). The fixture carries two
/// distinct `REPRESENTATION_CONTEXT` entities (one referenced by ABSR, one
/// by MDGPR) that happen to share unit values — this test asserts the
/// arena preserves both as separate entries and the round-trip survives.
#[test]
fn external_temp_fusion360_two_context_round_trip() {
    let src = include_str!("fixtures/external_temp_fusion360_two_context.stp");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    assert_eq!(
        model.units.len(),
        2,
        "fusion fixture must yield two distinct unit contexts"
    );
    let text = model.write_to_string().expect("write");
    let back = ReaderContext::convert(&parse(&text).expect("re-parse")).model;
    assert_eq!(
        back.units.len(),
        2,
        "writer must emit both unit contexts, re-read should see two"
    );
    // The MDGPR's `context` field should differ from the products' shape
    // contexts — Fusion 360's geometry vs. visualization split.
    let viz = back.visualization.as_ref().expect("MDGPR present");
    let mdgpr = viz.mdgprs.first().expect("at least one MDGPR");
    let Some(step_io::ir::RepresentationContextRef::Unitful(mdgpr_ctx)) = mdgpr.context else {
        panic!("MDGPR carries a unitful context after Commit 2");
    };
    let assembly = back.assembly.as_ref().expect("assembly present");
    let product = assembly
        .products
        .iter()
        .next()
        .expect("at least one product");
    let product_ctx = product
        .geometry_context
        .expect("Product carries a geometry context after Commit 2");
    assert_ne!(
        mdgpr_ctx, product_ctx,
        "geometry and visualization should reference distinct contexts"
    );
}

/// CATIA wire1 fixture has multi-product PC sharing plus
/// `PC.description = "specification"`. Verifies the IR captures the
/// non-empty PC description and that CATIA's AP214 IS export with
/// `_WITH_SPECIFIED_SOURCE` flips the loyalty flag.
#[test]
fn wire1_preserves_pc_chain_with_specification() {
    use step_io::ir::assembly::ProductCategory;
    let src = include_str!("fixtures/wire1_ap214_is.stp");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    let tree = model.assembly.as_ref().expect("assembly present");
    let product = tree.products.iter().next().expect("at least one product");
    let prpc = tree
        .product_categories
        .iter()
        .find_map(|pc| match pc {
            ProductCategory::ProductRelatedProductCategory(d) => Some(d),
            ProductCategory::Itself(_) => None,
        })
        .expect("CATIA fixture carries a PRPC entry");
    assert_eq!(prpc.name, "part");
    let pc_with_spec = tree.product_categories.iter().find_map(|pc| match pc {
        ProductCategory::Itself(d) if d.description.as_deref() == Some("specification") => Some(d),
        _ => None,
    });
    assert!(
        pc_with_spec.is_some(),
        "CATIA fixture carries a PC with description='specification'"
    );
    assert!(
        product.formation_with_source,
        "CATIA AP214 IS uses _WITH_SPECIFIED_SOURCE"
    );
}

/// PMI scaffolding reader check — same NIST fixture as the property test
/// reader-only check below. Verifies the reader populates
/// `model.shape_aspects` with at least one entry. Round-trip is not
/// asserted because the fixture's 2-unit GUAC keeps the assembly pass
/// from emitting a product chain (`product_def_shape_ids` cache empty →
/// `SHAPE_ASPECT` emit silent skip — see plan R1).
#[test]
fn external_temp_nist_shape_aspect_reader_only() {
    let src = include_str!("fixtures/external_temp_nist_property_def.stp");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    assert!(
        !model.shape_aspects.is_empty(),
        "at least one SHAPE_ASPECT parsed"
    );
}

/// NIST AP242 fixture (temporarily borrowed from
/// `step-io-reference-check/fixtures/nist/ap242/nist_stc_06_asme1_ap242-e3.stp`
/// — the user plans to replace it with a hand-curated minimal fixture
/// later, hence the `external_temp_` prefix). Carries hundreds of
/// `PROPERTY_DEFINITION` user-defined attributes (Pattern A — target =
/// `PRODUCT_DEFINITION`) plus a handful of geometric validation properties
/// (Pattern B — target = `SHAPE_ASPECT`, dropped at read).
///
/// This fixture's `GLOBAL_UNIT_ASSIGNED_CONTEXT` carries only two unit
/// refs (length + `plane_angle`, no `solid_angle`) so step-io's strict unit
/// context builder rejects it — `model.units` ends up empty and the
/// product chain is silently skipped on emit. As a result this test
/// exercises only the reader: it confirms user-defined attributes flow
/// from the source file into `model.properties`. Round-trip preservation
/// is measured by `step-io-reference-check` against fixtures whose unit
/// contexts are complete; once a hand-curated minimal fixture lands here
/// this test can be tightened to deep-equality round-trip.
#[test]
fn external_temp_nist_property_def_reader_only() {
    let src = include_str!("fixtures/external_temp_nist_property_def.stp");
    let model = ReaderContext::convert(&parse(src).expect("parse")).model;
    let pool = model
        .properties
        .as_ref()
        .expect("user-defined attribute chain present");
    assert!(
        !pool.properties.is_empty(),
        "at least one user-defined attribute parsed"
    );
    // Spot-check: NIST stc_06 has 'p1' .. 'pN'. The reader must surface
    // these by name, not collapse them into anonymous entries.
    assert!(
        pool.properties.iter().any(|p| p.name == "p1"),
        "expected a property named 'p1' in the fixture"
    );
    // Pattern B coverage: at least one `PROPERTY_DEFINITION` in the
    // fixture targets a `SHAPE_ASPECT` (geometric validation property).
    // Reader must surface it as a `CharacterizedDefinition::ShapeAspect`
    // arena entry rather than silently dropping it.
    let sa_pd_count = pool
        .property_definitions
        .iter()
        .filter(|pd| {
            matches!(
                pd,
                PropertyDefinition::Itself(d)
                    if matches!(d.definition, CharacterizedDefinition::ShapeAspect(_))
            )
        })
        .count();
    assert!(
        sa_pd_count >= 1,
        "expected at least one SHAPE_ASPECT-targeted PropertyDefinition"
    );
}
