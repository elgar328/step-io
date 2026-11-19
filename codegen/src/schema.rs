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
    /// `SELF\super.attr : type` redeclarations (scalar retypes, SELECT and
    /// reference narrowings), applied in place over the attribute `owner`
    /// declares.
    #[serde(default)]
    pub redeclared_attrs: Vec<Attr>,
    /// Raw EXPRESS DERIVE targets (universal.toml): each `"super.attr"` (a
    /// `SELF\super.attr` redeclaration) or `"attr"` (own-attr derive). Marks
    /// `*` slots; the hard/derivable split is computed in `main.rs`.
    #[serde(default)]
    pub derives: Vec<String>,
    /// Output profile only: attribute type restrictions the entity's WHERE
    /// rules make, holding for every subtype too.
    #[serde(default)]
    pub type_rules: Vec<TypeRule>,
    /// Source AP schemas declaring this entity, publication order (ap203e1 →
    /// ap242e3). Membership data for the coverage matrix doc; not used for
    /// generation.
    #[serde(default)]
    pub schemas: Vec<String>,
}

impl Schema {
    /// Full attribute list in Part21 positional order: inherited attrs first
    /// (each parent chain deepest-ancestor-first, parents left-to-right), then
    /// the entity's own attrs. Redeclarations are applied last as in-place
    /// type overrides, each on the attribute its `owner` declares; a nearer
    /// one (applied later) narrows a farther one.
    pub fn flattened_attrs(&self, ent_name: &str) -> Vec<Attr> {
        let mut out: Vec<(&str, Attr)> = Vec::new();
        let mut redeclared: Vec<(&str, &Attr)> = Vec::new();
        let mut visited = BTreeSet::new();
        self.collect_inherited(ent_name, &mut visited, &mut out, &mut redeclared);
        if let Some((name, ent)) = self.entity.get_key_value(ent_name) {
            out.extend(ent.own_attrs.iter().map(|a| (name.as_str(), a.clone())));
            redeclared.extend(ent.redeclared_attrs.iter().map(|r| (name.as_str(), r)));
        }
        let mut by: Vec<Option<&str>> = vec![None; out.len()];
        for (redeclarer, r) in redeclared {
            let slot = out.iter().position(|(origin, a)| {
                a.name == r.name && r.owner.as_deref().is_none_or(|o| o == *origin)
            });
            if let Some(i) = slot {
                if let Some(prev) = by[i] {
                    assert!(
                        out[i].1.ty == r.ty || self.is_ancestor(prev, redeclarer),
                        "{ent_name}.{}: unrelated redeclarations by {prev} and {redeclarer}",
                        r.name
                    );
                }
                out[i].1.ty.clone_from(&r.ty);
                by[i] = Some(redeclarer);
            }
        }
        out.into_iter().map(|(_, a)| a).collect()
    }

    /// Whether `ancestor` is a (transitive) supertype of `entity`.
    fn is_ancestor(&self, ancestor: &str, entity: &str) -> bool {
        let mut stack = vec![entity];
        let mut seen = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            if let Some(e) = self.entity.get(n) {
                for p in &e.parents {
                    if p == ancestor {
                        return true;
                    }
                    stack.push(p);
                }
            }
        }
        false
    }

    /// The WHERE-rule type restrictions holding for `ent_name`: its own and
    /// every supertype's, each with the entity declaring it, nearest first.
    pub fn type_rules_for(&self, ent_name: &str) -> Vec<(&str, &TypeRule)> {
        let mut out = Vec::new();
        let Some((first, _)) = self.entity.get_key_value(ent_name) else {
            return out;
        };
        let mut stack = vec![first.as_str()];
        let mut seen = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            if let Some((name, e)) = self.entity.get_key_value(n) {
                out.extend(e.type_rules.iter().map(|r| (name.as_str(), r)));
                stack.extend(e.parents.iter().map(String::as_str));
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

    /// Collect `ent_name`'s inherited attributes and redeclarations, each with
    /// the entity it comes from.
    pub fn collect_inherited<'a>(
        &'a self,
        ent_name: &str,
        visited: &mut BTreeSet<String>,
        out: &mut Vec<(&'a str, Attr)>,
        redeclared: &mut Vec<(&'a str, &'a Attr)>,
    ) {
        let Some(ent) = self.entity.get(ent_name) else {
            return; // dangling parent reference — skip
        };
        for parent in &ent.parents {
            if !visited.insert(parent.clone()) {
                continue; // diamond: shared ancestor already collected
            }
            self.collect_inherited(parent, visited, out, redeclared);
            if let Some((name, p)) = self.entity.get_key_value(parent) {
                out.extend(p.own_attrs.iter().map(|a| (name.as_str(), a.clone())));
                redeclared.extend(p.redeclared_attrs.iter().map(|r| (name.as_str(), r)));
            }
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct Attr {
    pub name: String,
    pub ty: String,
    /// For a redeclaration, the entity that declares the attribute.
    #[serde(default)]
    pub owner: Option<String>,
    /// For a redeclaration, whether every edition's type for the attribute
    /// falls inside it, so reading may enforce it on any instance.
    #[serde(default)]
    pub union_safe: Option<bool>,
}

/// A WHERE rule's restriction of an attribute's types (output profile).
#[derive(Deserialize, Clone)]
pub struct TypeRule {
    /// The restricted attribute.
    pub name: String,
    /// The entity declaring the attribute.
    pub owner: String,
    /// The single type the rule names, else `<entity>_<attr>`: the name of a
    /// reference type made for it.
    pub type_name: String,
    /// The entities (lowercase) the attribute, or each element of it, may
    /// reference.
    pub admits: BTreeSet<String>,
    /// How many of `names` the value's `TYPEOF` holds.
    pub count: RuleCount,
    /// The type names the rule lists (an entity, or a SELECT for its
    /// members); a complex value's `TYPEOF` is the union over its parts.
    pub names: Vec<String>,
}

/// How many of a type rule's names a value's `TYPEOF` holds.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RuleCount {
    AtLeastOne,
    ExactlyOne,
    Zero,
}

#[derive(Deserialize)]
pub struct TypeDef {
    pub aliased: String,
}

/// Deserialization model for a per-target output profile (`schema/ap*.toml`,
/// from blueprint `profile_export`). Only the fields the generators consume
/// are read: legal entity set (`entity` keys, author constructors), `oneof`
/// groups, and `meta.file_schema`. The rest of the export (`downgrade`,
/// `meta.apd`, entity attribute detail, `[type]`) is ignored.
#[derive(Deserialize)]
pub struct ProfileToml {
    pub meta: ProfileMeta,
    #[serde(default)]
    pub oneof: Vec<ProfileOneof>,
    #[serde(default)]
    pub entity: BTreeMap<String, toml::Value>,
}

/// One ONEOF exclusivity group from a `SUPERTYPE OF` clause: a complex
/// instance may draw parts from at most one branch. Branch = flat entity-name
/// set of one ONEOF child subtree (composite children like `loop ANDOR path`
/// collapse to their name set).
#[derive(Deserialize)]
pub struct ProfileOneof {
    /// Entity whose `SUPERTYPE OF` clause declares the group — provenance
    /// only (the exclusivity check needs just the branches).
    #[allow(dead_code)]
    pub supertype: String,
    pub branches: Vec<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ProfileMeta {
    pub file_schema: Vec<String>,
}

/// Inner element of a `LIST/SET/BAG/ARRAY [n:m] OF X` aggregation (bounds
/// optional in the repr), or `None`.
pub fn agg_inner(ty: &str) -> Option<&str> {
    agg_parts(ty).map(|(inner, _, _)| inner)
}

/// Cardinality bounds of an aggregation repr: `(lower, upper)`; `None` for
/// non-aggregates. The unbounded repr (`SET OF x`) is `(0, None)`.
pub fn agg_bounds(ty: &str) -> Option<(u32, Option<u32>)> {
    agg_parts(ty).map(|(_, lo, hi)| (lo, hi))
}

/// Split an aggregation repr into `(inner, lower, upper)` — handles both the
/// unbounded (`SET OF x`) and bounded (`SET [1:?] OF x`) forms emitted by the
/// schema exporter.
fn agg_parts(ty: &str) -> Option<(&str, u32, Option<u32>)> {
    let rest = ["LIST", "SET", "BAG", "ARRAY"]
        .iter()
        .find_map(|kw| ty.strip_prefix(kw))?;
    if let Some(inner) = rest.strip_prefix(" OF ") {
        return Some((inner.trim(), 0, None));
    }
    // Bounded form: ` [n:m] OF inner` (m may be `?`).
    let rest = rest.strip_prefix(" [")?;
    let (bound, after) = rest.split_once("] OF ")?;
    let (lo, hi) = bound.split_once(':')?;
    Some((
        after.trim(),
        lo.trim().parse().ok()?,
        hi.trim().parse().ok(),
    ))
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
