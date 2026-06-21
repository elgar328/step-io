//! `gen-early` — generates the mechanical part of the `EarlyModel` (L1) layer
//! (`src/early/generated/{model,bind,serialize}.rs`, incl. the `Early*Id`
//! cache-key types) from `schema/early.toml` + `schema/mapping.toml`. The
//! hand-written `lower`/`lift` (semantic mapping) stay hand-coded.
//!
//! Run from anywhere: `cargo run -p gen-early`. Output is committed; this is a
//! dev tool, not a `build.rs`. Generates the entities in the `generate` list
//! (mapping.toml) — or the whole schema under the `generate_all` diagnostic
//! flip. The full schema union (all 2195 entities) is supported: entity refs,
//! primitives (incl. logical / binary), ENUMs (hinted L2-reuse or synthesized
//! `Early*`), mixed SELECTs (incl. recursive / nested / typed aggregation
//! members), aggregations up to 3 levels, faithful `OPTIONAL` (`Option<T>`;
//! `lower` decides any collapse), and inheritance flattening (supertype attrs
//! prepended in Part21 order, see `EarlyToml::flattened_attrs`).

mod classify;
mod emit;
mod expr;
mod mapping;
mod profile;
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

    let (targets, skipped) = collect_targets(&ctx);

    let head = file_head(ctx.mapping.generate_all);
    let mut out = GenOut::new(&head);

    for ent_name in &targets {
        emit_entity(&ctx, ent_name, &mut out);
    }
    let GenOut {
        mut model,
        mut bind,
        mut serialize,
        used_enums,
        used_enum_singles,
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
    emit_enum_helpers(
        &ctx,
        &used_enums,
        &used_enum_singles,
        &mut model,
        &mut bind,
        &mut serialize,
        &mut emitted_models,
    );

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
    write_fmt(&format!("{dir}/profile.rs"), &profile::emit_profiles(root));
    write_fmt(
        &format!("{dir}/mod.rs"),
        &format!(
            "{HEADER}pub(crate) mod bind;\npub(crate) mod model;\npub(crate) mod profile;\npub(crate) mod serialize;\n"
        ),
    );
    eprintln!("gen-early: wrote {} entities to {dir}", targets.len());

    write_coverage(root, &ctx, &targets, &skipped);
}

/// The generated-file header: bare for the committed (off) output; under the
/// `generate_all` flip an `allow` block is added (most generated items are
/// unused diagnostic mass without lower/lift).
fn file_head(generate_all: bool) -> String {
    // Unused generated types/fns (every entity but the wired few have no
    // lower/lift) are expected under the flip — suppress dead_code there only,
    // so the committed (`generate_all = false`) output stays byte-identical.
    if generate_all {
        // Diagnostic mass: most types are unused (no lower/lift), empty entities'
        // serialize fns don't read `l1`, and big entities trip pedantic style
        // lints. All flip-only — the committed (off) output keeps a minimal header.
        format!(
            "{HEADER}#![allow(dead_code, unused_variables, clippy::too_many_lines, clippy::enum_variant_names, clippy::struct_field_names, clippy::struct_excessive_bools, clippy::clone_on_copy, clippy::single_match_else, clippy::match_single_binding)]\n\n"
        )
    } else {
        // Schema-faithful field names legitimately share pre/postfixes
        // (e.g. CALENDAR_DATE's *_component) — not a naming smell here.
        // `too_many_lines`: a large synth SELECT (e.g. `measure_value`, 42
        // members) emits a >100-line `bind_*`/`*_emit` — inherent to the
        // member count, not a complexity smell.
        format!(
            "{HEADER}#![allow(clippy::struct_field_names, clippy::too_many_lines, clippy::clone_on_copy, clippy::get_first)]\n\n"
        )
    }
}

/// Resolve the generation targets: the wired `generate` list, or (under the
/// `generate_all` diagnostic flip) every supported entity — the rest recorded
/// as (entity, reason) for the coverage report.
fn collect_targets(ctx: &Ctx) -> (Vec<String>, Vec<(String, String)>) {
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
    (targets, skipped)
}

/// Emit the standalone-field ENUM helpers: hint-less ones synthesize an
/// `Early*` enum; hinted ones get `bind_<alias>` / `<alias>_attr` against the
/// reused L2 type (split from `main`).
fn emit_enum_helpers(
    ctx: &Ctx,
    used_enums: &BTreeSet<String>,
    used_enum_singles: &BTreeSet<String>,
    model: &mut String,
    bind: &mut String,
    serialize: &mut String,
    emitted_models: &mut BTreeSet<String>,
) {
    for alias in used_enums {
        // The single `bind_<alias>` helper is dead for an enum used only inside a
        // `SET/LIST OF` (the `<alias>_list` helper inlines token parsing); emit it
        // only when the enum is used as a single attr somewhere.
        let single = used_enum_singles.contains(alias);
        // Hint-less ENUM -> synthesize an `Early*` enum (model def + helpers).
        let Some(h) = ctx.mapping.enums.get(alias) else {
            emit_synth_enum(ctx, alias, single, model, bind, serialize, emitted_models);
            continue;
        };
        // bind helper (single-attr only)
        if single {
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
                // Strict: a token outside the EXPRESS enumeration is non-standard
                // input. Surface it as `NonStandardEnumValue` so the dispatcher
                // reclassifies the drop as NORM (correct rejection), not a defect.
                writeln!(
                bind,
                "        other => Err(crate::ir::error::ConvertError::NonStandardEnumValue {{ entity_id, field: field.to_string(), token: other.to_string() }}),"
            )
            .unwrap();
            }
            writeln!(bind, "    }}\n}}\n").unwrap();
        }

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
}

/// Write `schema/coverage.txt` (diagnostic flip only). Deterministic:
/// `skipped` is built in sorted entity order; reasons tallied into a `BTreeMap`.
fn write_coverage(root: &str, ctx: &Ctx, targets: &[String], skipped: &[(String, String)]) {
    if ctx.mapping.generate_all {
        let total = ctx.schema.entity.len();
        let supported = targets.len();
        let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, reason) in skipped {
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
        for (ent, reason) in skipped {
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
