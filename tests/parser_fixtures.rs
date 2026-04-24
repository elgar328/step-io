//! Integration tests that parse every committed STEP fixture through the
//! full Part 21 pipeline (lexer → parser → `EntityGraph`).

use step_io::{Attribute, EntityGraph, RawEntity, SchemaClass, parse};

const FIXTURES: &[(&str, &str)] = &[
    // box — AP coverage (5 AP)
    ("box_ap203", include_str!("fixtures/box_ap203.step")),
    ("box_ap214_cd", include_str!("fixtures/box_ap214_cd.step")),
    ("box_ap214_dis", include_str!("fixtures/box_ap214_dis.step")),
    ("box_ap214_is", include_str!("fixtures/box_ap214_is.step")),
    ("box_ap242_dis", include_str!("fixtures/box_ap242_dis.step")),
    // shape coverage — ap214_is each
    (
        "fillet_box_ap214_is",
        include_str!("fixtures/fillet_box_ap214_is.step"),
    ),
    ("cone_ap214_is", include_str!("fixtures/cone_ap214_is.step")),
    (
        "torus_ap214_is",
        include_str!("fixtures/torus_ap214_is.step"),
    ),
    (
        "revolution_ap214_is",
        include_str!("fixtures/revolution_ap214_is.step"),
    ),
    (
        "tapered_box_ap214_is",
        include_str!("fixtures/tapered_box_ap214_is.step"),
    ),
    (
        "ellipse_ap214_is",
        include_str!("fixtures/ellipse_ap214_is.step"),
    ),
    // topology / assembly edge cases — ap214_is each
    (
        "cylinder_ap214_is",
        include_str!("fixtures/cylinder_ap214_is.step"),
    ),
    ("loft_ap214_is", include_str!("fixtures/loft_ap214_is.step")),
    (
        "hollow_box_ap214_is",
        include_str!("fixtures/hollow_box_ap214_is.step"),
    ),
    (
        "assembly_ap214_is",
        include_str!("fixtures/assembly_ap214_is.step"),
    ),
];

fn load(name: &str) -> EntityGraph {
    let (_, source) = FIXTURES
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("fixture {name} not found"));
    parse(source).unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"))
}

// ------------------------------------------------------------------
// Per-fixture schema identification
// ------------------------------------------------------------------

#[test]
fn box_ap203_schema() {
    let g = load("box_ap203");
    assert_eq!(g.schema.class(), Some(SchemaClass::Ap203));
    let raw = g.schema.raw().expect("raw preserved");
    assert!(
        raw.iter().any(|s| s.contains("CONFIG_CONTROL_DESIGN")),
        "AP203 raw missing CONFIG_CONTROL_DESIGN"
    );
}

#[test]
fn box_ap214_cd_schema() {
    let g = load("box_ap214_cd");
    assert_eq!(g.schema.class(), Some(SchemaClass::Ap214Cd));
    let raw = g.schema.raw().expect("raw preserved");
    assert!(raw.as_slice()[0].contains("AUTOMOTIVE_DESIGN_CC2"));
}

#[test]
fn box_ap214_dis_schema() {
    let g = load("box_ap214_dis");
    assert_eq!(g.schema.class(), Some(SchemaClass::Ap214Dis));
}

#[test]
fn box_ap214_is_schema() {
    let g = load("box_ap214_is");
    assert_eq!(g.schema.class(), Some(SchemaClass::Ap214Is));
}

#[test]
fn box_ap242_dis_schema() {
    let g = load("box_ap242_dis");
    assert_eq!(g.schema.class(), Some(SchemaClass::Ap242Dis));
    let raw = g.schema.raw().expect("raw preserved");
    assert!(raw.as_slice()[0].contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF"));
}

// ------------------------------------------------------------------
// All fixtures parse successfully
// ------------------------------------------------------------------

#[test]
fn every_fixture_parses_without_error() {
    for (name, source) in FIXTURES {
        let g = parse(source).unwrap_or_else(|e| panic!("fixture {name} failed: {e}"));
        assert!(
            !g.entities.is_empty(),
            "fixture {name} produced no entities"
        );
    }
}

// ------------------------------------------------------------------
// Common structural invariants
// ------------------------------------------------------------------

#[test]
fn every_fixture_has_three_header_entities() {
    for (name, _) in FIXTURES {
        let g = load(name);
        assert_eq!(
            g.header.len(),
            3,
            "fixture {name}: expected 3 HEADER entities, got {}",
            g.header.len()
        );
        let names: Vec<&str> = g
            .header
            .iter()
            .filter_map(|e| match e {
                RawEntity::Simple { name, .. } => Some(name.as_str()),
                RawEntity::Complex { .. } => None,
            })
            .collect();
        assert_eq!(
            names,
            vec!["FILE_DESCRIPTION", "FILE_NAME", "FILE_SCHEMA"],
            "fixture {name}: HEADER entity names mismatch"
        );
    }
}

#[test]
fn every_fixture_entity_ids_start_at_one() {
    for (name, _) in FIXTURES {
        let g = load(name);
        assert!(g.get(1).is_some(), "fixture {name}: entity #1 must exist");
    }
}

// ------------------------------------------------------------------
// box_ap214_is specific assertions
// ------------------------------------------------------------------

#[test]
fn box_ap214_is_entity_count() {
    let g = load("box_ap214_is");
    assert_eq!(g.entities.len(), 182);
}

#[test]
fn box_ap214_is_entity_165_is_complex_four_parts() {
    let g = load("box_ap214_is");
    let e = g.get(165).expect("entity #165 must exist");
    match e {
        RawEntity::Complex { parts, .. } => {
            assert_eq!(parts.len(), 4);
            assert_eq!(parts[0].name, "GEOMETRIC_REPRESENTATION_CONTEXT");
            assert_eq!(parts[1].name, "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT");
            assert_eq!(parts[2].name, "GLOBAL_UNIT_ASSIGNED_CONTEXT");
            assert_eq!(parts[3].name, "REPRESENTATION_CONTEXT");
        }
        RawEntity::Simple { .. } => panic!("#165 should be Complex"),
    }
}

#[test]
fn box_ap214_is_entity_169_typed_length_measure() {
    let g = load("box_ap214_is");
    let e = g.get(169).expect("entity #169 must exist");
    match e {
        RawEntity::Simple { attributes, .. } => {
            assert_eq!(
                attributes[0],
                Attribute::Typed {
                    type_name: "LENGTH_MEASURE".into(),
                    value: Box::new(Attribute::Real(1e-7)),
                }
            );
        }
        RawEntity::Complex { .. } => panic!("#169 should be Simple"),
    }
}

// ------------------------------------------------------------------
// Dangling reference check (DATA section only)
// ------------------------------------------------------------------

/// Recursively collect all `EntityRef(n)` values from an attribute tree.
fn walk_refs(attr: &Attribute, out: &mut Vec<u64>) {
    match attr {
        Attribute::EntityRef(n) => out.push(*n),
        Attribute::List(items) => {
            for item in items {
                walk_refs(item, out);
            }
        }
        Attribute::Typed { value, .. } => walk_refs(value, out),
        _ => {}
    }
}

fn collect_all_refs(graph: &EntityGraph) -> Vec<u64> {
    let mut refs = Vec::new();
    for entity in graph.entities.values() {
        let attrs = match entity {
            RawEntity::Simple { attributes, .. } => attributes.as_slice(),
            RawEntity::Complex { parts, .. } => {
                for part in parts {
                    for attr in &part.attributes {
                        walk_refs(attr, &mut refs);
                    }
                }
                continue;
            }
        };
        for attr in attrs {
            walk_refs(attr, &mut refs);
        }
    }
    refs
}

#[test]
fn every_fixture_has_no_dangling_references() {
    for (name, _) in FIXTURES {
        let g = load(name);
        let refs = collect_all_refs(&g);
        for ref_id in &refs {
            assert!(
                g.get(*ref_id).is_some(),
                "fixture {name}: dangling reference #{ref_id}"
            );
        }
    }
}

// ------------------------------------------------------------------
// Entity ID density (consecutive from #1)
// ------------------------------------------------------------------

#[test]
fn every_fixture_has_consecutive_entity_ids() {
    for (name, _) in FIXTURES {
        let g = load(name);
        let expected: Vec<u64> = (1..=g.entities.len() as u64).collect();
        let actual: Vec<u64> = g.entities.keys().copied().collect();
        assert_eq!(
            actual, expected,
            "fixture {name}: entity IDs are not consecutive from #1"
        );
    }
}

// ------------------------------------------------------------------
// Complex entity #166 part set (AP214/AP242 fixtures)
// ------------------------------------------------------------------

#[test]
fn non_ap203_fixtures_entity_166_is_complex_with_unit_parts() {
    for name in &[
        "box_ap214_cd",
        "box_ap214_dis",
        "box_ap214_is",
        "box_ap242_dis",
    ] {
        let g = load(name);
        let e = g
            .get(166)
            .unwrap_or_else(|| panic!("fixture {name}: entity #166 not found"));
        match e {
            RawEntity::Complex { parts, .. } => {
                let mut names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
                names.sort_unstable();
                assert_eq!(
                    names,
                    vec!["LENGTH_UNIT", "NAMED_UNIT", "SI_UNIT"],
                    "fixture {name}: #166 part names mismatch"
                );
            }
            RawEntity::Simple { .. } => panic!("fixture {name}: #166 should be Complex"),
        }
    }
}

// ------------------------------------------------------------------
// AP203 extra schema
// ------------------------------------------------------------------

#[test]
fn box_ap203_has_two_schema_entries() {
    let g = load("box_ap203");
    let raw = g.schema.raw().expect("raw preserved");
    assert!(
        raw.len() >= 2,
        "AP203 should have at least 2 FILE_SCHEMA entries"
    );
    assert!(raw.iter().any(|s| s.contains("SHAPE_APPEARANCE_LAYER_MIM")));
}
