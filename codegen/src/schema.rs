//! Deserialization model for `schema/universal.toml` — the EXPRESS schema union
//! (entities, attributes, SELECTs, ENUMs) the generator reads.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Schema {
    #[serde(default)]
    pub entity: BTreeMap<String, Entity>,
    #[serde(rename = "type", default)]
    pub types: BTreeMap<String, TypeDef>,
}

#[derive(Deserialize, Clone)]
pub struct Entity {
    /// Direct supertype names (EXPRESS declaration order, left-to-right).
    #[serde(default)]
    pub parents: Vec<String>,
    /// EXPRESS `ABSTRACT` flag. NOTE: known to be unreliable in the schema data
    /// (curve/surface/representation_item are EXPRESS-abstract but emit
    /// `is_abstract=false`); the generator does NOT use it for arena/ref
    /// classification — it is read only so deserialize does not choke and as a
    /// secondary diagnostic.
    #[serde(default)]
    #[allow(dead_code)] // intentionally not used for classification (see note above)
    pub is_abstract: bool,
    /// True if this entity can appear as a Part21 complex (multiple-inheritance)
    /// part (ANDOR-combinable leaf or a supertype of one) — auto-derived in
    /// `universal_export`. Drives the part-bag (`complex_parts`), replacing the
    /// former hand-written `[codegen].complex_parts`.
    #[serde(default)]
    pub is_complex_part: bool,
    #[serde(default)]
    pub own_attrs: Vec<Attr>,
    /// `SELF\super.attr : type` scalar-primitive narrowings, applied in-place.
    #[serde(default)]
    pub redeclared_attrs: Vec<Attr>,
    /// Raw EXPRESS DERIVE targets (universal.toml): each `"super.attr"` (a
    /// `SELF\super.attr` redeclaration) or `"attr"` (own-attr derive). Marks
    /// `*` slots; the hard/derivable split is computed in `main.rs`.
    #[serde(default)]
    pub derives: Vec<String>,
}

impl Schema {
    /// Full attribute list in Part21 positional order: inherited attrs first
    /// (each parent chain deepest-ancestor-first, parents left-to-right), then
    /// the entity's own attrs. Scalar redeclares applied last as in-place
    /// type overrides.
    pub fn flattened_attrs(&self, ent_name: &str) -> Vec<Attr> {
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
    /// complex-entity emitter (each part contributes its own attrs).
    pub fn own_attrs(&self, ent_name: &str) -> Vec<Attr> {
        self.entity
            .get(ent_name)
            .map(|e| e.own_attrs.clone())
            .unwrap_or_default()
    }

    pub fn collect_inherited<'a>(
        &'a self,
        ent_name: &str,
        visited: &mut BTreeSet<String>,
        out: &mut Vec<Attr>,
        redeclared: &mut Vec<&'a Attr>,
    ) {
        let Some(ent) = self.entity.get(ent_name) else {
            return; // dangling parent reference — skip
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
pub struct Attr {
    pub name: String,
    pub ty: String,
}

#[derive(Deserialize)]
pub struct TypeDef {
    pub aliased: String,
}

/// Inner element of a `LIST/SET/BAG/ARRAY OF X` aggregation, or `None`.
pub fn agg_inner(ty: &str) -> Option<&str> {
    ["LIST OF ", "SET OF ", "BAG OF ", "ARRAY OF "]
        .iter()
        .find_map(|p| ty.strip_prefix(p))
        .map(str::trim)
}

/// Members of an inline `SELECT(a, b, …)` aliased type, or `None`. Raw lexical
/// parse — does NOT apply the `coverage.toml [narrow]` filter. Prefer
/// [`crate::classify::Resolver::select_members`] in the generator so closure
/// expansion and arm emission stay narrow-consistent; call this directly only
/// where narrowing must not apply.
pub fn select_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("SELECT(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

/// Members of an inline `ENUM(a, b, …)` aliased type, or `None`.
pub fn enum_members(aliased: &str) -> Option<Vec<&str>> {
    aliased
        .strip_prefix("ENUM(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|inner| inner.split(',').map(str::trim).collect())
}

/// Split a leading `OPTIONAL ` off a `ty`, returning `(is_optional, inner_ty)`.
pub fn strip_optional(ty: &str) -> (bool, &str) {
    match ty.trim().strip_prefix("OPTIONAL ") {
        Some(inner) => (true, inner.trim()),
        None => (false, ty.trim()),
    }
}
