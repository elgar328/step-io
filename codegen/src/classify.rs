//! Attribute-type classification — DERIVED from `gen-early/src/classify.rs`
//! (frozen), trimmed to the subset the schema-faithful generator needs and
//! decoupled from the gen-early mapping hint types.
//!
//! The crux difference vs gen-early: an all-entity SELECT or a bare entity ref
//! is NOT collapsed to an opaque `u64` here. Instead it is classified as
//! `Ref(target)` carrying the EXPRESS target type name, so the generator can
//! build a discriminated ref enum over the concrete leaf entities.

use std::collections::BTreeMap;

use crate::schema::{EarlyToml, agg_inner, strip_optional};

/// How an attribute's resolved `ty` maps to a generated Rust field.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Entity ref (incl. all-entity SELECT). Carries the EXPRESS target type
    /// name (entity or SELECT alias) so a discriminated ref enum can be built.
    Ref(String),
    /// Aggregation -> Vec<inner>.
    Vec(Box<Kind>),
    Real,
    Int,
    Bool,
    Logical,
    Str,
    Binary,
    /// ENUM type-alias name -> generated Rust enum.
    Enum(String),
    /// A select-of-named-scalars (all members numeric) -> `MeasureValue`
    /// (`{type_name: Option<String>, value: f64}`). Carries the alias name.
    MeasureSelect(String),
    /// A select-of-named-strings (all members string, e.g. SELECT(identifier,
    /// message)) -> `StringSelectValue` (`{type_name: Option<String>, value:
    /// String}`). Carries the alias name. Named members are Typed-wrapped on the
    /// wire (`IDENTIFIER('x')`).
    StringSelect(String),
    /// A select-of-named-integers (all members integer, e.g. SELECT(u_direction
    /// _count, v_direction_count)) -> `IntMeasureValue` (`{type_name: Option<
    /// String>, value: i64}`). Like MeasureSelect but integer-valued, so counts
    /// round-trip as Part-21 integers rather than being widened to reals.
    IntSelect(String),
}

pub const MAX_DEPTH: u32 = 64;

pub struct Resolver<'a> {
    pub schema: &'a EarlyToml,
    /// codegen support-boundary `narrow` map (`coverage.toml [narrow]`):
    /// `SELECT alias -> kept members`. Empty map = no-op.
    pub narrow: &'a BTreeMap<String, Vec<String>>,
}

impl<'a> Resolver<'a> {
    /// Members of the SELECT aliased by `alias`, with the `narrow` filter
    /// applied. This is the single choke-point every SELECT-member consumer
    /// (closure expansion, arm emission, Kind classification) must go through so
    /// the closure and the emitted arms stay lockstep. Returns `None` when
    /// `alias` is not a `SELECT(...)` type.
    pub fn select_members(&self, alias: &str) -> Option<Vec<&'a str>> {
        let td = self.schema.types.get(alias)?;
        let members = crate::schema::select_members(td.aliased.trim())?;
        Some(match self.narrow.get(alias) {
            Some(keep) => members
                .into_iter()
                .filter(|m| keep.iter().any(|k| k == m))
                .collect(),
            None => members,
        })
    }

    /// True when `ty` resolves to an entity reference (encodes as a bare `#N`).
    pub fn ref_like(&self, ty: &str, depth: u32) -> bool {
        assert!(depth < MAX_DEPTH, "ref_like: ty `{ty}` too deep (cycle?)");
        let t = ty.trim();
        if self.schema.entity.contains_key(t) {
            return true;
        }
        let Some(td) = self.schema.types.get(t) else {
            return false;
        };
        let a = td.aliased.trim();
        if let Some(members) = self.select_members(t) {
            return members.iter().all(|m| self.ref_like(m, depth + 1));
        }
        if a.starts_with("ENUM(") {
            return false;
        }
        self.ref_like(a, depth + 1)
    }

    /// If `m` (a member ty token or alias) resolves to an aggregate
    /// (`LIST/SET/BAG/ARRAY OF E`), return its element ty `E`. Resolves one alias
    /// hop (e.g. `list_representation_item` -> `LIST OF representation_item`).
    pub fn agg_element(&self, m: &str) -> Option<String> {
        let t = m.trim();
        if let Some(inner) = agg_inner(t) {
            return Some(inner.to_string());
        }
        let aliased = self.schema.types.get(t)?.aliased.trim().to_string();
        agg_inner(&aliased).map(str::to_string)
    }

    /// True when a SELECT member is an aggregate of a ref-like element — the
    /// aggregate-arm shape (`SELECT(.., LIST OF X, SET OF X)`).
    pub fn agg_of_ref(&self, m: &str) -> bool {
        self.agg_element(m).is_some_and(|e| self.ref_like(&e, 0))
    }

    /// True when a SELECT member resolves to a named scalar (real/int/string/
    /// bool) — the scalar-arm shape (`SELECT(entity, parameter_value)`). Binary
    /// is excluded (only appears in the deferred maths cluster).
    pub fn is_scalar_member(&self, m: &str) -> bool {
        matches!(
            self.classify(m, 0),
            Kind::Real | Kind::Int | Kind::Str | Kind::Bool
        )
    }

    /// True when every member of a SELECT resolves to a string — the
    /// select-of-named-strings shape (`StringSelectValue`). Checked before
    /// `is_measure_select` (which also accepts `Str`) so all-string selects do
    /// not get mis-modeled as a numeric `MeasureValue`.
    pub fn is_string_select(&self, members: &[&str], depth: u32) -> bool {
        if depth >= MAX_DEPTH {
            return false;
        }
        members
            .iter()
            .all(|m| matches!(self.classify(m, depth + 1), Kind::Str))
    }

    /// True when every member of a SELECT resolves to a scalar primitive (no
    /// entity ref, no nested mixed SELECT) — the select-of-scalars / measure
    /// shape modeled generically as `MeasureValue { type_name, value }`.
    pub fn is_measure_select(&self, members: &[&str], depth: u32) -> bool {
        if depth >= MAX_DEPTH {
            return false;
        }
        members.iter().all(|m| {
            matches!(
                self.classify(m, depth + 1),
                Kind::Real | Kind::Int | Kind::Str | Kind::Bool
            )
        })
    }

    /// True when every member of a SELECT resolves to `Kind::Int` — an
    /// integer-only select-of-scalars, modeled as `IntMeasureValue` so the
    /// integers are preserved (vs the real-valued MeasureSelect which widens to
    /// f64). Must be checked before `is_measure_select`, which also accepts Int.
    pub fn is_int_select(&self, members: &[&str], depth: u32) -> bool {
        if depth >= MAX_DEPTH {
            return false;
        }
        members
            .iter()
            .all(|m| matches!(self.classify(m, depth + 1), Kind::Int))
    }

    /// Resolve a `ty` token to a [`Kind`]. `OPTIONAL` is stripped by the caller
    /// (tracked as a separate flag); this strips defensively too.
    pub fn classify(&self, ty: &str, depth: u32) -> Kind {
        assert!(depth < MAX_DEPTH, "classify: ty `{ty}` too deep (cycle?)");
        let t = ty.trim();
        if let Some(inner) = t.strip_prefix("OPTIONAL ") {
            return self.classify(inner.trim(), depth + 1);
        }
        if let Some(inner) = agg_inner(t) {
            return Kind::Vec(Box::new(self.classify(inner, depth + 1)));
        }
        match t {
            "real" | "number" => return Kind::Real,
            "integer" => return Kind::Int,
            "boolean" => return Kind::Bool,
            "logical" => return Kind::Logical,
            "string" => return Kind::Str,
            "binary" => return Kind::Binary,
            _ => {}
        }
        if self.schema.entity.contains_key(t) {
            return Kind::Ref(t.to_string());
        }
        if let Some(td) = self.schema.types.get(t) {
            let a = td.aliased.trim();
            if a.starts_with("ENUM(") {
                return Kind::Enum(t.to_string());
            }
            if let Some(members) = self.select_members(t) {
                // All-string select -> StringSelectValue (checked first, since the
                // numeric measure check below also accepts Str).
                if self.is_string_select(&members, depth + 1) {
                    return Kind::StringSelect(t.to_string());
                }
                // All-integer select -> IntMeasureValue (preserve integers).
                // Must precede the measure check, which also accepts Int and
                // would widen to f64.
                if self.is_int_select(&members, depth + 1) {
                    return Kind::IntSelect(t.to_string());
                }
                // All-scalar (numeric) select -> MeasureValue (select-of-scalars).
                if self.is_measure_select(&members, depth + 1) {
                    return Kind::MeasureSelect(t.to_string());
                }
                // Discriminated ref over the SELECT alias when every member is an
                // entity ref, an aggregate-of-entity (SELECT(X, LIST OF X, ..)),
                // an ENUM (SELECT(X, some_enum)), or a named scalar (SELECT(X,
                // parameter_value)). model_ir splits single/aggregate/enum/scalar
                // arms. Reached only after the all-string/all-numeric checks, so a
                // scalar member here means an entity is mixed in.
                if members.iter().all(|m| {
                    self.ref_like(m, 0)
                        || self.agg_of_ref(m)
                        || self.enum_alias(m, 0).is_some()
                        || self.is_scalar_member(m)
                }) {
                    return Kind::Ref(t.to_string());
                }
                // Other mixed selects (entity+scalar, all-string) are separate
                // capabilities, not yet built.
                panic!("classify: mixed SELECT `{t}` unsupported in Phase 0 closure");
            }
            return self.classify(a, depth + 1);
        }
        panic!("classify: cannot resolve ty token `{t}`");
    }

    /// Resolve a `ty` to its underlying ENUM alias name, if it is one.
    pub fn enum_alias(&self, ty: &str, depth: u32) -> Option<String> {
        if depth >= MAX_DEPTH {
            return None;
        }
        let t = ty.trim();
        let td = self.schema.types.get(t)?;
        let a = td.aliased.trim();
        if a.starts_with("ENUM(") {
            return Some(t.to_string());
        }
        if self.select_members(t).is_some() {
            return None;
        }
        self.enum_alias(a, depth + 1)
    }

    /// Strip OPTIONAL and classify in one go, returning `(optional, kind)`.
    pub fn classify_attr(&self, ty: &str) -> (bool, Kind) {
        let (opt, inner) = strip_optional(ty);
        (opt, self.classify(inner, 0))
    }
}

/// snake_case / UPPER_SNAKE -> PascalCase.
pub fn pascal(snake: &str) -> String {
    snake
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
                .unwrap_or_default()
        })
        .collect()
}
