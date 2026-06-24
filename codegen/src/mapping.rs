//! `schema/mapping.toml` model — the subset of hand-hints the schema-faithful
//! generator consumes: `[codegen].domains.*` seed lists and
//! `[codegen].complex_parts` (which entities form the generic part-bag).
//!
//! DERIVE (`*`) markers are NO LONGER read from mapping — they come from
//! `schema/universal.toml`'s per-entity `derives` (auto-extracted from EXPRESS).
//! The mapping.toml `[derived]`/`[complex.*]` sections remain for gen-early.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Mapping {
    #[serde(default)]
    pub codegen: CodegenHints,
}

#[derive(Deserialize, Default)]
pub struct CodegenHints {
    /// `domains.units` / `domains.geometry` / `domains.topology` seed lists.
    #[serde(default)]
    pub domains: BTreeMap<String, Vec<String>>,
    /// Per-domain entity names that appear only as parts of a complex instance
    /// (collected into one generic part-bag, not given simple arenas). Still
    /// manual: complex-part membership automation needs dual-appearance (Phase 1/2).
    #[serde(default)]
    pub complex_parts: BTreeMap<String, Vec<String>>,
}
