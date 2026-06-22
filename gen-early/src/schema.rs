//! `schema/early.toml` model (the L1 schema union) + the small EXPRESS-
//! syntax parsers shared by classification and emission.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct EarlyToml {
    #[serde(default)]
    pub(crate) entity: BTreeMap<String, Entity>,
    #[serde(rename = "type", default)]
    pub(crate) types: BTreeMap<String, TypeDef>,
}

#[derive(Deserialize)]
pub(crate) struct Entity {
    /// Direct supertype names (EXPRESS declaration order, left-to-right).
    #[serde(default)]
    pub(crate) parents: Vec<String>,
    #[serde(default)]
    pub(crate) own_attrs: Vec<Attr>,
    /// `SELF\super.attr : type` scalar-primitive narrowings (blueprint emits
    /// only these). Applied as in-place type overrides on the flattened attr
    /// list — they retype an inherited attr, never add a new positional slot.
    #[serde(default)]
    pub(crate) redeclared_attrs: Vec<Attr>,
}

impl EarlyToml {
    /// Full attribute list in Part21 positional order: inherited attrs first
    /// (each parent chain deepest-ancestor-first, parents left-to-right), then
    /// the entity's own attrs. Mirrors the blueprint's `collect_ancestor_attrs`.
    ///
    /// A shared ancestor reached via multiple parent paths (diamond) is included
    /// once — deduped by entity name via `visited`. Same-named attrs declared by
    /// *different* parents both survive (dedup is per-entity, not per-attr).
    ///
    /// Scalar `redeclared_attrs` (SELF\super.attr narrowings) from the entity and
    /// its ancestors are applied last as in-place type overrides on the matching
    /// inherited attr — positional order is preserved (a redeclare retypes, never
    /// appends). Returns owned `Attr`s so the override can mutate `ty`.
    pub(crate) fn flattened_attrs(&self, ent_name: &str) -> Vec<Attr> {
        let mut out: Vec<Attr> = Vec::new();
        let mut redeclared: Vec<&Attr> = Vec::new();
        let mut visited = BTreeSet::new();
        self.collect_inherited(ent_name, &mut visited, &mut out, &mut redeclared);
        if let Some(ent) = self.entity.get(ent_name) {
            out.extend(ent.own_attrs.iter().cloned());
            redeclared.extend(ent.redeclared_attrs.iter());
        }
        for r in redeclared {
            if let Some(a) = out.iter_mut().find(|a| a.name == r.name) {
                a.ty.clone_from(&r.ty);
            }
        }
        out
    }

    /// An entity's OWN attributes only (no inheritance flattening). Used by the
    /// complex-entity emitter: each part of a complex instance contributes its
    /// own attrs (the supertype parts carry no inherited attrs on the wire).
    pub(crate) fn own_attrs(&self, ent_name: &str) -> Vec<&Attr> {
        self.entity
            .get(ent_name)
            .map(|e| e.own_attrs.iter().collect())
            .unwrap_or_default()
    }

    pub(crate) fn collect_inherited<'a>(
        &'a self,
        ent_name: &str,
        visited: &mut BTreeSet<String>,
        out: &mut Vec<Attr>,
        redeclared: &mut Vec<&'a Attr>,
    ) {
        let Some(ent) = self.entity.get(ent_name) else {
            return; // dangling parent reference — skip (matches blueprint)
        };
        for parent in &ent.parents {
            if !visited.insert(parent.clone()) {
                continue; // diamond: shared ancestor already collected
            }
            self.collect_inherited(parent, visited, out, redeclared);
            if let Some(p) = self.entity.get(parent) {
                out.extend(p.own_attrs.iter().cloned());
                redeclared.extend(p.redeclared_attrs.iter());
            }
        }
    }
}

#[derive(Deserialize, Clone)]
pub(crate) struct Attr {
    pub(crate) name: String,
    pub(crate) ty: String,
}

#[derive(Deserialize)]
pub(crate) struct TypeDef {
    pub(crate) aliased: String,
}

/// Inner element of a `LIST/SET/BAG/ARRAY OF X` aggregation, or `None`.
pub(crate) fn agg_inner(ty: &str) -> Option<&str> {
    ["LIST OF ", "SET OF ", "BAG OF ", "ARRAY OF "]
        .iter()
        .find_map(|p| ty.strip_prefix(p))
        .map(str::trim)
}

/// Members of an inline `SELECT(a, b, …)` aliased type, or `None`.
pub(crate) fn select_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("SELECT(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

/// Members of an inline `ENUM(a, b, …)` aliased type, or `None`.
pub(crate) fn enum_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("ENUM(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

/// Split a leading `OPTIONAL ` off a `ty`, returning `(is_optional, inner_ty)`.
/// EXPRESS does not nest `OPTIONAL`, so a single strip suffices.
pub(crate) fn strip_optional(ty: &str) -> (bool, &str) {
    match ty.trim().strip_prefix("OPTIONAL ") {
        Some(inner) => (true, inner.trim()),
        None => (false, ty.trim()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;

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

    /// Inherited `name` (from `representation_item`) precedes the entity's own.
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

    /// Three-level chain: `representation_item` -> `geometric_representation_item`
    /// -> `text_literal`.
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
            parents: parents
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            own_attrs: own.iter().map(|n| attr(n)).collect(),
            redeclared_attrs: Vec::new(),
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

    /// A child's scalar `redeclared_attrs` retypes the inherited attr in place
    /// (the `int_literal`-narrows-`the_value`-to-INTEGER case) — same name, same
    /// position, narrowed type.
    #[test]
    fn flatten_applies_scalar_redeclare() {
        let mut entity = BTreeMap::new();
        entity.insert(
            "lit".to_string(),
            Entity {
                parents: vec![],
                own_attrs: vec![Attr {
                    name: "the_value".to_string(),
                    ty: "number".to_string(),
                }],
                redeclared_attrs: vec![],
            },
        );
        entity.insert(
            "int_lit".to_string(),
            Entity {
                parents: vec!["lit".to_string()],
                own_attrs: vec![],
                redeclared_attrs: vec![Attr {
                    name: "the_value".to_string(),
                    ty: "integer".to_string(),
                }],
            },
        );
        let s = EarlyToml {
            entity,
            types: BTreeMap::new(),
        };
        let flat = s.flattened_attrs("int_lit");
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].name, "the_value");
        assert_eq!(flat[0].ty, "integer");
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
}
