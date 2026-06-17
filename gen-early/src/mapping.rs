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
    /// Multi-part (complex) entities. Each part contributes its `own_attrs` (from
    /// early.toml) in order; the generated bind reads per-part via
    /// `require_part_attrs`, serialize emits a `WriterBody::Complex` via
    /// `push_complex`. Two shapes (exactly one set per entity):
    /// - `parts`: a SINGLE case (the part order must match the handler's
    ///   `#[step_entity_complex] cases`). The common case.
    /// - `cases`: MULTI-case (e.g. units SI vs `CONVERSION_BASED_UNIT`) — the L1
    ///   becomes an enum (one variant per case), bind discriminates by part
    ///   presence, serialize matches the variant.
    #[serde(rename = "complex", default)]
    pub(crate) complex: BTreeMap<String, ComplexHint>,
}

#[derive(Deserialize)]
pub(crate) struct ComplexHint {
    /// Single-case: ordered part (supertype) names,
    /// e.g. `["BOUNDED_CURVE", "B_SPLINE_CURVE", …]`. Mutually exclusive with
    /// `cases`.
    #[serde(default)]
    pub(crate) parts: Option<Vec<String>>,
    /// Multi-case: `case-name -> CaseHint`. The L1 is an enum whose variants are
    /// the (pascal-cased) case names. Mutually exclusive with `parts`.
    #[serde(default)]
    pub(crate) cases: Option<BTreeMap<String, CaseHint>>,
}

/// One case of a multi-case complex entity ([`ComplexHint::cases`]).
#[derive(Deserialize)]
pub(crate) struct CaseHint {
    /// Ordered part names for this case (= serialize emit order).
    pub(crate) parts: Vec<String>,
    /// EXPRESS Derived (`*`) attr names *for this case* (e.g. `NAMED_UNIT.
    /// dimensions` is `*` in the SI case but an explicit ref in the CBU case).
    /// Per-case because the same part's attr differs across cases.
    #[serde(default)]
    pub(crate) derived: Vec<String>,
    /// Part whose presence selects this case in the generated bind (e.g.
    /// `CONVERSION_BASED_UNIT`). `None` => the fallback case when no other
    /// case's discriminant matches.
    #[serde(default)]
    pub(crate) discriminant: Option<String>,
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
