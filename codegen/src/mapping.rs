//! `schema/mapping.toml` model — the subset of hand-hints the schema-faithful
//! generator consumes: `[codegen].domains.*` seed lists, `[complex.*]` part
//! orders/cases, and `[derived]` `*`-slot attribute names.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Mapping {
    #[serde(default)]
    pub codegen: CodegenHints,
    /// Per-entity EXPRESS Derived (`*`) attribute names (single-case complexes
    /// and simple entities like `oriented_edge`).
    #[serde(default)]
    pub derived: BTreeMap<String, Vec<String>>,
    /// Multi-part (complex) entities. (Reserved for Phase 1 complex part
    /// order/case handling; merkle is part-order-invariant so unused in Phase 0.)
    #[serde(rename = "complex", default)]
    #[allow(dead_code)]
    pub complex: BTreeMap<String, ComplexHint>,
}

#[derive(Deserialize, Default)]
pub struct CodegenHints {
    /// `domains.units` / `domains.geometry` / `domains.topology` seed lists.
    #[serde(default)]
    pub domains: BTreeMap<String, Vec<String>>,
    /// Per-domain entity names that appear only as parts of a complex instance
    /// (collected into one generic part-bag, not given simple arenas).
    #[serde(default)]
    pub complex_parts: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[allow(dead_code)] // reserved for Phase 1 (see `complex` field note)
pub struct ComplexHint {
    /// Single-case ordered part names.
    #[serde(default)]
    pub parts: Option<Vec<String>>,
    /// Multi-case: case-name -> CaseHint.
    #[serde(default)]
    pub cases: Option<BTreeMap<String, CaseHint>>,
    #[serde(default)]
    pub also_simple: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)] // reserved for Phase 1 (see `complex` field note)
pub struct CaseHint {
    pub parts: Vec<String>,
    /// Attr names on a part that are `*` (derived) in this case.
    #[serde(default)]
    pub derived: Vec<String>,
    /// Single part name whose presence selects this case.
    #[serde(default)]
    pub discriminant: Option<String>,
    /// Exact part-set that selects this case (when discriminant is ambiguous).
    #[serde(default)]
    pub match_parts: Option<Vec<String>>,
}
