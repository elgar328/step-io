//! Attribute-type classification — DERIVED from `gen-early/src/classify.rs`
//! (frozen), trimmed to the subset the schema-faithful generator needs and
//! decoupled from the gen-early mapping hint types.
//!
//! The crux difference vs gen-early: an all-entity SELECT or a bare entity ref
//! is NOT collapsed to an opaque `u64` here. Instead it is classified as
//! `Ref(target)` carrying the EXPRESS target type name, so the generator can
//! build a discriminated ref enum over the concrete leaf entities.

use crate::schema::{EarlyToml, agg_inner, select_members, strip_optional};

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
}

pub const MAX_DEPTH: u32 = 64;

pub struct Resolver<'a> {
    pub schema: &'a EarlyToml,
}

impl<'a> Resolver<'a> {
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
        if let Some(members) = select_members(a) {
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
            if let Some(members) = select_members(a) {
                // All-scalar select -> MeasureValue (select-of-scalars).
                if self.is_measure_select(&members, depth + 1) {
                    return Kind::MeasureSelect(t.to_string());
                }
                // All-entity select, OR entity + aggregate-of-entity members
                // (SELECT(X, LIST OF X, SET OF X)) -> discriminated ref over the
                // SELECT alias. model_ir splits single arms vs aggregate arms.
                if members
                    .iter()
                    .all(|m| self.ref_like(m, 0) || self.agg_of_ref(m))
                {
                    return Kind::Ref(t.to_string());
                }
                // Other mixed selects (entity+enum, entity+scalar, all-string)
                // are separate capabilities, not yet built.
                panic!("classify: mixed SELECT `{t}` unsupported in Phase 0 closure");
            }
            return self.classify(a, depth + 1);
        }
        panic!("classify: cannot resolve ty token `{t}`");
    }

    /// Resolve a `ty` to its underlying ENUM alias name, if it is one.
    #[allow(dead_code)] // helper carried from gen-early; not needed yet
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
        if select_members(a).is_some() {
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
