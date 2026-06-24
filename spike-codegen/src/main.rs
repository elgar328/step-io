//! Round-trip driver for the schema-faithful codegen spike (units subset).
//!
//! Proves: parse a box/fillet fixture -> filter the units subset into a map A'
//! -> generated read -> SpikeUnits -> generated write -> parse -> map B ->
//! `merkle_diff_maps(A', B) == None` (order/renumber-invariant data equality).

mod generated;
mod merkle;
mod normalize;
mod readwrite;

use std::collections::BTreeMap;

use step_io::{RawEntity, RawEntityPart, parse};

use merkle::merkle_diff_maps;
use readwrite::{Writer, read, wrap_step};

/// True if a raw entity belongs to the units subset this spike handles. [GEN]
fn is_unit_entity(ent: &RawEntity) -> bool {
    match ent {
        RawEntity::Simple { name, .. } => {
            name == "DIMENSIONAL_EXPONENTS"
                || name == "DERIVED_UNIT"
                || name == "DERIVED_UNIT_ELEMENT"
                || name.ends_with("MEASURE_WITH_UNIT")
        }
        RawEntity::Complex { parts, .. } => is_complex_unit_parts(parts),
    }
}

fn is_complex_unit_parts(parts: &[RawEntityPart]) -> bool {
    const NAMES: &[&str] = &[
        "NAMED_UNIT",
        "SI_UNIT",
        "CONVERSION_BASED_UNIT",
        "CONTEXT_DEPENDENT_UNIT",
        "LENGTH_UNIT",
        "MASS_UNIT",
        "PLANE_ANGLE_UNIT",
        "SOLID_ANGLE_UNIT",
        "TIME_UNIT",
    ];
    parts.iter().any(|p| NAMES.contains(&p.name.as_str()))
}

/// Filter a parsed graph down to the units subset map A'. Clones the
/// `RawEntity`s into a standalone `BTreeMap` (EntityGraph is not Clone, but
/// RawEntity is). The UNCERTAINTY_MEASURE_WITH_UNIT extra (name/description)
/// is part of A', so its faithfulness is checked too.
fn units_subset(src: &str) -> BTreeMap<u64, RawEntity> {
    let g = parse(src).expect("parse fixture");
    let mut m = BTreeMap::new();
    for (&id, ent) in &g.entities {
        if is_unit_entity(ent) {
            m.insert(id, ent.clone());
        }
    }
    m
}

fn run_fixture(path: &str) -> Result<usize, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let a = units_subset(&src);
    if a.is_empty() {
        return Err(format!("{path}: no unit entities found"));
    }
    let count = a.len();

    // generated read -> model -> generated write
    let (model, _idmap) = read(&a);
    let body = Writer::new(&model).emit_all();
    let file = wrap_step(&body);

    // re-parse the regenerated file, filter to the same units subset
    let b = units_subset(&file);

    match merkle_diff_maps(&a, &b) {
        None => Ok(count),
        Some(reason) => Err(format!("{path}: MERKLE MISMATCH: {reason}")),
    }
}

fn main() {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/");
    let fixtures = [
        // CRUX: 3 complex SI units, no refs (box_ap203 #166-168)
        "box_ap203.step",
        "box_ap214_dis.step",
        "box_ap214_is.step",
        // EXPANDED: CBU cluster (complex w/ refs + dim_exps + MWU chain)
        "fillet_box_ap214_is.step",
        "external_temp_abc_explicit_de.step",
        "external_temp_nist_property_def.stp",
    ];

    let mut all_pass = true;
    for f in fixtures {
        let path = format!("{base}{f}");
        match run_fixture(&path) {
            Ok(n) => println!("PASS  {f}  ({n} unit entities, merkle None)"),
            Err(e) => {
                all_pass = false;
                println!("FAIL  {e}");
            }
        }
    }

    println!(
        "\n=== generic-complex round-trip: {} ===",
        if all_pass {
            "GO (PASS)"
        } else {
            "NO-GO (FAIL)"
        }
    );

    // --- NormCase hook: CBU factor identification on the generic model ---
    println!("\n--- CBU normalize hook (factor identification) ---");
    let cbu_path = format!("{base}fillet_box_ap214_is.step");
    let src = std::fs::read_to_string(&cbu_path).expect("read cbu fixture");
    let a = units_subset(&src);
    let (model, _) = read(&a);
    let classified = normalize::classify_cbus(&model);
    if classified.is_empty() {
        println!("  (no CONVERSION_BASED_UNIT in fixture)");
    }
    for (name, id) in &classified {
        println!("  CBU '{name}' -> {id:?}");
    }

    if !all_pass {
        std::process::exit(1);
    }
}
