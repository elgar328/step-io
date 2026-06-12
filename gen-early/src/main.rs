//! `gen-early` — generates the mechanical part of the EarlyModel (L1) layer
//! (`src/early/generated/{model,bind,serialize}.rs`) from `schema/early.toml`
//! + `schema/mapping.toml`. The hand-written `lower`/`lift` (semantic mapping)
//! and the `early_id!` ids stay hand-coded.
//!
//! Run from anywhere: `cargo run -p gen-early`. Output is committed; this is a
//! dev tool, not a `build.rs`. Generates the entities in the `generate` list
//! (mapping.toml). Supports entity refs / primitives (real, integer, bool,
//! logical, string) / ENUMs (hinted L2-reuse or auto-synthesized `Early*` when hint-less)
//! / SELECTs (all-entity -> ref; mixed -> hinted or auto-synthesized `Early*`
//! for scalar+ENUM members) / aggregations (`LIST/SET OF` ref or scalar ->
//! `Vec<u64/f64/i64/String>`) / OPTIONAL (faithful `Option<T>`; `lower` decides
//! any collapse) and **inheritance flattening** (supertype attrs prepended in
//! Part21 order, see [`EarlyToml::flattened_attrs`]).

mod classify;
mod emit;
mod expr;
mod mapping;
mod schema;
#[cfg(test)]
mod testutil;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::process::Command;

use classify::Ctx;
use emit::{
    GenOut, HEADER, emit_entity, emit_enum_list, emit_logical_list, emit_select, emit_select_grid,
    emit_select_list, emit_select_synth, emit_synth_enum, emit_token_enum,
};
use mapping::Mapping;
use schema::EarlyToml;

fn main() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let schema: EarlyToml = toml::from_str(
        &std::fs::read_to_string(format!("{root}/schema/early.toml")).expect("read early.toml"),
    )
    .expect("parse early.toml");
    let mapping: Mapping = toml::from_str(
        &std::fs::read_to_string(format!("{root}/schema/mapping.toml")).expect("read mapping.toml"),
    )
    .expect("parse mapping.toml");
    let ctx = Ctx { schema, mapping };

    // Targets: normally the wired `generate` list; under the diagnostic flip
    // (`generate_all`), every entity the codegen can handle (the rest skipped +
    // recorded for the coverage report).
    let mut skipped: Vec<(String, String)> = Vec::new();
    let targets: Vec<String> = if ctx.mapping.generate_all {
        let mut t = Vec::new();
        for ent_name in ctx.schema.entity.keys() {
            match ctx.entity_supported(ent_name) {
                Ok(()) => t.push(ent_name.clone()),
                Err(reason) => skipped.push((ent_name.clone(), reason)),
            }
        }
        t
    } else {
        ctx.mapping.generate.clone()
    };

    // Unused generated types/fns (every entity but the wired few have no
    // lower/lift) are expected under the flip — suppress dead_code there only,
    // so the committed (`generate_all = false`) output stays byte-identical.
    let head = if ctx.mapping.generate_all {
        // Diagnostic mass: most types are unused (no lower/lift), empty entities'
        // serialize fns don't read `l1`, and big entities trip pedantic style
        // lints. All flip-only — the committed (off) output keeps a bare header.
        format!(
            "{HEADER}#![allow(dead_code, unused_variables, clippy::too_many_lines, clippy::enum_variant_names, clippy::struct_field_names, clippy::struct_excessive_bools, clippy::clone_on_copy, clippy::single_match_else, clippy::match_single_binding)]\n\n"
        )
    } else {
        HEADER.to_string()
    };
    let mut out = GenOut::new(&head);

    for ent_name in &targets {
        emit_entity(&ctx, ent_name, &mut out);
    }
    let GenOut {
        mut model,
        mut bind,
        mut serialize,
        used_enums,
        used_enum_lists,
        used_selects,
        used_select_lists,
        used_select_grids,
        any_bool,
        any_logical_list,
    } = out;

    // generated mixed-SELECT helpers. Hinted SELECTs use `emit_select` (token-
    // based enum helpers, collected in `token_enums`); hint-less SELECTs use
    // `emit_select_synth` (inline enum maps). Synth enum *models* are deduped
    // across selects and standalone fields via `emitted_models`.
    let mut token_enums: BTreeSet<String> = BTreeSet::new();
    let mut emitted_models: BTreeSet<String> = BTreeSet::new();
    let mut emitted_selects: BTreeSet<String> = BTreeSet::new();
    for sel in &used_selects {
        if ctx.mapping.selects.contains_key(sel) {
            emit_select(
                &ctx,
                sel,
                &mut model,
                &mut bind,
                &mut serialize,
                &mut token_enums,
            );
        } else {
            emit_select_synth(
                &ctx,
                sel,
                &mut model,
                &mut bind,
                &mut serialize,
                &mut emitted_models,
                &mut emitted_selects,
            );
        }
    }
    for alias in &token_enums {
        emit_token_enum(&ctx, alias, &mut bind, &mut serialize);
    }

    // generated standalone-field ENUM helpers
    for alias in &used_enums {
        // Hint-less ENUM -> synthesize an `Early*` enum (model def + helpers).
        let Some(h) = ctx.mapping.enums.get(alias) else {
            emit_synth_enum(
                &ctx,
                alias,
                &mut model,
                &mut bind,
                &mut serialize,
                &mut emitted_models,
            );
            continue;
        };
        // bind helper
        writeln!(
            bind,
            "fn bind_{alias}(attrs: &[crate::parser::entity::Attribute], index: usize, entity_id: u64, field: &'static str) -> Result<{}, crate::ir::error::ConvertError> {{",
            h.rust_type
        )
        .unwrap();
        writeln!(
            bind,
            "    match crate::ir::attr::read_enum(attrs, index, entity_id, field)? {{"
        )
        .unwrap();
        for (member, variant) in &h.variants {
            // Skip members whose variant is the default — the catch-all below
            // already covers them (avoids a `match_same_arms` duplicate).
            if h.default.as_ref() == Some(variant) {
                continue;
            }
            writeln!(
                bind,
                "        \"{}\" => Ok({}::{variant}),",
                member.to_uppercase(),
                h.rust_type
            )
            .unwrap();
        }
        if let Some(default) = &h.default {
            writeln!(bind, "        _ => Ok({}::{default}),", h.rust_type).unwrap();
        } else {
            writeln!(
                bind,
                "        other => Err(crate::ir::error::ConvertError::UnexpectedEntityForm {{ entity_id, detail: format!(\"{{field}}: unknown {alias} '.{{other}}.'\") }}),"
            )
            .unwrap();
        }
        writeln!(bind, "    }}\n}}\n").unwrap();

        // serialize helper
        writeln!(
            serialize,
            "fn {alias}_attr(v: {}) -> crate::parser::entity::Attribute {{",
            h.rust_type
        )
        .unwrap();
        writeln!(
            serialize,
            "    crate::parser::entity::Attribute::Enum(match v {{"
        )
        .unwrap();
        for (member, variant) in &h.variants {
            writeln!(
                serialize,
                "        {}::{variant} => \"{}\",",
                h.rust_type,
                member.to_uppercase()
            )
            .unwrap();
        }
        writeln!(serialize, "    }}.into())\n}}\n").unwrap();
    }

    // generated `<alias>_list` bind helpers for ENUMs used in `Vec<…>` fields.
    for alias in &used_enum_lists {
        emit_enum_list(&ctx, alias, &mut bind);
    }
    // ...and for SELECTs in `Vec<…>` fields (disjoint alias sets, no clash).
    for alias in &used_select_lists {
        emit_select_list(&ctx, alias, &mut bind);
    }
    // ...and for SELECTs in `Vec<Vec<…>>` fields (`<alias>_grid`).
    for alias in &used_select_grids {
        emit_select_grid(&ctx, alias, &mut bind);
    }
    if any_logical_list {
        emit_logical_list(&mut bind);
    }

    if any_bool {
        writeln!(
            serialize,
            "fn bool_attr(b: bool) -> crate::parser::entity::Attribute {{\n    crate::parser::entity::Attribute::Enum(if b {{ \"T\" }} else {{ \"F\" }}.into())\n}}\n"
        )
        .unwrap();
    }

    let dir = format!("{root}/src/early/generated");
    std::fs::create_dir_all(&dir).expect("mkdir generated");
    write_fmt(&format!("{dir}/model.rs"), &model);
    write_fmt(&format!("{dir}/bind.rs"), &bind);
    write_fmt(&format!("{dir}/serialize.rs"), &serialize);
    write_fmt(
        &format!("{dir}/mod.rs"),
        &format!(
            "{HEADER}pub(crate) mod bind;\npub(crate) mod model;\npub(crate) mod serialize;\n"
        ),
    );
    eprintln!("gen-early: wrote {} entities to {dir}", targets.len());

    // Coverage report (diagnostic flip only). Deterministic: `skipped` is built
    // in sorted entity order; reasons tallied into a BTreeMap.
    if ctx.mapping.generate_all {
        let total = ctx.schema.entity.len();
        let supported = targets.len();
        let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, reason) in &skipped {
            *by_reason.entry(reason.as_str()).or_default() += 1;
        }
        let mut report = String::new();
        writeln!(
            report,
            "gen-early coverage (generate_all)\nsupported: {supported} / {total}\nskipped:   {}\n",
            skipped.len()
        )
        .unwrap();
        writeln!(report, "-- skipped by reason --").unwrap();
        for (reason, n) in &by_reason {
            writeln!(report, "  {reason:<16} {n}").unwrap();
        }
        writeln!(report, "\n-- skipped entities --").unwrap();
        for (ent, reason) in &skipped {
            writeln!(report, "  {ent:<48} {reason}").unwrap();
        }
        std::fs::write(format!("{root}/schema/coverage.txt"), &report).expect("write coverage.txt");
        eprintln!(
            "gen-early: coverage {supported}/{total} supported, {} skipped",
            skipped.len()
        );
    }
}

fn write_fmt(path: &str, body: &str) {
    std::fs::write(path, body).expect("write generated file");
    let status = Command::new("rustfmt")
        .args(["--edition", "2024", path])
        .status()
        .expect("run rustfmt");
    assert!(status.success(), "rustfmt failed on {path}");
}
