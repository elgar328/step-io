//! `codegen/coverage.toml` model — the hand-curated codegen support boundary
//! (NOT corpus-automated). This is codegen POLICY (how far the generator
//! reaches), kept separate from `schema/*.toml` schema data and from
//! gen-early's `schema/mapping.toml` hints.
//!
//! - `cover`:   closure seed lists (per domain key); flattened into one seed set.
//! - `exclude`: entities kept out of the closure entirely.
//! - `narrow`:  per-SELECT member allow-lists (restrict closure + arm emission).

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Coverage {
    /// Domain-keyed closure seed lists (`[cover]`); main flattens `values()`.
    #[serde(default)]
    pub cover: BTreeMap<String, Vec<String>>,
    /// Entities kept OUT of the closure (never generated). Their referrers
    /// escape via the round-trip filter. Used to defer entities whose attrs need
    /// a generator capability not yet built, or for closure SCOPE.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Per-SELECT member allow-lists (`[narrow]`): `alias -> kept members`.
    /// Restricts which members of a mega-SELECT enter the closure / get arms.
    #[serde(default)]
    pub narrow: BTreeMap<String, Vec<String>>,
}
