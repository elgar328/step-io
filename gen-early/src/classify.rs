//! Attribute-type classification ([`Kind`]) and the support predicates
//! ([`Ctx`]) that mirror the emit-time panics (the v4.6 backstop design).

use std::collections::BTreeSet;

use crate::mapping::{EnumHint, Mapping, SelectHint};
use crate::schema::{EarlyToml, agg_inner, select_members, strip_optional};

/// How an attribute's resolved `ty` maps to Rust / bind / serialize.
#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Ref,            // entity ref (incl. all-entity SELECT) -> u64
    Vec(Box<Kind>), // aggregation -> Vec<inner> (LIST/SET/BAG/ARRAY OF inner)
    Real,           // -> f64
    Int,            // -> i64
    Bool,           // -> bool
    Logical,        // -> crate::ir::geometry::Logical (.T./.F./.U.)
    Str,            // -> String
    Binary,         // -> String (hex; Attribute::Binary)
    Enum(String),   // ENUM type-alias name (hinted -> L2 reuse, else synth Early*)
    Select(String), // mixed SELECT type-alias name (looked up in mapping.selects)
}

pub(crate) const MAX_DEPTH: u32 = 32;

pub(crate) struct Ctx {
    pub(crate) schema: EarlyToml,
    pub(crate) mapping: Mapping,
}

impl Ctx {
    /// True when `ty` is (or resolves through aliases / all-entity SELECTs to)
    /// an entity reference — i.e. encodes in Part21 as a bare `#N`.
    pub(crate) fn ref_like(&self, ty: &str, depth: u32) -> bool {
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
    pub(crate) fn classify(&self, ty: &str, depth: u32) -> Kind {
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
            "binary" => return Kind::Binary,
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
    pub(crate) fn try_classify(&self, ty: &str, depth: u32) -> Option<Kind> {
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
            "binary" => return Some(Kind::Binary),
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
    pub(crate) fn emittable(&self, k: &Kind, optional: bool) -> bool {
        match k {
            // read_optional_* exist for ref/scalar (optional is fine too); an
            // optional enum wraps the standalone enum decode (v4.10).
            Kind::Ref | Kind::Real | Kind::Int | Kind::Str | Kind::Enum(_) => true,
            // scalar bind exists; optional deferred (no read_optional_*).
            Kind::Bool | Kind::Logical | Kind::Binary => !optional,
            // ref / scalar / enum / supported-select element (single list), or a
            // `Vec<Vec<ref/real/int>>` grid. Single-level aggregations may be
            // `OPTIONAL` (`Option<Vec<_>>` via the v4.16 presence-check wrapper);
            // the grid is still non-optional only (no opt-grid occurrence).
            Kind::Vec(inner) => match &**inner {
                Kind::Ref | Kind::Real | Kind::Int | Kind::Str | Kind::Enum(_) | Kind::Logical => {
                    true
                }
                Kind::Select(s) => self.select_supported(s),
                Kind::Vec(i2) => {
                    !optional
                        && match &**i2 {
                            // 2D grid (v4.9).
                            Kind::Ref | Kind::Real | Kind::Int => true,
                            // 2D grid of a mixed select (v4.21): `<alias>_grid`.
                            Kind::Select(s) => self.select_supported(s),
                            // 3D grid (v4.17): ref / real only (no int occurrence).
                            Kind::Vec(i3) => matches!(**i3, Kind::Ref | Kind::Real),
                            _ => false,
                        }
                }
                _ => false,
            },
            // optional select is handled by the v4.2 let-form; both need a
            // supported select.
            Kind::Select(sel) => self.select_supported(sel),
        }
    }

    /// Whether a mixed SELECT is emittable — hinted ones always; hint-less ones
    /// only in the shape [`emit_select_synth`] accepts.
    pub(crate) fn select_supported(&self, sel: &str) -> bool {
        self.select_supported_inner(sel, &mut BTreeSet::new())
    }

    /// [`select_supported`] with a self-recursion guard: a select reached again
    /// while already being checked (e.g. `maths_value` → `maths_tuple` = `LIST OF
    /// maths_value`) is optimistically fine — any genuinely unsupported member
    /// fails its own check.
    pub(crate) fn select_supported_inner(&self, sel: &str, seen: &mut BTreeSet<String>) -> bool {
        if self.mapping.selects.contains_key(sel) {
            return true; // hinted -> emit_select
        }
        if !seen.insert(sel.to_string()) {
            return true; // self-recursive reference
        }
        let Some(td) = self.schema.types.get(sel) else {
            return false;
        };
        let Some(members) = select_members(td.aliased.trim()) else {
            return false;
        };
        for m in members {
            let Some(k) = self.try_classify(m, 0) else {
                return false; // unresolved member
            };
            match k {
                // hinted enum member -> dual representation, deferred.
                Kind::Enum(a) if self.mapping.enums.contains_key(&a) => return false,
                // Entity members (any number — they collapse into one combined
                // `EntityRef(u64)` variant, `lower` resolves the type), scalar
                // members incl. logical / binary, and non-hinted enums (each a
                // single `Typed{tag, ..}`, distinguishable by tag) are fine.
                Kind::Ref
                | Kind::Real
                | Kind::Int
                | Kind::Str
                | Kind::Bool
                | Kind::Logical
                | Kind::Binary
                | Kind::Enum(_) => {}
                // Aggregation member: per Part21 it arrives as a *typed parameter*
                // `TAG((...))` (e.g. `COMMON_DATUM_LIST((#1,#2))`, see the
                // hand-written reader in src/entities/pmi.rs), so the tag
                // disambiguates it — any number / any mix of inner kinds is fine.
                Kind::Vec(inner)
                    if matches!(*inner, Kind::Ref | Kind::Real | Kind::Int | Kind::Str) => {}
                // Aggregation of a mixed select (e.g. `maths_tuple` = `LIST OF
                // maths_value`): elements decode via `bind_<inner>` if supported.
                Kind::Vec(inner) => match &*inner {
                    Kind::Select(s) => {
                        if !self.select_supported_inner(s, seen) {
                            return false;
                        }
                    }
                    _ => return false,
                },
                // Nested mixed SELECT member: all its values must arrive as
                // `Typed` (so the outer bind can delegate the whole attr to
                // `bind_<nested>`) and the nested select itself must be supported.
                Kind::Select(nested) => {
                    if !self.select_typed_only(&nested)
                        || !self.select_supported_inner(&nested, seen)
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Whether *every value* of a mixed SELECT arrives as `Attribute::Typed{tag,
    /// ..}` — scalars (incl. logical / binary), non-hinted enums, aggregations
    /// (typed parameters per Part21, v4.20), and recursively Typed-only nested
    /// selects. No bare `EntityRef` member (a `#N` would be ambiguous with the
    /// outer select's refs). Such a select can be a *nested member* of another
    /// synth select, decoded by delegating the whole `attr` to its `bind_<sel>`
    /// from the outer Typed `_ =>` arm.
    pub(crate) fn select_typed_only(&self, sel: &str) -> bool {
        self.select_typed_only_inner(sel, &mut BTreeSet::new())
    }

    pub(crate) fn select_typed_only_inner(&self, sel: &str, seen: &mut BTreeSet<String>) -> bool {
        if !seen.insert(sel.to_string()) {
            return true; // self-recursive reference
        }
        let Some(td) = self.schema.types.get(sel) else {
            return false;
        };
        let Some(members) = select_members(td.aliased.trim()) else {
            return false;
        };
        members.iter().all(|m| match self.try_classify(m, 0) {
            // Scalars (incl. logical / binary) and aggregation members all
            // arrive as typed parameters `TAG(..)` — for aggregations the
            // decodability is select_supported's concern, not the form.
            Some(
                Kind::Real
                | Kind::Int
                | Kind::Str
                | Kind::Bool
                | Kind::Logical
                | Kind::Binary
                | Kind::Vec(_),
            ) => true,
            // non-hinted enum members serialize as `Typed{tag, Enum}` too.
            Some(Kind::Enum(a)) => !self.mapping.enums.contains_key(&a),
            // a nested select is Typed-form iff it is itself Typed-only.
            Some(Kind::Select(s)) => self.select_typed_only_inner(&s, seen),
            _ => false,
        })
    }

    /// `Ok` if every flattened attribute of `ent` is emittable, else the first
    /// failure's reason tag (for the coverage report).
    pub(crate) fn entity_supported(&self, ent: &str) -> Result<(), String> {
        // Duplicate flattened attr names (multiple inheritance) are handled by
        // disambiguating the Rust field identifier in the emit loop, so they no
        // longer block an entity — only unsupported attr *shapes* do.
        for a in self.schema.flattened_attrs(ent) {
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

    pub(crate) fn enum_hint(&self, alias: &str) -> &EnumHint {
        self.mapping
            .enums
            .get(alias)
            .unwrap_or_else(|| panic!("gen-early: no [enum.{alias}] hint in mapping.toml"))
    }

    pub(crate) fn select_hint(&self, alias: &str) -> &SelectHint {
        self.mapping
            .selects
            .get(alias)
            .unwrap_or_else(|| panic!("gen-early: no [select.{alias}] hint in mapping.toml"))
    }
}

pub(crate) fn pascal(snake: &str) -> String {
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
pub(crate) fn kind_tag(k: &Kind) -> &'static str {
    match k {
        Kind::Ref => "ref",
        Kind::Real => "real",
        Kind::Int => "int",
        Kind::Bool => "bool",
        Kind::Logical => "logical",
        Kind::Str => "str",
        Kind::Binary => "binary",
        Kind::Enum(_) => "enum",
        Kind::Select(_) => "select",
        Kind::Vec(_) => "vec",
    }
}

/// Categorize why an unsupported (kind, optional) attribute can't be emitted.
pub(crate) fn unsupported_reason(k: &Kind, optional: bool) -> String {
    match k {
        Kind::Vec(inner) if matches!(**inner, Kind::Vec(_)) => "nested-agg".to_string(),
        Kind::Vec(inner) => format!("vec-{}", kind_tag(inner)),
        Kind::Select(_) => "select-edge".to_string(),
        _ if optional => format!("opt-{}", kind_tag(k)),
        _ => kind_tag(k).to_string(),
    }
}
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::schema::{TypeDef, enum_members};
    use crate::testutil::*;

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

    /// `select_typed_only`: true for an all-scalar select, false once a member is
    /// an entity ref (bare `#N`); aggregations / nested selects are Typed-form.
    #[test]
    fn select_typed_only_classification() {
        let ctx = ctx_from(schema());
        assert!(ctx.select_typed_only("measure_value")); // 42 scalar measures
        assert!(!ctx.select_typed_only("trim_condition_select")); // has entity refs
        assert!(!ctx.select_typed_only("maths_value")); // generic_expression = ref
        // logical / binary members are still Typed-only (single `Typed` value).
        assert!(ctx.select_typed_only("maths_simple_atom"));
        // v4.21: agg members are Typed (`TAG((...))`), nested recursion guarded —
        // atom_based_value (self-recursive tuple + nested maths_atom) qualifies.
        assert!(ctx.select_typed_only("atom_based_value"));
    }
}
