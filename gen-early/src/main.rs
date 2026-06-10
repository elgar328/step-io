//! `gen-early` — generates the mechanical part of the EarlyModel (L1) layer
//! (`src/early/generated/{model,bind,serialize}.rs`) from `schema/early.toml`
//! + `schema/mapping.toml`. The hand-written `lower`/`lift` (semantic mapping)
//! and the `early_id!` ids stay hand-coded.
//!
//! Run from anywhere: `cargo run -p gen-early`. Output is committed; this is a
//! dev tool, not a `build.rs`. v0 scope: entities with no SELECT / OPTIONAL /
//! inherited attributes (the `generate` list in mapping.toml).

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
    #[serde(default)]
    own_attrs: Vec<Attr>,
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
}

// ---- classification ----

/// How an attribute's resolved `ty` maps to Rust / bind / serialize.
enum Kind {
    Ref,          // entity ref (incl. all-entity SELECT) -> u64
    VecRef,       // aggregation of refs -> Vec<u64>
    Real,         // -> f64
    Bool,         // -> bool
    Str,          // -> String
    Enum(String), // ENUM type-alias name (looked up in mapping.enums)
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
        assert!(
            !t.starts_with("OPTIONAL "),
            "gen-early v1 cannot handle OPTIONAL ty `{t}` yet (v2)"
        );
        if let Some(inner) = agg_inner(t) {
            assert!(
                self.ref_like(inner, 0),
                "gen-early v1: aggregation of non-ref `{inner}` not supported yet"
            );
            return Kind::VecRef;
        }
        match t {
            "real" | "number" => return Kind::Real,
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
                assert!(
                    members.iter().all(|m| self.ref_like(m, 0)),
                    "gen-early v1: mixed SELECT `{t}` not supported yet (v2)"
                );
                return Kind::Ref; // all-entity SELECT -> bare ref
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
    // Enums needing generated bind/serialize helpers (dedup across entities).
    let mut used_enums: BTreeSet<String> = BTreeSet::new();
    let mut any_bool = false;

    for ent_name in &ctx.mapping.generate {
        let ent = ctx
            .schema
            .entity
            .get(ent_name)
            .unwrap_or_else(|| panic!("gen-early: entity `{ent_name}` not in early.toml"));
        let type_name = format!("Early{}", pascal(ent_name));
        let step_name = ent_name.to_uppercase();
        let attrs: Vec<(&Attr, Kind)> = ent
            .own_attrs
            .iter()
            .map(|a| (a, ctx.classify(&a.ty, 0)))
            .collect();

        // model struct
        writeln!(model, "/// L1 `{step_name}` (generated).").unwrap();
        writeln!(model, "#[derive(Debug, Clone, PartialEq)]").unwrap();
        writeln!(model, "pub(crate) struct {type_name} {{").unwrap();
        for (a, k) in &attrs {
            writeln!(model, "    pub(crate) {}: {},", a.name, field_ty(&ctx, k)).unwrap();
        }
        writeln!(model, "}}\n").unwrap();

        // bind fn
        writeln!(
            bind,
            "pub(crate) fn bind_{ent_name}(entity_id: u64, attrs: &[crate::parser::entity::Attribute]) -> Result<super::model::{type_name}, crate::ir::error::ConvertError> {{"
        )
        .unwrap();
        writeln!(
            bind,
            "    crate::ir::attr::check_count(attrs, {}, entity_id, \"{step_name}\")?;",
            attrs.len()
        )
        .unwrap();
        writeln!(bind, "    Ok(super::model::{type_name} {{").unwrap();
        for (i, (a, k)) in attrs.iter().enumerate() {
            writeln!(bind, "        {}: {},", a.name, bind_expr(k, i, &a.name)).unwrap();
        }
        writeln!(bind, "    }})\n}}\n").unwrap();

        // serialize fn
        writeln!(
            serialize,
            "pub(crate) fn serialize_{ent_name}(buf: &mut crate::writer::buffer::WriteBuffer, l1: &super::model::{type_name}) -> u64 {{"
        )
        .unwrap();
        writeln!(serialize, "    buf.push_simple(\"{step_name}\", vec![").unwrap();
        for (a, k) in &attrs {
            writeln!(serialize, "        {},", serialize_expr(k, &a.name)).unwrap();
            if matches!(k, Kind::Bool) {
                any_bool = true;
            }
            if let Kind::Enum(alias) = k {
                used_enums.insert(alias.clone());
            }
        }
        writeln!(serialize, "    ])\n}}\n").unwrap();
    }

    // generated enum helpers
    for alias in &used_enums {
        let h = ctx.enum_hint(alias);
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

fn field_ty(ctx: &Ctx, k: &Kind) -> String {
    match k {
        Kind::Ref => "u64".into(),
        Kind::VecRef => "Vec<u64>".into(),
        Kind::Real => "f64".into(),
        Kind::Bool => "bool".into(),
        Kind::Str => "String".into(),
        Kind::Enum(alias) => ctx.enum_hint(alias).rust_type.clone(),
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
        Kind::Bool => format!("crate::ir::attr::read_bool(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Str => {
            format!(
                "crate::ir::attr::read_string_or_unset(attrs, {i}, entity_id, \"{field}\")?.to_owned()"
            )
        }
        Kind::Enum(alias) => format!("bind_{alias}(attrs, {i}, entity_id, \"{field}\")?"),
    }
}

fn serialize_expr(k: &Kind, field: &str) -> String {
    match k {
        Kind::Ref => format!("crate::parser::entity::Attribute::EntityRef(l1.{field})"),
        Kind::VecRef => format!(
            "crate::parser::entity::Attribute::List(l1.{field}.iter().map(|&s| crate::parser::entity::Attribute::EntityRef(s)).collect())"
        ),
        Kind::Real => format!("crate::parser::entity::Attribute::Real(l1.{field})"),
        Kind::Bool => format!("bool_attr(l1.{field})"),
        Kind::Str => format!("crate::parser::entity::Attribute::String(l1.{field}.clone())"),
        Kind::Enum(alias) => format!("{alias}_attr(l1.{field})"),
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
