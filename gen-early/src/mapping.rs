//! `schema/mapping.toml` model — hand-authored generation hints.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Mapping {
    pub(crate) generate: Vec<String>,
    /// Diagnostic flip: when true, generate *every* entity the codegen can
    /// handle (filtered by [`Ctx::entity_supported`]) + a coverage report,
    /// instead of just the wired `generate` list. Committed as `false`.
    #[serde(default)]
    pub(crate) generate_all: bool,
    /// Entities that additionally get a `serialize_<entity>_with_id` variant
    /// (`push_simple_with_id` under a pre-reserved id) — for writer paths
    /// using the reserve-then-fill pattern.
    #[serde(default)]
    pub(crate) serialize_with_id: Vec<String>,
    #[serde(rename = "enum", default)]
    pub(crate) enums: BTreeMap<String, EnumHint>,
    #[serde(rename = "select", default)]
    pub(crate) selects: BTreeMap<String, SelectHint>,
    /// Per-entity EXPRESS Derived (`*`) attribute names. early.toml (a generated
    /// artifact) cannot carry DERIVE info, so it is hand-authored here. A listed
    /// attr is omitted from the L1 struct + bind (its positional slot is kept so
    /// later slots read at the right index) and serialized as `*`.
    #[serde(default)]
    pub(crate) derived: BTreeMap<String, Vec<String>>,
    /// Entities generated as model + bind only (serialize skipped). For
    /// read-back-only minority forms the writer normalizes away (e.g.
    /// `QUASI_UNIFORM_CURVE` → `B_SPLINE_CURVE_WITH_KNOTS`): they are read but
    /// never emitted, so their handler `write` is `unreachable!()`. Precondition:
    /// any enum/select such an entity binds must also be used by a serialized
    /// entity (enum/select helper emission keys off serialize usage); otherwise
    /// the generated bind fails to compile.
    #[serde(default)]
    pub(crate) read_only: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct EnumHint {
    /// Full path of the reused L2 enum, e.g. `crate::ir::visualization::Projection`.
    pub(crate) rust_type: String,
    /// early.toml ENUM member -> L2 variant. STEP token = member upper-cased.
    pub(crate) variants: BTreeMap<String, String>,
    /// Unknown STEP token -> this variant (matches a hand-written `_ => …`
    /// catch-all). `None` => error on unknown.
    #[serde(default)]
    pub(crate) default: Option<String>,
    /// Variant that carries the raw token `String` for unknown values (e.g.
    /// `MarkerType::Other(token)`). Used by the token-based helpers a SELECT
    /// member needs. `None` => no such variant.
    #[serde(default)]
    pub(crate) catch_all: Option<String>,
}

/// Hint for a *mixed* SELECT (members span entity / enum / primitive) -> a
/// synthesized `Early*` enum. Member kinds and any enum hint are auto-derived.
#[derive(Deserialize)]
pub(crate) struct SelectHint {
    /// Synthesized enum name, e.g. `EarlyMarker` (lives in generated/model.rs).
    pub(crate) rust_type: String,
    /// SELECT member -> L1 variant name.
    pub(crate) variants: BTreeMap<String, String>,
}

// ---- classification ----
