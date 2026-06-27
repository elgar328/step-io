//! `codegen` generator binary. Reads `schema/universal.toml` + `codegen/coverage.toml`,
//! computes the units/geometry/topology schema closure, and emits a
//! schema-faithful model + generic 2-pass read + topo write into
//! `codegen/src/generated/`. Run: `cargo run -p codegen --bin codegen`.

mod classify;
mod coverage;
mod emit;
mod model_ir;
mod schema;

use std::collections::BTreeSet;
use std::process::Command;

use classify::Resolver;
use model_ir::{ModelIr, build_closure};
use schema::EarlyToml;

fn main() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let schema: EarlyToml = toml::from_str(
        &std::fs::read_to_string(format!("{root}/schema/universal.toml"))
            .expect("read universal.toml"),
    )
    .expect("parse universal.toml");
    // codegen support boundary (coverage.toml) lives in the codegen crate root
    // (CARGO_MANIFEST_DIR, no `/..`), separate from the schema/ data files.
    let coverage: coverage::Coverage = toml::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/coverage.toml"))
            .expect("read coverage.toml"),
    )
    .expect("parse coverage.toml");

    let res = Resolver {
        schema: &schema,
        narrow: &coverage.narrow,
    };

    // Cover seeds -> single combined closure.
    let mut seeds: Vec<String> = Vec::new();
    for list in coverage.cover.values() {
        seeds.extend(list.iter().cloned());
    }
    let exclude: BTreeSet<String> = coverage.exclude.iter().cloned().collect();
    let closure = build_closure(&schema, &res, &seeds, &exclude);

    // Complex-part entities = closure entities flagged is_complex_part in
    // universal.toml (auto-derived from EXPRESS ANDOR; replaces the former hand
    // -written [codegen].complex_parts). complex-only leaves are pulled into the
    // closure via domains seeds (they are not ref-reachable).
    let complex_parts: BTreeSet<String> = closure
        .iter()
        .filter(|n| schema.entity.get(*n).is_some_and(|e| e.is_complex_part))
        .cloned()
        .collect();

    // DERIVE (`*`) markers from universal.toml `derives` (each "super.attr" or
    // "attr"), split into hard vs conditional using `complex_parts`:
    //  - hard `derived`: a non-complex-part (simple-leaf) entity computes the
    //    attr always (e.g. oriented_edge.edge_start) → drop field, render `*`.
    //  - conditional `derivable`: a complex-part supertype P's attr is `*` only
    //    when a deriving sibling part is present (e.g. si_unit derives
    //    SELF\named_unit.dimensions → named_unit.dimensions). Keyed by P.
    let mut derived: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut derivable: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for (name, ent) in &schema.entity {
        for d in &ent.derives {
            let (sup, attr) = d
                .rsplit_once('.')
                .map_or((None, d.as_str()), |(s, a)| (Some(s), a));
            // derivable: the SUPER part's attr is `*` only when a deriving
            // sibling part is present (complex form). Keyed by the super part.
            // Only count IN-CLOSURE derivers — if the only deriver is out of
            // closure (e.g. mechanism_representation derives representation.
            // context_of_items, but kinematics isn't in scope), the attr is NOT
            // `*` for any in-closure instance, so it stays a plain ref (which
            // the Null-tolerant ref read handles for a non-standard `$`).
            if let Some(s) = sup
                && complex_parts.contains(s)
                && closure.contains(name)
            {
                derivable
                    .entry(s.to_string())
                    .or_default()
                    .insert(attr.to_string());
            }
            // hard derived: in E's OWN (simple) form the attr is always `*`
            // (E computes it) — applies regardless of E being a complex part too
            // (a complex part may also appear standalone). Only affects E's
            // flattened simple struct; E's part-bag variant uses own_attrs (the
            // inherited derived attr isn't an own attr, so it's a no-op there).
            derived
                .entry(name.clone())
                .or_default()
                .push(attr.to_string());
        }
    }

    let ir = ModelIr::build(
        &schema,
        &res,
        &closure,
        &complex_parts,
        &derived,
        &derivable,
    );

    eprintln!(
        "codegen: closure={} simple={} parts={} ref_enums={} enums={}",
        closure.len(),
        ir.simples.len(),
        ir.parts.len(),
        ir.ref_enums.len(),
        ir.enums.len(),
    );

    // Target selects output dir + the import prefix baked into the generated
    // files. Default = the codegen crate's own copy (used by the merkle oracle),
    // which imports the parser types from the `step_io` crate. `CODEGEN_TARGET=
    // step-io` emits into step-io's own `src/generated`, where the parser types
    // are reached as `crate::` (the generated code lives inside step-io).
    let (dir, crate_path) = match std::env::var("CODEGEN_TARGET").as_deref() {
        Ok("step-io") => (format!("{root}/src/generated"), "crate"),
        _ => (format!("{root}/codegen/src/generated"), "step_io"),
    };
    std::fs::create_dir_all(&dir).expect("mkdir generated");
    write_fmt(&format!("{dir}/model.rs"), &emit::emit_model(&ir));
    write_fmt(&format!("{dir}/read.rs"), &emit::emit_read(&ir, crate_path));
    write_fmt(&format!("{dir}/write.rs"), &emit::emit_write(&ir));
    write_fmt(
        &format!("{dir}/generic_normalize.rs"),
        &emit::emit_normalize(&ir, crate_path),
    );
    std::fs::write(
        format!("{dir}/mod.rs"),
        "// @generated by codegen. DO NOT EDIT.\n#![allow(dead_code, unused_variables, clippy::all, clippy::pedantic)]\npub mod generic_normalize;\npub mod model;\npub mod read;\npub mod write;\n",
    )
    .expect("write mod.rs");
    eprintln!("codegen: wrote model/read/write to {dir}");
}

fn write_fmt(path: &str, body: &str) {
    std::fs::write(path, body).expect("write generated file");
    // rustfmt is not idempotent on some deeply-nested generated expressions: one
    // pass can leave formatting that a second pass changes, so a single pass would
    // differ from a later `cargo fmt`. Run to a fixpoint so generated output is
    // already fully formatted (no `cargo fmt` churn on regen).
    for _ in 0..5 {
        let before = std::fs::read_to_string(path).unwrap_or_default();
        match Command::new("rustfmt")
            .args(["--edition", "2024", path])
            .status()
        {
            Ok(st) if st.success() => {}
            _ => {
                eprintln!("codegen: rustfmt skipped/failed for {path} (kept raw)");
                return;
            }
        }
        if std::fs::read_to_string(path).unwrap_or_default() == before {
            return;
        }
    }
}
