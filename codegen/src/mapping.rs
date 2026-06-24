//! `schema/mapping.toml` model — the only hand-hint the schema-faithful
//! generator now consumes: `[codegen].domains.*` closure seed lists.
//!
//! DERIVE (`*`) markers and complex-part membership are NO LONGER read from
//! mapping — they come from `schema/universal.toml` (`derives` + `is_complex_part`,
//! auto-extracted from EXPRESS). The mapping.toml `[derived]`/`[complex.*]`
//! sections remain for gen-early.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Mapping {
    #[serde(default)]
    pub codegen: CodegenHints,
}

#[derive(Deserialize, Default)]
pub struct CodegenHints {
    /// `domains.units` / `domains.geometry` / `domains.topology` closure seeds.
    #[serde(default)]
    pub domains: BTreeMap<String, Vec<String>>,
    /// Entities kept OUT of the closure (never generated). Their referrers
    /// escape via the round-trip filter. Used to defer entities whose attrs need
    /// a generator capability not yet built (e.g. SELECT-with-aggregate-arms).
    #[serde(default)]
    pub exclude: Vec<String>,
}
