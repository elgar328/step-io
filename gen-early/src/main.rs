//! `gen-early` — generates the mechanical part of the EarlyModel (L1) layer
//! (`src/early/generated/{model,bind,serialize}.rs`) from `schema/early.toml`
//! + `schema/mapping.toml`. The hand-written `lower`/`lift` (semantic mapping)
//! and the `early_id!` ids stay hand-coded.
//!
//! Run from anywhere: `cargo run -p gen-early`. Output is committed; this is a
//! dev tool, not a `build.rs`. Generates the entities in the `generate` list
//! (mapping.toml). Supports entity refs / primitives (real, integer, bool,
//! string) / ENUMs (hinted L2-reuse or auto-synthesized `Early*` when hint-less)
//! / all-entity & mixed SELECTs / OPTIONAL (required+drop) and **inheritance
//! flattening** (supertype attrs prepended in Part21 order, see
//! [`EarlyToml::flattened_attrs`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::process::Command;

use serde::Deserialize;

// ---- schema/early.toml ----

#[derive(Deserialize)]
struct EarlyToml {
    #[serde(default)]
    entity: BTreeMap<String, Entity>,
    #[serde(rename = "type", default)]
    types: BTreeMap<String, TypeDef>,
}

#[derive(Deserialize)]
struct Entity {
    /// Direct supertype names (EXPRESS declaration order, left-to-right).
    #[serde(default)]
    parents: Vec<String>,
    #[serde(default)]
    own_attrs: Vec<Attr>,
}

impl EarlyToml {
    /// Full attribute list in Part21 positional order: inherited attrs first
    /// (each parent chain deepest-ancestor-first, parents left-to-right), then
    /// the entity's own attrs. Mirrors the blueprint's `collect_ancestor_attrs`.
    ///
    /// A shared ancestor reached via multiple parent paths (diamond) is included
    /// once — deduped by entity name via `visited`. Same-named attrs declared by
    /// *different* parents both survive (dedup is per-entity, not per-attr).
    /// L1 carries no redeclared attrs, so no type narrowing is applied here.
    fn flattened_attrs(&self, ent_name: &str) -> Vec<&Attr> {
        let mut out = Vec::new();
        let mut visited = BTreeSet::new();
        self.collect_inherited(ent_name, &mut visited, &mut out);
        if let Some(ent) = self.entity.get(ent_name) {
            out.extend(ent.own_attrs.iter());
        }
        out
    }

    fn collect_inherited<'a>(
        &'a self,
        ent_name: &str,
        visited: &mut BTreeSet<String>,
        out: &mut Vec<&'a Attr>,
    ) {
        let Some(ent) = self.entity.get(ent_name) else {
            return; // dangling parent reference — skip (matches blueprint)
        };
        for parent in &ent.parents {
            if !visited.insert(parent.clone()) {
                continue; // diamond: shared ancestor already collected
            }
            self.collect_inherited(parent, visited, out);
            if let Some(p) = self.entity.get(parent) {
                out.extend(p.own_attrs.iter());
            }
        }
    }
}

#[derive(Deserialize)]
struct Attr {
    name: String,
    ty: String,
}

#[derive(Deserialize)]
struct TypeDef {
    aliased: String,
}

// ---- schema/mapping.toml ----

#[derive(Deserialize)]
struct Mapping {
    generate: Vec<String>,
    #[serde(rename = "enum", default)]
    enums: BTreeMap<String, EnumHint>,
    #[serde(rename = "select", default)]
    selects: BTreeMap<String, SelectHint>,
}

#[derive(Deserialize)]
struct EnumHint {
    /// Full path of the reused L2 enum, e.g. `crate::ir::visualization::Projection`.
    rust_type: String,
    /// early.toml ENUM member -> L2 variant. STEP token = member upper-cased.
    variants: BTreeMap<String, String>,
    /// Unknown STEP token -> this variant (matches a hand-written `_ => …`
    /// catch-all). `None` => error on unknown.
    #[serde(default)]
    default: Option<String>,
    /// Variant that carries the raw token `String` for unknown values (e.g.
    /// `MarkerType::Other(token)`). Used by the token-based helpers a SELECT
    /// member needs. `None` => no such variant.
    #[serde(default)]
    catch_all: Option<String>,
}

/// Hint for a *mixed* SELECT (members span entity / enum / primitive) -> a
/// synthesized `Early*` enum. Member kinds and any enum hint are auto-derived.
#[derive(Deserialize)]
struct SelectHint {
    /// Synthesized enum name, e.g. `EarlyMarker` (lives in generated/model.rs).
    rust_type: String,
    /// SELECT member -> L1 variant name.
    variants: BTreeMap<String, String>,
}

// ---- classification ----

/// How an attribute's resolved `ty` maps to Rust / bind / serialize.
enum Kind {
    Ref,            // entity ref (incl. all-entity SELECT) -> u64
    VecRef,         // aggregation of refs -> Vec<u64>
    Real,           // -> f64
    Int,            // -> i64
    Bool,           // -> bool
    Str,            // -> String
    Enum(String),   // ENUM type-alias name (hinted -> L2 reuse, else synth Early*)
    Select(String), // mixed SELECT type-alias name (looked up in mapping.selects)
}

const MAX_DEPTH: u32 = 32;

/// Inner element of a `LIST/SET/BAG/ARRAY OF X` aggregation, or `None`.
fn agg_inner(ty: &str) -> Option<&str> {
    ["LIST OF ", "SET OF ", "BAG OF ", "ARRAY OF "]
        .iter()
        .find_map(|p| ty.strip_prefix(p))
        .map(str::trim)
}

/// Members of an inline `SELECT(a, b, …)` aliased type, or `None`.
fn select_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("SELECT(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

/// Members of an inline `ENUM(a, b, …)` aliased type, or `None`.
fn enum_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("ENUM(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

struct Ctx {
    schema: EarlyToml,
    mapping: Mapping,
}

impl Ctx {
    /// True when `ty` is (or resolves through aliases / all-entity SELECTs to)
    /// an entity reference — i.e. encodes in Part21 as a bare `#N`.
    fn ref_like(&self, ty: &str, depth: u32) -> bool {
        assert!(
            depth < MAX_DEPTH,
            "gen-early: ty `{ty}` resolution too deep (cycle?)"
        );
        let t = ty.trim();
        if self.schema.entity.contains_key(t) {
            return true;
        }
        let Some(td) = self.schema.types.get(t) else {
            return false; // primitive keyword or unknown
        };
        let a = td.aliased.trim();
        if let Some(members) = select_members(a) {
            return members.iter().all(|m| self.ref_like(m, depth + 1));
        }
        if a.starts_with("ENUM(") {
            return false;
        }
        self.ref_like(a, depth + 1)
    }

    /// Resolve a `ty` token to a [`Kind`]. Handles entity refs, primitives
    /// (after alias resolution), reused ENUMs, all-entity SELECTs (-> ref) and
    /// aggregations of refs (-> `Vec<u64>`). OPTIONAL and mixed SELECTs are out
    /// of scope (v2) and panic loudly.
    fn classify(&self, ty: &str, depth: u32) -> Kind {
        assert!(
            depth < MAX_DEPTH,
            "gen-early: ty `{ty}` resolution too deep (cycle?)"
        );
        let t = ty.trim();
        // OPTIONAL is transparent: the inner type is bound as required; an
        // absent / unrecognized value drops the whole entity (matching the
        // hand-written point_style, whose L2 is non-optional). True `Option<T>`
        // (where L2 is `Option`) is deferred to v4.
        if let Some(inner) = t.strip_prefix("OPTIONAL ") {
            return self.classify(inner.trim(), depth + 1);
        }
        if let Some(inner) = agg_inner(t) {
            assert!(
                self.ref_like(inner, 0),
                "gen-early: aggregation of non-ref `{inner}` not supported yet"
            );
            return Kind::VecRef;
        }
        match t {
            "real" | "number" => return Kind::Real,
            "integer" => return Kind::Int,
            "boolean" => return Kind::Bool,
            "string" => return Kind::Str,
            _ => {}
        }
        if self.schema.entity.contains_key(t) {
            return Kind::Ref;
        }
        if let Some(td) = self.schema.types.get(t) {
            let a = td.aliased.trim();
            if a.starts_with("ENUM(") {
                return Kind::Enum(t.to_string());
            }
            if let Some(members) = select_members(a) {
                if members.iter().all(|m| self.ref_like(m, 0)) {
                    return Kind::Ref; // all-entity SELECT -> bare ref
                }
                assert!(
                    self.mapping.selects.contains_key(t),
                    "gen-early: mixed SELECT `{t}` needs a [select.{t}] hint"
                );
                return Kind::Select(t.to_string());
            }
            return self.classify(a, depth + 1);
        }
        panic!("gen-early: cannot resolve ty token `{t}`");
    }

    fn enum_hint(&self, alias: &str) -> &EnumHint {
        self.mapping
            .enums
            .get(alias)
            .unwrap_or_else(|| panic!("gen-early: no [enum.{alias}] hint in mapping.toml"))
    }

    fn select_hint(&self, alias: &str) -> &SelectHint {
        self.mapping
            .selects
            .get(alias)
            .unwrap_or_else(|| panic!("gen-early: no [select.{alias}] hint in mapping.toml"))
    }
}

fn pascal(snake: &str) -> String {
    snake
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

const HEADER: &str = "// DO NOT EDIT — generated by `cargo run -p gen-early` from schema/early.toml\n// + schema/mapping.toml. Hand-written lower/lift/ids live in the sibling modules.\n\n";

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

    let mut model = String::from(HEADER);
    let mut bind = String::from(HEADER);
    let mut serialize = String::from(HEADER);
    // Helpers to emit once after the loop (dedup across entities).
    let mut used_enums: BTreeSet<String> = BTreeSet::new(); // standalone field ENUMs
    let mut used_selects: BTreeSet<String> = BTreeSet::new(); // mixed SELECTs
    let mut any_bool = false;

    for ent_name in &ctx.mapping.generate {
        assert!(
            ctx.schema.entity.contains_key(ent_name),
            "gen-early: entity `{ent_name}` not in early.toml"
        );
        let type_name = format!("Early{}", pascal(ent_name));
        let step_name = ent_name.to_uppercase();
        // Full Part21 positional attribute list (inherited supertype attrs
        // prepended, then own); see `EarlyToml::flattened_attrs`.
        let attrs: Vec<(&Attr, Kind)> = ctx
            .schema
            .flattened_attrs(ent_name)
            .into_iter()
            .map(|a| (a, ctx.classify(&a.ty, 0)))
            .collect();
        let has_select = attrs.iter().any(|(_, k)| matches!(k, Kind::Select(_)));

        // model struct
        writeln!(model, "/// L1 `{step_name}` (generated).").unwrap();
        writeln!(model, "#[derive(Debug, Clone, PartialEq)]").unwrap();
        writeln!(model, "pub(crate) struct {type_name} {{").unwrap();
        for (a, k) in &attrs {
            writeln!(model, "    pub(crate) {}: {},", a.name, field_ty(&ctx, k)).unwrap();
        }
        writeln!(model, "}}\n").unwrap();

        // bind fn. A `Select` field drops the whole entity on `None`, so when one
        // is present the return type is `Result<Option<…>>` and binding uses a
        // let-form with early `return Ok(None)`; otherwise the simple literal form.
        let ret = if has_select {
            format!("Result<Option<super::model::{type_name}>, crate::ir::error::ConvertError>")
        } else {
            format!("Result<super::model::{type_name}, crate::ir::error::ConvertError>")
        };
        writeln!(
            bind,
            "pub(crate) fn bind_{ent_name}(entity_id: u64, attrs: &[crate::parser::entity::Attribute]) -> {ret} {{"
        )
        .unwrap();
        writeln!(
            bind,
            "    crate::ir::attr::check_count(attrs, {}, entity_id, \"{step_name}\")?;",
            attrs.len()
        )
        .unwrap();
        if has_select {
            for (i, (a, k)) in attrs.iter().enumerate() {
                if let Kind::Select(sel) = k {
                    writeln!(
                        bind,
                        "    let Some({}) = bind_{sel}(&attrs[{i}]) else {{ return Ok(None); }};",
                        a.name
                    )
                    .unwrap();
                } else {
                    writeln!(bind, "    let {} = {};", a.name, bind_expr(k, i, &a.name)).unwrap();
                }
            }
            writeln!(bind, "    Ok(Some(super::model::{type_name} {{").unwrap();
            for (a, _) in &attrs {
                writeln!(bind, "        {},", a.name).unwrap();
            }
            writeln!(bind, "    }}))\n}}\n").unwrap();
        } else {
            writeln!(bind, "    Ok(super::model::{type_name} {{").unwrap();
            for (i, (a, k)) in attrs.iter().enumerate() {
                writeln!(bind, "        {}: {},", a.name, bind_expr(k, i, &a.name)).unwrap();
            }
            writeln!(bind, "    }})\n}}\n").unwrap();
        }

        // serialize fn
        writeln!(
            serialize,
            "pub(crate) fn serialize_{ent_name}(buf: &mut crate::writer::buffer::WriteBuffer, l1: &super::model::{type_name}) -> u64 {{"
        )
        .unwrap();
        writeln!(serialize, "    buf.push_simple(\"{step_name}\", vec![").unwrap();
        for (a, k) in &attrs {
            writeln!(serialize, "        {},", serialize_expr(k, &a.name)).unwrap();
            match k {
                Kind::Bool => any_bool = true,
                Kind::Enum(alias) => {
                    used_enums.insert(alias.clone());
                }
                Kind::Select(alias) => {
                    used_selects.insert(alias.clone());
                }
                _ => {}
            }
        }
        writeln!(serialize, "    ])\n}}\n").unwrap();
    }

    // generated mixed-SELECT helpers (synthesized enum + bind/serialize). Enum
    // members need token-based helpers, collected here and emitted once below.
    let mut token_enums: BTreeSet<String> = BTreeSet::new();
    for sel in &used_selects {
        emit_select(
            &ctx,
            sel,
            &mut model,
            &mut bind,
            &mut serialize,
            &mut token_enums,
        );
    }
    for alias in &token_enums {
        emit_token_enum(&ctx, alias, &mut bind, &mut serialize);
    }

    // generated standalone-field ENUM helpers
    for alias in &used_enums {
        // Hint-less ENUM -> synthesize an `Early*` enum (model def + helpers).
        let Some(h) = ctx.mapping.enums.get(alias) else {
            emit_synth_enum(&ctx, alias, &mut model, &mut bind, &mut serialize);
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
    eprintln!(
        "gen-early: wrote {} entities to {dir}",
        ctx.mapping.generate.len()
    );
}

/// Emit a synthesized `Early*` enum + `bind_<sel>` / `<sel>_emit` for a mixed
/// SELECT. Enum members are recorded in `token_enums` for [`emit_token_enum`].
fn emit_select(
    ctx: &Ctx,
    sel: &str,
    model: &mut String,
    bind: &mut String,
    serialize: &mut String,
    token_enums: &mut BTreeSet<String>,
) {
    let hint = ctx.select_hint(sel);
    let rt = hint.rust_type.clone();
    // The synthesized enum lives in generated/model.rs; bind/serialize (sibling
    // modules) reference it through `super::model::`.
    let rtq = format!("super::model::{rt}");
    let aliased = ctx
        .schema
        .types
        .get(sel)
        .unwrap_or_else(|| panic!("gen-early: select `{sel}` not a type alias"))
        .aliased
        .clone();
    let members = select_members(aliased.trim())
        .unwrap_or_else(|| panic!("gen-early: select `{sel}` not a SELECT"));
    // (member name, L1 variant, member kind)
    let mems: Vec<(&str, String, Kind)> = members
        .iter()
        .map(|&m| {
            let v =
                hint.variants.get(m).cloned().unwrap_or_else(|| {
                    panic!("gen-early: [select.{sel}] missing variant for `{m}`")
                });
            (m, v, ctx.classify(m, 0))
        })
        .collect();
    let count = |pred: fn(&Kind) -> bool| mems.iter().filter(|(_, _, k)| pred(k)).count();
    assert!(
        count(|k| matches!(k, Kind::Ref)) <= 1,
        "gen-early: select `{sel}` has >1 entity member (cannot disambiguate refs)"
    );
    assert!(
        count(|k| matches!(k, Kind::Enum(_))) <= 1,
        "gen-early: select `{sel}` has >1 enum member (cannot disambiguate bare enums)"
    );
    let payload = |k: &Kind| -> String {
        match k {
            Kind::Ref => "u64".into(),
            Kind::Real => "f64".into(),
            Kind::Str => "String".into(),
            Kind::Enum(a) => ctx.enum_hint(a).rust_type.clone(),
            _ => panic!("gen-early: select `{sel}` member kind unsupported"),
        }
    };

    // model enum
    writeln!(model, "/// L1 mixed SELECT `{sel}` (generated).").unwrap();
    writeln!(model, "#[derive(Debug, Clone, PartialEq)]").unwrap();
    writeln!(model, "pub(crate) enum {rt} {{").unwrap();
    for (_, v, k) in &mems {
        writeln!(model, "    {v}({}),", payload(k)).unwrap();
    }
    writeln!(model, "}}\n").unwrap();

    // bind_<sel>
    if mems.iter().any(|(_, _, k)| matches!(k, Kind::Real)) {
        writeln!(bind, "#[allow(clippy::cast_precision_loss)]").unwrap();
    }
    writeln!(
        bind,
        "fn bind_{sel}(attr: &crate::parser::entity::Attribute) -> Option<{rtq}> {{"
    )
    .unwrap();
    writeln!(bind, "    match attr {{").unwrap();
    for (_, v, k) in &mems {
        if matches!(k, Kind::Ref) {
            writeln!(
                bind,
                "        crate::parser::entity::Attribute::EntityRef(n) => Some({rtq}::{v}(*n)),"
            )
            .unwrap();
        }
    }
    if mems
        .iter()
        .any(|(_, _, k)| matches!(k, Kind::Enum(_) | Kind::Real | Kind::Str))
    {
        writeln!(
            bind,
            "        crate::parser::entity::Attribute::Typed {{ type_name, value }} => match (type_name.as_str(), value.as_ref()) {{"
        )
        .unwrap();
        for (m, v, k) in &mems {
            let tag = m.to_uppercase();
            match k {
                Kind::Enum(a) => {
                    token_enums.insert(a.clone());
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Enum(t)) => Some({rtq}::{v}({a}_from_token(t))),").unwrap();
                }
                Kind::Real => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Real(x)) => Some({rtq}::{v}(*x)),").unwrap();
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Integer(x)) => Some({rtq}::{v}(*x as f64)),").unwrap();
                }
                Kind::Str => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::String(s)) => Some({rtq}::{v}(s.clone())),").unwrap();
                }
                _ => {}
            }
        }
        writeln!(bind, "            _ => None,").unwrap();
        writeln!(bind, "        }},").unwrap();
    }
    for (_, v, k) in &mems {
        if let Kind::Enum(a) = k {
            writeln!(
                bind,
                "        crate::parser::entity::Attribute::Enum(t) => Some({rtq}::{v}({a}_from_token(t))),"
            )
            .unwrap();
        }
    }
    writeln!(bind, "        _ => None,").unwrap();
    writeln!(bind, "    }}\n}}\n").unwrap();

    // <sel>_emit
    writeln!(
        serialize,
        "fn {sel}_emit(v: &{rtq}) -> crate::parser::entity::Attribute {{"
    )
    .unwrap();
    writeln!(serialize, "    match v {{").unwrap();
    for (m, v, k) in &mems {
        let tag = m.to_uppercase();
        let arm = match k {
            Kind::Ref => {
                format!("{rtq}::{v}(step) => crate::parser::entity::Attribute::EntityRef(*step),")
            }
            Kind::Enum(a) => format!(
                "{rtq}::{v}(t) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Enum({a}_to_token(t))) }},"
            ),
            Kind::Real => format!(
                "{rtq}::{v}(x) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Real(*x)) }},"
            ),
            Kind::Str => format!(
                "{rtq}::{v}(s) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::String(s.clone())) }},"
            ),
            _ => panic!("gen-early: select `{sel}` member kind unsupported"),
        };
        writeln!(serialize, "        {arm}").unwrap();
    }
    writeln!(serialize, "    }}\n}}\n").unwrap();
}

/// Emit token-based helpers (`<enum>_from_token` in bind, `<enum>_to_token` in
/// serialize) for a `catch_all` ENUM used as a SELECT member.
fn emit_token_enum(ctx: &Ctx, alias: &str, bind: &mut String, serialize: &mut String) {
    let h = ctx.enum_hint(alias);
    let rt = &h.rust_type;
    let catch = h
        .catch_all
        .as_ref()
        .unwrap_or_else(|| panic!("gen-early: enum `{alias}` used in a SELECT needs `catch_all`"));
    writeln!(bind, "fn {alias}_from_token(t: &str) -> {rt} {{").unwrap();
    writeln!(bind, "    match t {{").unwrap();
    for (member, variant) in &h.variants {
        writeln!(
            bind,
            "        \"{}\" => {rt}::{variant},",
            member.to_uppercase()
        )
        .unwrap();
    }
    writeln!(bind, "        other => {rt}::{catch}(other.to_owned()),").unwrap();
    writeln!(bind, "    }}\n}}\n").unwrap();

    writeln!(serialize, "fn {alias}_to_token(v: &{rt}) -> String {{").unwrap();
    writeln!(serialize, "    match v {{").unwrap();
    for (member, variant) in &h.variants {
        writeln!(
            serialize,
            "        {rt}::{variant} => \"{}\".into(),",
            member.to_uppercase()
        )
        .unwrap();
    }
    writeln!(serialize, "        {rt}::{catch}(s) => s.clone(),").unwrap();
    writeln!(serialize, "    }}\n}}\n").unwrap();
}

/// Emit a synthesized `Early*` enum (model def) + `bind_<alias>` / `<alias>_attr`
/// for a hint-less standalone-field ENUM. Variant = `pascal(member)`, STEP token
/// = `member.to_uppercase()`. Strict: an unknown token errors (EXPRESS ENUMs are
/// closed). Mirrors the hinted path but with a generated type instead of L2 reuse.
fn emit_synth_enum(
    ctx: &Ctx,
    alias: &str,
    model: &mut String,
    bind: &mut String,
    serialize: &mut String,
) {
    let aliased = ctx
        .schema
        .types
        .get(alias)
        .unwrap_or_else(|| panic!("gen-early: enum `{alias}` not a type alias"))
        .aliased
        .clone();
    let members = enum_members(aliased.trim())
        .unwrap_or_else(|| panic!("gen-early: enum `{alias}` not an ENUM"));
    let rt = format!("Early{}", pascal(alias));
    // (STEP token, Rust variant)
    let mems: Vec<(String, String)> = members
        .iter()
        .map(|m| (m.to_uppercase(), pascal(m)))
        .collect();

    // model enum
    writeln!(model, "/// L1 ENUM `{alias}` (generated).").unwrap();
    writeln!(model, "#[derive(Debug, Clone, PartialEq)]").unwrap();
    writeln!(model, "pub(crate) enum {rt} {{").unwrap();
    for (_, v) in &mems {
        writeln!(model, "    {v},").unwrap();
    }
    writeln!(model, "}}\n").unwrap();

    // bind helper
    writeln!(
        bind,
        "fn bind_{alias}(attrs: &[crate::parser::entity::Attribute], index: usize, entity_id: u64, field: &'static str) -> Result<super::model::{rt}, crate::ir::error::ConvertError> {{"
    )
    .unwrap();
    writeln!(
        bind,
        "    match crate::ir::attr::read_enum(attrs, index, entity_id, field)? {{"
    )
    .unwrap();
    for (tag, v) in &mems {
        writeln!(bind, "        \"{tag}\" => Ok(super::model::{rt}::{v}),").unwrap();
    }
    writeln!(
        bind,
        "        other => Err(crate::ir::error::ConvertError::UnexpectedEntityForm {{ entity_id, detail: format!(\"{{field}}: unknown {alias} '.{{other}}.'\") }}),"
    )
    .unwrap();
    writeln!(bind, "    }}\n}}\n").unwrap();

    // serialize helper
    writeln!(
        serialize,
        "fn {alias}_attr(v: super::model::{rt}) -> crate::parser::entity::Attribute {{"
    )
    .unwrap();
    writeln!(
        serialize,
        "    crate::parser::entity::Attribute::Enum(match v {{"
    )
    .unwrap();
    for (tag, v) in &mems {
        writeln!(serialize, "        super::model::{rt}::{v} => \"{tag}\",").unwrap();
    }
    writeln!(serialize, "    }}.into())\n}}\n").unwrap();
}

fn field_ty(ctx: &Ctx, k: &Kind) -> String {
    match k {
        Kind::Ref => "u64".into(),
        Kind::VecRef => "Vec<u64>".into(),
        Kind::Real => "f64".into(),
        Kind::Int => "i64".into(),
        Kind::Bool => "bool".into(),
        Kind::Str => "String".into(),
        // Hinted ENUM reuses the L2 type; hint-less ENUM uses the synthesized
        // `Early*` (defined in the same generated/model.rs, so referenced bare).
        Kind::Enum(alias) => match ctx.mapping.enums.get(alias) {
            Some(h) => h.rust_type.clone(),
            None => format!("Early{}", pascal(alias)),
        },
        Kind::Select(alias) => ctx.select_hint(alias).rust_type.clone(),
    }
}

fn bind_expr(k: &Kind, i: usize, field: &str) -> String {
    match k {
        Kind::Ref => {
            format!("crate::ir::attr::read_entity_ref(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::VecRef => {
            format!("crate::ir::attr::read_entity_ref_list(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Real => format!("crate::ir::attr::read_real(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Int => format!("crate::ir::attr::read_integer(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Bool => format!("crate::ir::attr::read_bool(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Str => {
            format!(
                "crate::ir::attr::read_string_or_unset(attrs, {i}, entity_id, \"{field}\")?.to_owned()"
            )
        }
        Kind::Enum(alias) => format!("bind_{alias}(attrs, {i}, entity_id, \"{field}\")?"),
        // Select fields drop the whole entity on `None`, so they are emitted in
        // the let-form path (see `main`), never via this expression helper.
        Kind::Select(_) => unreachable!("Select handled in the let-form bind path"),
    }
}

fn serialize_expr(k: &Kind, field: &str) -> String {
    match k {
        Kind::Ref => format!("crate::parser::entity::Attribute::EntityRef(l1.{field})"),
        Kind::VecRef => format!(
            "crate::parser::entity::Attribute::List(l1.{field}.iter().map(|&s| crate::parser::entity::Attribute::EntityRef(s)).collect())"
        ),
        Kind::Real => format!("crate::parser::entity::Attribute::Real(l1.{field})"),
        Kind::Int => format!("crate::parser::entity::Attribute::Integer(l1.{field})"),
        Kind::Bool => format!("bool_attr(l1.{field})"),
        Kind::Str => format!("crate::parser::entity::Attribute::String(l1.{field}.clone())"),
        Kind::Enum(alias) => format!("{alias}_attr(l1.{field})"),
        Kind::Select(alias) => format!("{alias}_emit(&l1.{field})"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The real `schema/early.toml` — the authority the generator reads.
    fn schema() -> EarlyToml {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        toml::from_str(
            &std::fs::read_to_string(format!("{root}/schema/early.toml")).expect("read early.toml"),
        )
        .expect("parse early.toml")
    }

    fn names(s: &EarlyToml, ent: &str) -> Vec<String> {
        s.flattened_attrs(ent)
            .iter()
            .map(|a| a.name.clone())
            .collect()
    }

    /// Single inherited attr, empty own — flattening yields just the supertype's.
    #[test]
    fn flatten_single_inheritance() {
        let s = schema();
        assert_eq!(names(&s, "geometric_representation_item"), ["name"]);
    }

    /// Inherited `name` (from representation_item) precedes the entity's own.
    #[test]
    fn flatten_inherited_before_own() {
        let s = schema();
        let flat = s.flattened_attrs("styled_item");
        assert_eq!(
            flat.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            ["name", "styles", "item"]
        );
        // ty is carried through from the supertype's own_attrs.
        assert_eq!(flat[0].ty, "label");
    }

    /// Three-level chain: representation_item -> geometric_representation_item
    /// -> text_literal.
    #[test]
    fn flatten_deep_chain() {
        let s = schema();
        assert_eq!(
            names(&s, "text_literal"),
            ["name", "literal", "placement", "alignment", "path", "font"]
        );
    }

    /// One of the v0..v2 generated entities — parent `founded_item` is empty, so
    /// flattening is a no-op (guarantees the existing 6 regenerate byte-identical).
    #[test]
    fn flatten_noop_for_founded_item_subtype() {
        let s = schema();
        assert_eq!(
            names(&s, "point_style"),
            ["name", "marker", "marker_size", "marker_colour"]
        );
    }

    /// Multiple inheritance: parents left-to-right, and same-named attrs declared
    /// by *different* parents both survive (dedup is per-entity, not per-attr).
    #[test]
    fn flatten_multi_parent_keeps_collisions() {
        let s = schema();
        // document_file : (document, characterized_object), own_attrs empty.
        // document -> [id, name, description, kind]; characterized_object ->
        // [name, description]. Both `name`/`description` appear twice.
        assert_eq!(
            names(&s, "document_file"),
            ["id", "name", "description", "kind", "name", "description"]
        );
    }

    /// Diamond: a common ancestor reached via two parent paths contributes its
    /// attrs once, at first encounter (deduped by entity name via `visited`).
    #[test]
    fn flatten_diamond_dedups_shared_ancestor() {
        let attr = |n: &str| Attr {
            name: n.to_string(),
            ty: "string".to_string(),
        };
        let ent = |parents: &[&str], own: &[&str]| Entity {
            parents: parents.iter().map(|p| p.to_string()).collect(),
            own_attrs: own.iter().map(|n| attr(n)).collect(),
        };
        let mut entity = BTreeMap::new();
        entity.insert("gp".to_string(), ent(&[], &["g"]));
        entity.insert("a".to_string(), ent(&["gp"], &["x"]));
        entity.insert("b".to_string(), ent(&["gp"], &["y"]));
        entity.insert("leaf".to_string(), ent(&["a", "b"], &["z"]));
        let s = EarlyToml {
            entity,
            types: BTreeMap::new(),
        };
        // gp's `g` appears once (via a's chain), not twice.
        assert_eq!(names(&s, "leaf"), ["g", "x", "y", "z"]);
    }

    fn ctx_from(schema: EarlyToml) -> Ctx {
        Ctx {
            schema,
            mapping: Mapping {
                generate: vec![],
                enums: BTreeMap::new(),
                selects: BTreeMap::new(),
            },
        }
    }

    /// `integer` (and aliases resolving to it) classify as `Int`.
    #[test]
    fn classify_integer_and_aliases() {
        let mut types = BTreeMap::new();
        types.insert(
            "positive_integer".to_string(),
            TypeDef {
                aliased: "integer".to_string(),
            },
        );
        let ctx = ctx_from(EarlyToml {
            entity: BTreeMap::new(),
            types,
        });
        assert!(matches!(ctx.classify("integer", 0), Kind::Int));
        assert!(matches!(ctx.classify("positive_integer", 0), Kind::Int));
    }

    /// ENUM member parsing + variant naming.
    #[test]
    fn enum_members_and_pascal() {
        assert_eq!(
            enum_members("ENUM(ahead, exact, behind)").unwrap(),
            ["ahead", "exact", "behind"]
        );
        assert_eq!(pascal("not_actuated"), "NotActuated");
        assert!(enum_members("SELECT(a, b)").is_none());
    }

    /// Hint-less ENUM auto-synthesis: model enum + strict bind/serialize helpers.
    #[test]
    fn synth_enum_emits_expected() {
        // `actuated_direction` is hint-less (no [enum.*] in mapping.toml).
        let ctx = ctx_from(schema());
        let (mut m, mut b, mut s) = (String::new(), String::new(), String::new());
        emit_synth_enum(&ctx, "actuated_direction", &mut m, &mut b, &mut s);
        assert!(m.contains("pub(crate) enum EarlyActuatedDirection"));
        assert!(m.contains("Bidirectional"));
        assert!(m.contains("NotActuated"));
        assert!(
            b.contains("\"NOT_ACTUATED\" => Ok(super::model::EarlyActuatedDirection::NotActuated)")
        );
        assert!(
            s.contains("super::model::EarlyActuatedDirection::NotActuated => \"NOT_ACTUATED\"")
        );
    }
}
