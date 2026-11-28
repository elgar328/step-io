//! `codegen` generator binary. Reads `schema/universal.toml` + `codegen/coverage.toml`,
//! computes the units/geometry/topology schema closure, and emits a
//! schema-faithful model + generic 2-pass read + topo write into step-io's
//! `src/generated/`. Run: `cargo run -p codegen`.

mod author_narrow;
mod classify;
mod coverage;
mod emit;
mod model_ir;
mod schema;

use std::collections::BTreeSet;
use std::process::Command;

use classify::Resolver;
use model_ir::{ModelIr, build_closure};
use schema::Schema;

fn main() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let schema: Schema = toml::from_str(
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
    report_closure_change(&format!("{root}/docs/entities.md"), &closure);

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
    //    SELF\named_unit.dimensions → named_unit.dimensions), in any edition.
    //    Keyed by P. Only in-closure derivers count: an attr only an entity
    //    out of scope derives stays a plain field.
    let mut derived: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut derivable: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    // The in-closure parts deriving each derivable (super part, attr): in
    // every edition declaring them, in some edition (`edition_derives` adds
    // the DERIVEs only some editions make), in their latest declaration.
    let mut part_derivers: std::collections::BTreeMap<(String, String), model_ir::PartDerivers> =
        std::collections::BTreeMap::new();
    for (name, ent) in &schema.entity {
        if !closure.contains(name) {
            continue;
        }
        for d in ent.derives.iter().chain(&ent.edition_derives) {
            let Some((sup, attr)) = d.rsplit_once('.') else {
                continue;
            };
            if !complex_parts.contains(sup) {
                continue;
            }
            assert!(
                schema.entity[sup].own_attrs.iter().any(|a| a.name == attr),
                "{name} derives {d}, which {sup} does not declare"
            );
            derivable
                .entry(sup.to_string())
                .or_default()
                .insert(attr.to_string());
            let latest = ent.derives.contains(d);
            let e = part_derivers
                .entry((sup.to_uppercase(), attr.to_string()))
                .or_default();
            e.sometimes.insert(name.to_uppercase());
            if latest {
                e.latest.insert(name.to_uppercase());
                if !ent.edition_derives.contains(d) {
                    e.always.insert(name.to_uppercase());
                }
            }
        }
    }
    for (name, ent) in &schema.entity {
        for d in &ent.derives {
            let attr = d.rsplit_once('.').map_or(d.as_str(), |(_, a)| a);
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

    let mut ir = ModelIr::build(
        &schema,
        &res,
        &closure,
        &complex_parts,
        &derived,
        &derivable,
    );
    ir.part_derivers = part_derivers;

    eprintln!(
        "codegen: closure={} simple={} parts={} ref_enums={} enums={}",
        closure.len(),
        ir.simples.len(),
        ir.parts.len(),
        ir.ref_enums.len(),
        ir.enums.len(),
    );

    // Emit into step-io's own `src/generated`, where the parser types are reached
    // as `crate::` (the generated code lives inside step-io).
    // Output profile (legal set / downgrade / header), baked from the vendored
    // schema/ap242e4.toml. AP242 ed4 is the single output schema (step-io
    // reads all APs but always writes ed4); its FILE_SCHEMA identity is baked
    // into src/generated/schema.rs for the authoring layer to stamp.
    let profile_specs = [("AP242E4", "Ap242", "ap242e4")];
    let parsed: Vec<(&str, &str, schema::ProfileToml)> = profile_specs
        .iter()
        .map(|(prefix, variant, stem)| {
            let p: schema::ProfileToml = toml::from_str(
                &std::fs::read_to_string(format!("{root}/schema/{stem}.toml"))
                    .unwrap_or_else(|_| panic!("read {stem}.toml")),
            )
            .unwrap_or_else(|_| panic!("parse {stem}.toml"));
            (*prefix, *variant, p)
        })
        .collect();
    // The output schema read as a schema of its own (entities, attributes
    // with every redeclaration, SELECTs), for where it is narrower than the
    // model.
    let target_schema: Schema = toml::from_str(
        &std::fs::read_to_string(format!("{root}/schema/{}.toml", profile_specs[0].2))
            .expect("read output profile"),
    )
    .expect("parse output profile as a schema");
    let legal: BTreeSet<String> = parsed[0]
        .2
        .entity
        .keys()
        .map(|k| k.to_uppercase())
        .collect();
    let narrowing = author_narrow::AuthorNarrowing::build(&ir, &target_schema, &legal);
    for line in narrowing.report() {
        eprintln!("codegen: narrow {line}");
    }
    let targets: Vec<(&str, &str, &schema::ProfileToml)> =
        parsed.iter().map(|(a, b, c)| (*a, *b, c)).collect();
    let dir = format!("{root}/src/generated");
    let crate_path = "crate";
    std::fs::create_dir_all(&dir).expect("mkdir generated");
    write_fmt(
        &format!("{dir}/model.rs"),
        &emit::emit_model(&ir, &narrowing),
    );
    write_fmt(
        &format!("{dir}/read.rs"),
        &emit::emit_read(&ir, &schema, crate_path),
    );
    write_fmt(&format!("{dir}/write.rs"), &emit::emit_write(&ir));
    write_fmt(&format!("{dir}/walk.rs"), &emit::emit_walk(&ir));
    write_fmt(&format!("{dir}/resolve.rs"), &emit::emit_resolvers(&ir));
    write_fmt(
        &format!("{dir}/generic_normalize.rs"),
        &emit::emit_normalize(&ir, crate_path),
    );

    write_fmt(&format!("{dir}/schema.rs"), &emit::emit_schema(&targets));

    // Strict AP242 authoring layer: constructors only for AP242-legal entities
    // that are not ABSTRACT.
    write_fmt(
        &format!("{dir}/author.rs"),
        &emit::emit_author(&ir, &parsed[0].2, &schema, &narrowing),
    );

    write_fmt(
        &format!("{dir}/mod.rs"),
        "// @generated by codegen. DO NOT EDIT.\n//! The generated layer — the schema-faithful [`model`] with its [`author`]\n//! constructors and [`resolve`] navigation, plus the read/write plumbing.\n//!\n//! All emitted by the `codegen` crate from `schema/universal.toml`. Users see\n//! the model and its navigation as `step_io::model`, and the authoring layer\n//! at the crate root.\n#![allow(dead_code, unused_variables, unreachable_code, clippy::all, clippy::pedantic)]\npub mod author;\npub(crate) mod generic_normalize;\npub mod model;\npub(crate) mod read;\npub mod resolve;\npub(crate) mod schema;\npub(crate) mod walk;\npub(crate) mod write;\n",
    );

    // Coverage matrix doc: every universal entity vs the source APs, the
    // closure (read), and the output target (write). Follows profile_specs
    // when the output target moves. Plain markdown — not a rustfmt target.
    std::fs::create_dir_all(format!("{root}/docs")).expect("mkdir docs");
    std::fs::write(
        format!("{root}/docs/entities.md"),
        emit::emit_entity_matrix(&schema, &closure, profile_specs[0].2, &parsed[0].2),
    )
    .expect("write entities.md");
    eprintln!("codegen: wrote model/read/write/schema to {dir}");
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

/// Print the entities the closure gains and loses against the read column of
/// the committed `docs/entities.md`, so a schema or coverage change that moves
/// the support boundary is seen before it lands.
fn report_closure_change(matrix: &str, closure: &BTreeSet<String>) {
    let Ok(text) = std::fs::read_to_string(matrix) else {
        return;
    };
    let cols = |l: &str| -> Vec<String> { l.split('|').map(|c| c.trim().to_string()).collect() };
    // The read column's position, from the entity table's header row.
    let Some(read) = text
        .lines()
        .find(|l| l.starts_with("| Entity |"))
        .and_then(|h| cols(h).iter().position(|c| c == "step-io (read)"))
    else {
        return;
    };
    let before: BTreeSet<String> = text
        .lines()
        .filter_map(|l| {
            let cols = cols(l);
            let name = cols.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            (cols.get(read).map(String::as_str) == Some("o")).then(|| name.to_string())
        })
        .collect();
    let added: Vec<&String> = closure.difference(&before).collect();
    let removed: Vec<&String> = before.difference(closure).collect();
    if !added.is_empty() || !removed.is_empty() {
        eprintln!("codegen: closure +{} {added:?}", added.len());
        eprintln!("codegen: closure -{} {removed:?}", removed.len());
    }
}
