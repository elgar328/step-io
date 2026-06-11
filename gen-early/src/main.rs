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
    /// Diagnostic flip: when true, generate *every* entity the codegen can
    /// handle (filtered by [`Ctx::entity_supported`]) + a coverage report,
    /// instead of just the wired `generate` list. Committed as `false`.
    #[serde(default)]
    generate_all: bool,
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
#[derive(Debug)]
enum Kind {
    Ref,            // entity ref (incl. all-entity SELECT) -> u64
    Vec(Box<Kind>), // aggregation -> Vec<inner> (LIST/SET/BAG/ARRAY OF inner)
    Real,           // -> f64
    Int,            // -> i64
    Bool,           // -> bool
    Logical,        // -> crate::ir::geometry::Logical (.T./.F./.U.)
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

/// Split a leading `OPTIONAL ` off a `ty`, returning `(is_optional, inner_ty)`.
/// EXPRESS does not nest `OPTIONAL`, so a single strip suffices.
fn strip_optional(ty: &str) -> (bool, &str) {
    match ty.trim().strip_prefix("OPTIONAL ") {
        Some(inner) => (true, inner.trim()),
        None => (false, ty.trim()),
    }
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
    /// (after alias resolution), ENUMs (hinted or synthesized), all-entity
    /// SELECTs (-> ref), mixed SELECTs (-> `Select`) and aggregations of refs
    /// (-> `Vec<u64>`). `OPTIONAL` is stripped here defensively but is normally
    /// handled by `main` (tracked as a separate flag for `Option<T>`).
    fn classify(&self, ty: &str, depth: u32) -> Kind {
        assert!(
            depth < MAX_DEPTH,
            "gen-early: ty `{ty}` resolution too deep (cycle?)"
        );
        let t = ty.trim();
        // `main` strips `OPTIONAL` (via `strip_optional`) and tracks it as a
        // separate `optional` flag so the field becomes a faithful `Option<T>`;
        // this defensive strip keeps `classify` correct on any raw-ty path.
        if let Some(inner) = t.strip_prefix("OPTIONAL ") {
            return self.classify(inner.trim(), depth + 1);
        }
        if let Some(inner) = agg_inner(t) {
            // Classify the inner type recursively; which inner kinds are actually
            // emittable is decided at emit time (v4.4 = ref/scalar; enum/select/
            // nested deferred to v4.5).
            return Kind::Vec(Box::new(self.classify(inner, depth + 1)));
        }
        match t {
            "real" | "number" => return Kind::Real,
            "integer" => return Kind::Int,
            "boolean" => return Kind::Bool,
            "logical" => return Kind::Logical,
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
                // Mixed SELECT: hinted -> `emit_select`, hint-less -> auto-synth
                // via `emit_select_synth` (decided at emit time).
                return Kind::Select(t.to_string());
            }
            return self.classify(a, depth + 1);
        }
        panic!("gen-early: cannot resolve ty token `{t}`");
    }

    // ---- support predicate (diagnostic flip; mirrors emit-time panics) ----
    // Separate fallible classify so the real `classify` (the 6 entities' emit
    // path) stays untouched. Drift vs the emit path is caught loudly by the
    // emit-time panic backstop during a local flip run.

    /// Like [`classify`](Self::classify) but `None` (instead of panic) on an
    /// unresolvable token (e.g. `binary`) or a too-deep / cyclic resolution.
    fn try_classify(&self, ty: &str, depth: u32) -> Option<Kind> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let t = ty.trim();
        if let Some(inner) = t.strip_prefix("OPTIONAL ") {
            return self.try_classify(inner.trim(), depth + 1);
        }
        if let Some(inner) = agg_inner(t) {
            return Some(Kind::Vec(Box::new(self.try_classify(inner, depth + 1)?)));
        }
        match t {
            "real" | "number" => return Some(Kind::Real),
            "integer" => return Some(Kind::Int),
            "boolean" => return Some(Kind::Bool),
            "logical" => return Some(Kind::Logical),
            "string" => return Some(Kind::Str),
            _ => {}
        }
        if self.schema.entity.contains_key(t) {
            return Some(Kind::Ref);
        }
        let td = self.schema.types.get(t)?;
        let a = td.aliased.trim();
        if a.starts_with("ENUM(") {
            return Some(Kind::Enum(t.to_string()));
        }
        if let Some(members) = select_members(a) {
            if members.iter().all(|m| self.ref_like(m, 0)) {
                return Some(Kind::Ref);
            }
            return Some(Kind::Select(t.to_string()));
        }
        self.try_classify(a, depth + 1)
    }

    /// Whether a classified attribute kind (with its `OPTIONAL` flag) is emit-
    /// able by the current codegen — i.e. would not hit an emit-time panic.
    fn emittable(&self, k: &Kind, optional: bool) -> bool {
        match k {
            // read_optional_* exist for these, so optional is fine too.
            Kind::Ref | Kind::Real | Kind::Int | Kind::Str => true,
            // scalar bind exists; optional deferred (no read_optional_*).
            Kind::Bool | Kind::Logical | Kind::Enum(_) => !optional,
            // scalar/ref element only; optional aggregation deferred.
            Kind::Vec(inner) => {
                !optional && matches!(**inner, Kind::Ref | Kind::Real | Kind::Int | Kind::Str)
            }
            // optional select is handled by the v4.2 let-form; both need a
            // supported select.
            Kind::Select(sel) => self.select_supported(sel),
        }
    }

    /// Whether a mixed SELECT is emittable — hinted ones always; hint-less ones
    /// only in the shape [`emit_select_synth`] accepts.
    fn select_supported(&self, sel: &str) -> bool {
        if self.mapping.selects.contains_key(sel) {
            return true; // hinted -> emit_select
        }
        let Some(td) = self.schema.types.get(sel) else {
            return false;
        };
        let Some(members) = select_members(td.aliased.trim()) else {
            return false;
        };
        let mut entity_members = 0;
        for m in members {
            let Some(k) = self.try_classify(m, 0) else {
                return false; // unresolved member
            };
            match k {
                Kind::Ref => entity_members += 1,
                Kind::Real | Kind::Int | Kind::Str | Kind::Bool => {}
                // hinted enum member -> dual representation, deferred.
                Kind::Enum(a) if self.mapping.enums.contains_key(&a) => return false,
                Kind::Enum(_) => {}
                // logical / aggregation / nested mixed select members deferred.
                Kind::Logical | Kind::Vec(_) | Kind::Select(_) => return false,
            }
        }
        entity_members <= 1
    }

    /// `Ok` if every flattened attribute of `ent` is emittable, else the first
    /// failure's reason tag (for the coverage report).
    fn entity_supported(&self, ent: &str) -> Result<(), String> {
        let attrs = self.schema.flattened_attrs(ent);
        // Multiple inheritance can yield two attrs sharing a name (e.g. both a
        // `document` and a `characterized_object` parent declare `name`); a Rust
        // struct can't have duplicate fields. Deferred (would need qualified
        // field names).
        let mut seen = BTreeSet::new();
        for a in &attrs {
            if !seen.insert(a.name.as_str()) {
                return Err("dup-attr".to_string());
            }
        }
        for a in &attrs {
            let (optional, inner) = strip_optional(&a.ty);
            match self.try_classify(inner, 0) {
                None => return Err("unresolved-token".to_string()),
                Some(k) if !self.emittable(&k, optional) => {
                    return Err(unsupported_reason(&k, optional));
                }
                _ => {}
            }
        }
        Ok(())
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

/// Short tag for a [`Kind`] (coverage report).
fn kind_tag(k: &Kind) -> &'static str {
    match k {
        Kind::Ref => "ref",
        Kind::Real => "real",
        Kind::Int => "int",
        Kind::Bool => "bool",
        Kind::Logical => "logical",
        Kind::Str => "str",
        Kind::Enum(_) => "enum",
        Kind::Select(_) => "select",
        Kind::Vec(_) => "vec",
    }
}

/// Categorize why an unsupported (kind, optional) attribute can't be emitted.
fn unsupported_reason(k: &Kind, optional: bool) -> String {
    match k {
        Kind::Vec(inner) if matches!(**inner, Kind::Vec(_)) => "nested-agg".to_string(),
        Kind::Vec(inner) => format!("vec-{}", kind_tag(inner)),
        Kind::Select(_) => "select-edge".to_string(),
        _ if optional => format!("opt-{}", kind_tag(k)),
        _ => kind_tag(k).to_string(),
    }
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
            "{HEADER}#![allow(dead_code, unused_variables, clippy::too_many_lines, clippy::enum_variant_names, clippy::struct_field_names)]\n\n"
        )
    } else {
        HEADER.to_string()
    };
    let mut model = head.clone();
    let mut bind = head.clone();
    let mut serialize = head;
    // Helpers to emit once after the loop (dedup across entities).
    let mut used_enums: BTreeSet<String> = BTreeSet::new(); // standalone field ENUMs
    let mut used_selects: BTreeSet<String> = BTreeSet::new(); // mixed SELECTs
    let mut any_bool = false;

    for ent_name in &targets {
        assert!(
            ctx.schema.entity.contains_key(ent_name),
            "gen-early: entity `{ent_name}` not in early.toml"
        );
        let type_name = format!("Early{}", pascal(ent_name));
        let step_name = ent_name.to_uppercase();
        // Full Part21 positional attribute list (inherited supertype attrs
        // prepended, then own); see `EarlyToml::flattened_attrs`. Each entry also
        // carries whether the attr is `OPTIONAL` (-> faithful `Option<T>` in L1).
        let attrs: Vec<(&Attr, Kind, bool)> = ctx
            .schema
            .flattened_attrs(ent_name)
            .into_iter()
            .map(|a| {
                let (optional, inner) = strip_optional(&a.ty);
                (a, ctx.classify(inner, 0), optional)
            })
            .collect();
        let has_select = attrs.iter().any(|(_, k, _)| matches!(k, Kind::Select(_)));

        // model struct
        writeln!(model, "/// L1 `{step_name}` (generated).").unwrap();
        writeln!(model, "#[derive(Debug, Clone, PartialEq)]").unwrap();
        writeln!(model, "pub(crate) struct {type_name} {{").unwrap();
        for (a, k, opt) in &attrs {
            writeln!(
                model,
                "    pub(crate) {}: {},",
                a.name,
                field_ty_full(&ctx, k, *opt)
            )
            .unwrap();
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
            for (i, (a, k, opt)) in attrs.iter().enumerate() {
                match (k, opt) {
                    // Optional SELECT: `$`/`*` -> None (entity survives); a present
                    // but unrecognized form -> drop the whole entity (preserving
                    // the required-select behavior for malformed input).
                    (Kind::Select(sel), true) => {
                        writeln!(bind, "    let {} = match &attrs[{i}] {{", a.name).unwrap();
                        writeln!(
                            bind,
                            "        crate::parser::entity::Attribute::Unset | crate::parser::entity::Attribute::Derived => None,"
                        )
                        .unwrap();
                        writeln!(
                            bind,
                            "        _ => match bind_{sel}(&attrs[{i}]) {{ Some(v) => Some(v), None => return Ok(None) }},"
                        )
                        .unwrap();
                        writeln!(bind, "    }};").unwrap();
                    }
                    // Required SELECT: drop the whole entity on an unrecognized form.
                    (Kind::Select(sel), false) => {
                        writeln!(
                            bind,
                            "    let Some({}) = bind_{sel}(&attrs[{i}]) else {{ return Ok(None); }};",
                            a.name
                        )
                        .unwrap();
                    }
                    _ => {
                        writeln!(
                            bind,
                            "    let {} = {};",
                            a.name,
                            bind_expr_full(k, i, &a.name, *opt)
                        )
                        .unwrap();
                    }
                }
            }
            writeln!(bind, "    Ok(Some(super::model::{type_name} {{").unwrap();
            for (a, _, _) in &attrs {
                writeln!(bind, "        {},", a.name).unwrap();
            }
            writeln!(bind, "    }}))\n}}\n").unwrap();
        } else {
            writeln!(bind, "    Ok(super::model::{type_name} {{").unwrap();
            for (i, (a, k, opt)) in attrs.iter().enumerate() {
                writeln!(
                    bind,
                    "        {}: {},",
                    a.name,
                    bind_expr_full(k, i, &a.name, *opt)
                )
                .unwrap();
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
        for (a, k, opt) in &attrs {
            writeln!(
                serialize,
                "        {},",
                serialize_expr_full(k, &a.name, *opt)
            )
            .unwrap();
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

    // generated mixed-SELECT helpers. Hinted SELECTs use `emit_select` (token-
    // based enum helpers, collected in `token_enums`); hint-less SELECTs use
    // `emit_select_synth` (inline enum maps). Synth enum *models* are deduped
    // across selects and standalone fields via `emitted_models`.
    let mut token_enums: BTreeSet<String> = BTreeSet::new();
    let mut emitted_models: BTreeSet<String> = BTreeSet::new();
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

/// Emit a synthesized `Early*` enum + `bind_<sel>` / `<sel>_emit` for a
/// **hint-less** mixed SELECT (no `[select.*]`). Variant = `pascal(member)`,
/// member kind = [`Ctx::classify`]. Supported members: entity (≤1, bare `#N`),
/// scalar (real/int/str/bool), and hint-less ENUM (synth enum, token map inlined
/// so no `from_token` helper). Deferred shapes panic loudly (none reached by the
/// current generate-list): ≥2 entity (combined-ref), aggregation member, nested
/// mixed SELECT, or a hinted ENUM member.
fn emit_select_synth(
    ctx: &Ctx,
    sel: &str,
    model: &mut String,
    bind: &mut String,
    serialize: &mut String,
    emitted_models: &mut BTreeSet<String>,
) {
    let rt = format!("Early{}", pascal(sel));
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
        .map(|&m| (m, pascal(m), ctx.classify(m, 0)))
        .collect();

    let count = |pred: fn(&Kind) -> bool| mems.iter().filter(|(_, _, k)| pred(k)).count();
    assert!(
        count(|k| matches!(k, Kind::Ref)) <= 1,
        "gen-early: select `{sel}` has >=2 entity members (multi-entity combined-ref: v4.x)"
    );
    for (m, _, k) in &mems {
        match k {
            Kind::Vec(_) => {
                panic!("gen-early: select `{sel}` member `{m}` is an aggregation (v4.x)")
            }
            Kind::Select(_) => {
                panic!("gen-early: select `{sel}` member `{m}` is a nested mixed select (v4.x)")
            }
            Kind::Enum(a) if ctx.mapping.enums.contains_key(a) => panic!(
                "gen-early: select `{sel}` member `{m}` is a hinted enum in a hint-less select (v4.x)"
            ),
            Kind::Logical => {
                panic!("gen-early: select `{sel}` member `{m}` is a logical (v4.x)")
            }
            _ => {}
        }
    }

    let payload = |k: &Kind| -> String {
        match k {
            Kind::Ref => "u64".into(),
            Kind::Real => "f64".into(),
            Kind::Int => "i64".into(),
            Kind::Bool => "bool".into(),
            Kind::Str => "String".into(),
            Kind::Enum(a) => format!("super::model::Early{}", pascal(a)),
            _ => unreachable!("filtered by the deferred-shape checks above"),
        }
    };

    // synth enum models for ENUM members (deduped across selects / fields).
    for (_, _, k) in &mems {
        if let Kind::Enum(a) = k {
            write_synth_enum_model(ctx, a, model, emitted_models);
        }
    }

    // model enum
    writeln!(model, "/// L1 mixed SELECT `{sel}` (generated, hint-less).").unwrap();
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
    if mems.iter().any(|(_, _, k)| !matches!(k, Kind::Ref)) {
        writeln!(
            bind,
            "        crate::parser::entity::Attribute::Typed {{ type_name, value }} => match (type_name.as_str(), value.as_ref()) {{"
        )
        .unwrap();
        for (m, v, k) in &mems {
            let tag = m.to_uppercase();
            match k {
                Kind::Real => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Real(x)) => Some({rtq}::{v}(*x)),").unwrap();
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Integer(x)) => Some({rtq}::{v}(*x as f64)),").unwrap();
                }
                Kind::Int => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Integer(x)) => Some({rtq}::{v}(*x)),").unwrap();
                }
                Kind::Str => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::String(s)) => Some({rtq}::{v}(s.clone())),").unwrap();
                }
                Kind::Bool => {
                    writeln!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Enum(t)) => Some({rtq}::{v}(t.as_str() == \"T\")),").unwrap();
                }
                Kind::Enum(a) => {
                    let etype = format!("super::model::Early{}", pascal(a));
                    write!(bind, "            (\"{tag}\", crate::parser::entity::Attribute::Enum(t)) => match t.as_str() {{").unwrap();
                    for (etag, evar) in synth_enum_members(ctx, a) {
                        write!(bind, " \"{etag}\" => Some({rtq}::{v}({etype}::{evar})),").unwrap();
                    }
                    writeln!(bind, " _ => None }},").unwrap();
                }
                _ => {}
            }
        }
        writeln!(bind, "            _ => None,").unwrap();
        writeln!(bind, "        }},").unwrap();
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
            Kind::Real => format!(
                "{rtq}::{v}(x) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Real(*x)) }},"
            ),
            Kind::Int => format!(
                "{rtq}::{v}(x) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Integer(*x)) }},"
            ),
            Kind::Str => format!(
                "{rtq}::{v}(s) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::String(s.clone())) }},"
            ),
            Kind::Bool => format!(
                "{rtq}::{v}(b) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Enum(if *b {{ \"T\" }} else {{ \"F\" }}.into())) }},"
            ),
            Kind::Enum(a) => {
                let etype = format!("super::model::Early{}", pascal(a));
                let mut arms = String::new();
                for (etag, evar) in synth_enum_members(ctx, a) {
                    write!(arms, " {etype}::{evar} => \"{etag}\",").unwrap();
                }
                format!(
                    "{rtq}::{v}(e) => crate::parser::entity::Attribute::Typed {{ type_name: \"{tag}\".into(), value: Box::new(crate::parser::entity::Attribute::Enum(match e {{{arms} }}.into())) }},"
                )
            }
            _ => unreachable!("filtered by the deferred-shape checks above"),
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

/// Resolve a hint-less ENUM alias to its `(STEP token, Rust variant)` members.
fn synth_enum_members(ctx: &Ctx, alias: &str) -> Vec<(String, String)> {
    let aliased = ctx
        .schema
        .types
        .get(alias)
        .unwrap_or_else(|| panic!("gen-early: enum `{alias}` not a type alias"))
        .aliased
        .clone();
    enum_members(aliased.trim())
        .unwrap_or_else(|| panic!("gen-early: enum `{alias}` not an ENUM"))
        .iter()
        .map(|m| (m.to_uppercase(), pascal(m)))
        .collect()
}

/// Emit the synthesized `Early*` enum *model definition* once (deduped via
/// `emitted_models`). Shared by standalone-field ENUMs and hint-less SELECT
/// members so a model used both ways is defined exactly once.
fn write_synth_enum_model(
    ctx: &Ctx,
    alias: &str,
    model: &mut String,
    emitted_models: &mut BTreeSet<String>,
) {
    if !emitted_models.insert(alias.to_string()) {
        return;
    }
    let rt = format!("Early{}", pascal(alias));
    // Fieldless (unit variants) -> `Copy`, so the by-value `<alias>_attr`
    // serialize helper compiles for non-trivial entities too.
    writeln!(model, "/// L1 ENUM `{alias}` (generated).").unwrap();
    writeln!(model, "#[derive(Debug, Clone, Copy, PartialEq)]").unwrap();
    writeln!(model, "pub(crate) enum {rt} {{").unwrap();
    for (_, v) in synth_enum_members(ctx, alias) {
        writeln!(model, "    {v},").unwrap();
    }
    writeln!(model, "}}\n").unwrap();
}

/// Emit a synthesized `Early*` enum (model def + `bind_<alias>` / `<alias>_attr`)
/// for a hint-less standalone-field ENUM. Variant = `pascal(member)`, STEP token
/// = `member.to_uppercase()`. Strict: an unknown token errors (EXPRESS ENUMs are
/// closed). Mirrors the hinted path but with a generated type instead of L2 reuse.
fn emit_synth_enum(
    ctx: &Ctx,
    alias: &str,
    model: &mut String,
    bind: &mut String,
    serialize: &mut String,
    emitted_models: &mut BTreeSet<String>,
) {
    let rt = format!("Early{}", pascal(alias));
    let mems = synth_enum_members(ctx, alias);
    write_synth_enum_model(ctx, alias, model, emitted_models);

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
        Kind::Vec(inner) => format!("Vec<{}>", field_ty(ctx, inner)),
        Kind::Real => "f64".into(),
        Kind::Int => "i64".into(),
        Kind::Bool => "bool".into(),
        Kind::Logical => "crate::ir::geometry::Logical".into(),
        Kind::Str => "String".into(),
        // Hinted ENUM reuses the L2 type; hint-less ENUM uses the synthesized
        // `Early*` (defined in the same generated/model.rs, so referenced bare).
        Kind::Enum(alias) => match ctx.mapping.enums.get(alias) {
            Some(h) => h.rust_type.clone(),
            None => format!("Early{}", pascal(alias)),
        },
        // Hinted SELECT reuses the hint's type; hint-less SELECT uses the
        // synthesized `Early*` (parallel to the ENUM branch).
        Kind::Select(alias) => match ctx.mapping.selects.get(alias) {
            Some(h) => h.rust_type.clone(),
            None => format!("Early{}", pascal(alias)),
        },
    }
}

fn bind_expr(k: &Kind, i: usize, field: &str) -> String {
    match k {
        Kind::Ref => {
            format!("crate::ir::attr::read_entity_ref(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Vec(inner) => {
            let list = match &**inner {
                Kind::Ref => "read_entity_ref_list",
                Kind::Real => "read_real_list",
                Kind::Int => "read_integer_list",
                Kind::Str => "read_string_list",
                other => panic!("gen-early: aggregation of {other:?} not yet supported (v4.5)"),
            };
            format!("crate::ir::attr::{list}(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Real => format!("crate::ir::attr::read_real(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Int => format!("crate::ir::attr::read_integer(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Bool => format!("crate::ir::attr::read_bool(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Logical => {
            format!("crate::ir::attr::read_logical(attrs, {i}, entity_id, \"{field}\")?")
        }
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
        Kind::Vec(inner) => {
            // Ref arm reproduces the previous `VecRef` output byte-for-byte.
            let elem = match &**inner {
                Kind::Ref => "|&s| crate::parser::entity::Attribute::EntityRef(s)".to_string(),
                Kind::Real => "|&x| crate::parser::entity::Attribute::Real(x)".to_string(),
                Kind::Int => "|&x| crate::parser::entity::Attribute::Integer(x)".to_string(),
                Kind::Str => "|s| crate::parser::entity::Attribute::String(s.clone())".to_string(),
                other => panic!("gen-early: aggregation of {other:?} not yet supported (v4.5)"),
            };
            format!(
                "crate::parser::entity::Attribute::List(l1.{field}.iter().map({elem}).collect())"
            )
        }
        Kind::Real => format!("crate::parser::entity::Attribute::Real(l1.{field})"),
        Kind::Int => format!("crate::parser::entity::Attribute::Integer(l1.{field})"),
        Kind::Bool => format!("bool_attr(l1.{field})"),
        Kind::Logical => format!(
            "crate::parser::entity::Attribute::Enum(crate::ir::attr::logical_to_step(l1.{field}).into())"
        ),
        Kind::Str => format!("crate::parser::entity::Attribute::String(l1.{field}.clone())"),
        Kind::Enum(alias) => format!("{alias}_attr(l1.{field})"),
        Kind::Select(alias) => format!("{alias}_emit(&l1.{field})"),
    }
}

/// Field type, wrapping `Option<…>` for an `OPTIONAL` schema attribute (faithful
/// L1 optionality; `lower` decides any collapse).
fn field_ty_full(ctx: &Ctx, k: &Kind, optional: bool) -> String {
    let base = field_ty(ctx, k);
    if optional {
        format!("Option<{base}>")
    } else {
        base
    }
}

/// `bind` expression, optional-aware (non-`Select` kinds; `Select` is emitted
/// inline in `main`'s let-form path, never here).
fn bind_expr_full(k: &Kind, i: usize, field: &str, optional: bool) -> String {
    if optional {
        bind_expr_opt(k, i, field)
    } else {
        bind_expr(k, i, field)
    }
}

/// Optional `bind` for kinds with an existing `read_optional_*` helper. Other
/// kinds (`Bool`/`Logical`/`Enum`/`Vec`) are deferred — they need new helpers
/// that would be dead code until an entity uses them.
fn bind_expr_opt(k: &Kind, i: usize, field: &str) -> String {
    match k {
        Kind::Ref => {
            format!(
                "crate::ir::attr::read_optional_entity_ref(attrs, {i}, entity_id, \"{field}\")?"
            )
        }
        Kind::Real => {
            format!("crate::ir::attr::read_optional_real(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Int => {
            format!("crate::ir::attr::read_optional_integer(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Str => {
            format!("crate::ir::attr::read_optional_string(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Bool | Kind::Logical | Kind::Enum(_) | Kind::Vec(_) => {
            panic!("gen-early: OPTIONAL {k:?} not yet supported (v4.x)")
        }
        Kind::Select(_) => unreachable!("optional Select handled in the let-form bind path"),
    }
}

/// `serialize` expression, optional-aware: `Some` -> inner attribute, `None` ->
/// `$` (`Attribute::Unset`).
fn serialize_expr_full(k: &Kind, field: &str, optional: bool) -> String {
    if optional {
        serialize_expr_opt(k, field)
    } else {
        serialize_expr(k, field)
    }
}

fn serialize_expr_opt(k: &Kind, field: &str) -> String {
    let unset = "crate::parser::entity::Attribute::Unset";
    match k {
        Kind::Ref => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::EntityRef(v), None => {unset} }}"
        ),
        Kind::Real => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::Real(v), None => {unset} }}"
        ),
        Kind::Int => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::Integer(v), None => {unset} }}"
        ),
        Kind::Str => format!(
            "match &l1.{field} {{ Some(v) => crate::parser::entity::Attribute::String(v.clone()), None => {unset} }}"
        ),
        Kind::Select(alias) => {
            format!("match &l1.{field} {{ Some(v) => {alias}_emit(v), None => {unset} }}")
        }
        Kind::Bool | Kind::Logical | Kind::Enum(_) | Kind::Vec(_) => {
            panic!("gen-early: OPTIONAL {k:?} not yet supported (v4.x)")
        }
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
                generate_all: false,
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
        emit_synth_enum(
            &ctx,
            "actuated_direction",
            &mut m,
            &mut b,
            &mut s,
            &mut BTreeSet::new(),
        );
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

    /// `OPTIONAL ` is split off and tracked as a flag; the inner ty stays.
    #[test]
    fn strip_optional_splits_prefix() {
        assert_eq!(strip_optional("OPTIONAL colour"), (true, "colour"));
        assert_eq!(
            strip_optional("OPTIONAL SET OF curve"),
            (true, "SET OF curve")
        );
        assert_eq!(strip_optional("label"), (false, "label"));
    }

    /// Faithful `Option<T>` codegen for the supported kinds.
    #[test]
    fn optional_codegen() {
        let ctx = ctx_from(EarlyToml {
            entity: BTreeMap::new(),
            types: BTreeMap::new(),
        });
        // field wraps Option only when optional.
        assert_eq!(field_ty_full(&ctx, &Kind::Real, true), "Option<f64>");
        assert_eq!(field_ty_full(&ctx, &Kind::Real, false), "f64");
        // bind uses the read_optional_* helpers.
        assert!(bind_expr_full(&Kind::Int, 2, "x", true).contains("read_optional_integer"));
        assert!(bind_expr_full(&Kind::Ref, 3, "c", true).contains("read_optional_entity_ref"));
        // required path is unchanged.
        assert!(bind_expr_full(&Kind::Int, 2, "x", false).contains("read_integer(attrs"));
        // serialize maps None -> Unset.
        let s = serialize_expr_full(&Kind::Ref, "c", true);
        assert!(s.contains("Some(v) => crate::parser::entity::Attribute::EntityRef(v)"));
        assert!(s.contains("None => crate::parser::entity::Attribute::Unset"));
    }

    fn synth_select(sel: &str) -> (String, String, String) {
        let ctx = ctx_from(schema());
        let (mut m, mut b, mut s) = (String::new(), String::new(), String::new());
        emit_select_synth(&ctx, sel, &mut m, &mut b, &mut s, &mut BTreeSet::new());
        (m, b, s)
    }

    /// Hint-less SELECT with two string members — each disambiguated by its tag.
    #[test]
    fn synth_select_strings() {
        let (m, b, s) = synth_select("source_item");
        assert!(m.contains("pub(crate) enum EarlySourceItem"));
        assert!(m.contains("Identifier(String)"));
        assert!(m.contains("Message(String)"));
        assert!(b.contains(
            "(\"IDENTIFIER\", crate::parser::entity::Attribute::String(s)) => Some(super::model::EarlySourceItem::Identifier(s.clone())),"
        ));
        assert!(s.contains("EarlySourceItem::Message(s) =>"));
    }

    /// Hint-less SELECT with real members (measures) — `Real`/`Integer` accepted.
    #[test]
    fn synth_select_reals() {
        let (m, b, _) = synth_select("tolerance_deviation_select");
        assert!(m.contains("CurveToleranceDeviation(f64)"));
        assert!(b.contains("#[allow(clippy::cast_precision_loss)]"));
        assert!(b.contains("crate::parser::entity::Attribute::Integer(x)) => Some(super::model::EarlyToleranceDeviationSelect::"));
    }

    /// Hint-less SELECT with an ENUM member — synth enum model + inlined token map.
    #[test]
    fn synth_select_enum_member() {
        let (m, b, s) = synth_select("gps_filtration_type");
        // the member enum's model is emitted...
        assert!(m.contains("pub(crate) enum EarlyGeometricToleranceModifier"));
        // ...and the token map is inlined into the select's bind/serialize.
        assert!(b.contains("=> match t.as_str() {"));
        assert!(b.contains("=> Some(super::model::EarlyGpsFiltrationType::"));
        assert!(s.contains("super::model::EarlyGeometricToleranceModifier::"));
    }

    /// ≥2 entity members can't be disambiguated from bare `#N` — deferred (panic).
    #[test]
    #[should_panic(expected = "multi-entity")]
    fn synth_select_multi_entity_panics() {
        synth_select("trim_condition_select");
    }

    /// An aggregation member (LIST/SET) is deferred to v4.4 (panic). Uses a
    /// ≤1-entity select so the multi-entity assert doesn't fire first.
    #[test]
    #[should_panic(expected = "aggregation")]
    fn synth_select_aggregation_member_panics() {
        synth_select("item_identified_representation_usage_select");
    }

    fn empty_ctx() -> Ctx {
        ctx_from(EarlyToml {
            entity: BTreeMap::new(),
            types: BTreeMap::new(),
        })
    }

    /// Scalar aggregations classify to `Vec(<scalar>)`; nesting recurses.
    #[test]
    fn classify_scalar_aggregations() {
        let ctx = empty_ctx();
        assert!(
            matches!(ctx.classify("LIST OF real", 0), Kind::Vec(b) if matches!(*b, Kind::Real))
        );
        assert!(
            matches!(ctx.classify("SET OF integer", 0), Kind::Vec(b) if matches!(*b, Kind::Int))
        );
        // nested: Vec(Vec(Real)) — classified, but emit defers (v4.5).
        assert!(matches!(
            ctx.classify("LIST OF LIST OF real", 0),
            Kind::Vec(b) if matches!(*b, Kind::Vec(_))
        ));
    }

    /// `Vec<T>` codegen for scalar inner; the ref arm stays byte-identical.
    #[test]
    fn vec_scalar_codegen() {
        let ctx = empty_ctx();
        assert_eq!(field_ty(&ctx, &Kind::Vec(Box::new(Kind::Real))), "Vec<f64>");
        assert_eq!(
            field_ty(&ctx, &Kind::Vec(Box::new(Kind::Str))),
            "Vec<String>"
        );
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Real)), 1, "x").contains("read_real_list"));
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Int)), 1, "x").contains("read_integer_list"));
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Str)), 1, "x").contains("read_string_list"));
        // ref arm unchanged.
        assert!(
            bind_expr(&Kind::Vec(Box::new(Kind::Ref)), 1, "x").contains("read_entity_ref_list")
        );
        let sr = serialize_expr(&Kind::Vec(Box::new(Kind::Ref)), "items");
        assert!(sr.contains("|&s| crate::parser::entity::Attribute::EntityRef(s)"));
        let ss = serialize_expr(&Kind::Vec(Box::new(Kind::Str)), "labels");
        assert!(ss.contains("|s| crate::parser::entity::Attribute::String(s.clone())"));
        assert!(serialize_expr(&Kind::Vec(Box::new(Kind::Real)), "v").contains("Real(x)"));
    }

    /// Aggregations of enum / nested are deferred to v4.5 (panic).
    #[test]
    #[should_panic(expected = "not yet supported")]
    fn vec_enum_bind_panics() {
        bind_expr(&Kind::Vec(Box::new(Kind::Enum("foo".into()))), 1, "x");
    }

    #[test]
    #[should_panic(expected = "not yet supported")]
    fn vec_nested_bind_panics() {
        bind_expr(
            &Kind::Vec(Box::new(Kind::Vec(Box::new(Kind::Real)))),
            1,
            "x",
        );
    }

    /// STEP `logical` (.T./.F./.U.) -> `crate::ir::geometry::Logical`.
    #[test]
    fn logical_codegen() {
        let ctx = empty_ctx();
        assert!(matches!(ctx.classify("logical", 0), Kind::Logical));
        assert_eq!(
            field_ty(&ctx, &Kind::Logical),
            "crate::ir::geometry::Logical"
        );
        assert!(bind_expr(&Kind::Logical, 1, "x").contains("read_logical"));
        assert!(serialize_expr(&Kind::Logical, "x").contains("logical_to_step"));
    }
}
