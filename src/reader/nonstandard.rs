//! Catalogue of non-standard input normalizations.
//!
//! Several CAD systems emit STEP files that violate the ISO 10303 (EXPRESS)
//! schema — a required field is Unset, a `SET[1:?]` is empty, a reference
//! dangles, a unit is misnamed. The reader accepts these *leniently*: it
//! detects the deviation, normalizes to the standard form (or drops the
//! entity when it carries no information), and records a
//! [`ConvertError::NonStandardInput`](crate::ir::error::ConvertError::NonStandardInput)
//! so round-trip analysis does not count the recovery as data loss. The IR
//! itself only ever holds the *standard* shape — non-standard forms never
//! leak past the reader, and the writer always re-emits the standard form
//! (round-trip symmetric).
//!
//! This module holds the [`NsCase`] marker (below) plus the knowledge index:
//! for every lenient path, *which CAD produced what non-standard form, which
//! schema rule it broke, and how the reader accepts it.* Each catalogued case
//! has a stable slug `NS-<slug>` and a matching [`NsCase`] variant. Every NORM
//! recording goes through `ReaderContext::ns_record` / `ns_push`, which take an
//! [`NsCase`], so to find every lenient path in the code:
//!
//! ```text
//! rg "NsCase::" src/           # every recording site + reference
//! ```
//!
//! The `ns_case_slugs_match_catalogue` test asserts the [`NsCase`] variants and
//! the `### NS-<slug>` sections below stay in sync; the
//! `nonstandard_input_only_via_funnel` test asserts no code constructs a NORM
//! warning outside the funnel — so a forgotten marker is impossible.
//!
//! # Why the normalization code is not a separate module
//!
//! Each lenient path is two parts: **detection** (a mostly-pure predicate —
//! "is this set empty / this ref dangling / this factor a degree / this value
//! Unset?") and **recovery** (mapping the non-standard input onto standard
//! IR). Detection could be hoisted into pure helpers, but recovery is by
//! definition a branch of the *standard* handler path and depends on the
//! `ReaderContext`'s id-maps and arenas, so it cannot leave the handler
//! without threading the whole context back in. Cascade cases share a
//! `HashSet` across entities; post-pass cases depend on a fully-built arena.
//! Physically relocating the code would buy little cohesion for real
//! regression risk — so the *knowledge* is centralised here while the code
//! stays where it runs, joined by the typed [`NsCase`] marker.
//!
//! # Two recording mechanisms (note — call convention is inconsistent)
//!
//! Most sites record immediately via `ns_push` (count per occurrence); the
//! surface-style sites instead call `ns_record`, which aggregates into a
//! per-file tally flushed once at the end of `convert`. The GISU post-pass
//! pushes a single aggregated `count`. Unifying every site on the aggregating
//! path would change the emitted warning `count`s (and thus byte output), so it
//! is intentionally **out of scope** — tracked as a possible follow-up.
//!
//! ----------------------------------------------------------------------
//!
//! # ① Fallback-branch cases (try standard → accept non-standard variant)
//!
//! ### NS-shape-aspect-of-shape-pd
//! - **Source**: C3D kernel.
//! - **Schema rule broken**: `SHAPE_ASPECT.of_shape` must reference a
//!   `PRODUCT_DEFINITION_SHAPE`; C3D references a `PRODUCT_DEFINITION` directly.
//! - **Acceptance**: the typed `product_of_pds` probe misses; the fallback
//!   `product_of_pdef` probe resolves to the product, and the PDS link is
//!   reconstructed.
//! - **Writer symmetry**: emits the standard `of_shape = PRODUCT_DEFINITION_SHAPE`
//!   form (via the product's `product_def_shape_ids`).
//! - **Code**: `entities/shape_rep/shape_aspect.rs`.
//! - **Fixtures**: grabcad input-shaft / shaft-238.
//!
//! ### NS-cbu-angle-factor
//! - **Source**: anonymizing tools (rename a unit to a non-standard name, e.g.
//!   `'MIAU'` for degree).
//! - **Schema rule broken**: `CONVERSION_BASED_UNIT.name` no longer identifies
//!   the unit by name.
//! - **Acceptance**: identify the angle unit by its conversion *factor*
//!   (degree's fixed SI-relative factor), falling back to the name; warn when
//!   the factor-derived unit disagrees with the name.
//! - **Writer symmetry**: emits the standard `DEGREE` / `RADIAN` name.
//! - **Code**: `entities/units/shared.rs` (PlaneAngle arm).
//! - **Fixtures**: hvac (2 files).
//!
//! ### NS-cbu-mass-factor
//! - **Source**: anonymizing tools (as above).
//! - **Schema rule broken**: `CONVERSION_BASED_UNIT.name` does not identify the
//!   mass unit by name.
//! - **Acceptance**: identify by conversion factor (mass has a fixed kg base),
//!   name fallback; warn on disagreement.
//! - **Writer symmetry**: emits the standard `POUND` / `GRAM` / `KILOGRAM` name.
//! - **Code**: `entities/units/shared.rs` (Mass arm).
//! - **Fixtures**: hvac (2 files).
//!
//! ### NS-filename-unset
//! - **Source**: grabcad SO14 sensor / blower exports.
//! - **Schema rule broken**: Part 21 `FILE_NAME.originating_system` /
//!   `.authorization` are required `STRING`s (`''` denotes unspecified); the
//!   exporter writes `$` (Unset).
//! - **Acceptance**: `$` is normalized to the empty string rather than
//!   discarding the whole header.
//! - **Writer symmetry**: emits `''`.
//! - **Code**: `reader/header.rs` (`read_header_string`).
//! - **Fixtures**: blower-1, SO14.
//!
//! # ② Drop-on-detect cases (non-standard input carries no information)
//!
//! ### NS-empty-invisibility
//! - **Source**: some grabcad exports.
//! - **Schema rule broken**: `INVISIBILITY.invisible_items` is `SET[1:?]`; an
//!   empty `()` violates the minimum cardinality (and hides nothing).
//! - **Acceptance**: drop as a normalization. INVISIBILITY is a leaf — no
//!   cascade. (Distinct from the *non-empty but fully unresolved* case, which
//!   is surfaced as an `UnexpectedEntityForm` defect, not a normalization.)
//! - **Writer symmetry**: absent on re-read (nothing emitted).
//! - **Code**: `entities/visualization/invisibility.rs`.
//! - **Fixtures**: supercapacitor.
//!
//! ### NS-empty-prrpc
//! - **Source**: some CATIA / Autodesk exports.
//! - **Schema rule broken**: `PRODUCT_RELATED_PRODUCT_CATEGORY.products` is
//!   `SET[1:?]`; an empty `()` relates no products.
//! - **Acceptance**: drop as a normalization; record the id in
//!   `empty_prrpc_refs` so the referencing relationship cascades (see
//!   `NS-empty-prrpc-cascade`).
//! - **Writer symmetry**: absent on re-read.
//! - **Code**: `entities/assembly_product/product_related_product_category.rs`.
//! - **Fixtures**: CATIA / Autodesk (3 files).
//!
//! ### NS-dangling-reference-drop
//! - **Source**: malformed exports — ABC dataset (an `EDGE_CURVE` with
//!   `edge_geometry = #0`, undefined), anonymizing tools / grabcad (a scrubbed
//!   person leaves the `#18446744073709551615` (u64::MAX) sentinel), etc.
//! - **Schema rule broken**: a required reference *dangles* — it points to an
//!   id the file never defines.
//! - **Acceptance**: handled generically in `reader/dispatch.rs`
//!   (`record_drop_or_warn`): when a Simple handler returns a `MissingReference`
//!   whose target is dangling (`graph.get(to).is_none()`) the drop is malformed
//!   *input*, not a step-io coverage gap — record it as a `NonStandardInput`
//!   normalization and seed `nonstandard_dropped_refs` so anything transitively
//!   requiring it cascades the same way (its own `MissingReference` to a seeded
//!   id is reclassified identically). A ref that *is* defined but unmodelled
//!   (`graph.get` is `Some`, not seeded) stays a defect — that is a real gap.
//!   Cascade propagates only through `MissingReference`-shaped drops; the one
//!   `Option`-returning link (`STYLED_ITEM`) checks the set explicitly.
//! - **Writer symmetry**: absent on re-read (the entity built nothing).
//! - **Code**: `reader/dispatch.rs` (`record_drop_or_warn`); seed-surfacing in
//!   `entities/topology/edge_curve.rs` (resolve before `same_sense`),
//!   `entities/plm/person_and_organization.rs` (dangling person/org → Err),
//!   `entities/plm/approval_person_organization.rs`,
//!   `entities/plm/cc_design_person_and_organization_assignment.rs`,
//!   `entities/visualization/styled_item.rs` (Option-path set check).
//! - **Fixtures**: abc (2 files), grabcad person/org (2 files).
//!
//! # ③ Cascade cases (parent normalized → child drops with it)
//!
//! ### NS-empty-prrpc-cascade
//! - **Source / rule**: parent of `NS-empty-prrpc`.
//! - **Acceptance**: a `PRODUCT_CATEGORY_RELATIONSHIP` whose `sub_category` is a
//!   dropped empty PRRPC (`empty_prrpc_refs`) carries no information — drop as a
//!   normalization, not a `MissingReference` defect.
//! - **Writer symmetry**: absent on re-read.
//! - **Code**: `entities/assembly_product/product_category_relationship.rs`.
//!
//! ### NS-dangling-reference-orphan
//! - **Source / rule**: a refinement of `NS-dangling-reference-drop` for
//!   `EDGE_LOOP`. Unlike faces / shells / solids (emitted from flat arenas, so a
//!   dropped container does not orphan its built members), an `ORIENTED_EDGE` is
//!   emitted *only* via its `EDGE_LOOP`. When a loop drops because one member is
//!   a dangling-reference cascade, its other (successfully resolved) members are
//!   good edges that now emit nowhere — they orphan.
//! - **Acceptance**: `EDGE_LOOP` resolves all members; if any is a cascade drop
//!   (and none is a genuine miss), it records each resolved member as a dropped
//!   `ORIENTED_EDGE` (so a round-trip checker subtracts them) and returns a
//!   `MissingReference` to the cascade member so the dispatcher reclassifies the
//!   loop itself. A genuine missing member stays a defect (no orphan record).
//! - **Writer symmetry**: absent on re-read.
//! - **Code**: `entities/topology/edge_loop.rs`.
//!
//! # ④ Post-pass cases (recovered after the arena is fully built)
//!
//! ### NS-gisu-unset-used-rep
//! - **Source**: CATIA (emits `$` for "Solid" GISUs).
//! - **Schema rule broken**: `GEOMETRIC_ITEM_SPECIFIC_USAGE.used_representation`
//!   is a required `representation`; CATIA writes `$`.
//! - **Acceptance**: the standard value (the WHERE-rule container of
//!   `identified_item`) is not referenced by this GISU, so dispatch order gives
//!   no guarantee the container was read first — the read side *defers* the `$`
//!   case and `resolve_deferred_gisu_used_representation` derives the container
//!   in a post-pass. (When no container is found, the GISU is dropped as an
//!   `UnexpectedEntityForm` defect, not a normalization.)
//! - **Writer symmetry**: emits the derived container ref (standard form).
//! - **Code**: `entities/shape_rep/geometric_item_specific_usage.rs` (detect /
//!   defer), `reader/mod.rs` `resolve_deferred_gisu_used_representation`
//!   (recover) — two sites, one slug.
//! - **Fixtures**: work-holding.
//!
//! # ⑤ Aggregated (`ns_record`) cases
//!
//! ### NS-surface-style-rendering-method
//! - **Source**: exporters that omit or mis-spell the shading method.
//! - **Schema rule broken**: `SURFACE_STYLE_RENDERING(.._WITH_PROPERTIES)
//!   .rendering_method` is a required enum; some files write `$` or an
//!   unrecognized value.
//! - **Acceptance**: normalize to `NORMAL_SHADING`; aggregated via
//!   `ns_record`.
//! - **Writer symmetry**: emits `NORMAL_SHADING`.
//! - **Code**: `entities/visualization/surface_style_rendering.rs`
//!   (`read_rendering_method`).
//!
//! ### NS-surface-style-surface-colour
//! - **Source**: exporters that leave the colour unset.
//! - **Schema rule broken**: `SURFACE_STYLE_RENDERING(.._WITH_PROPERTIES)
//!   .surface_colour` is required in EXPRESS; some files write `$`.
//! - **Acceptance**: normalize to a bare `COLOUR()` (`Colour::Itself`, the
//!   schema's unspecified-colour placeholder) rather than fabricating a
//!   specific colour; aggregated via `ns_record`.
//! - **Writer symmetry**: emits `COLOUR()`.
//! - **Code**: `entities/visualization/surface_style_rendering.rs`
//!   (`read_surface_colour`).
//!
//! ### NS-psa-bare-null-style
//! - **Source**: exporters that abbreviate the null style placeholder.
//! - **Schema rule broken**: a `PRESENTATION_STYLE_ASSIGNMENT.styles`
//!   (`presentation_style_select`) member is written as a bare `.NULL.` enum
//!   instead of the typed `NULL_STYLE(.NULL.)` placeholder.
//! - **Acceptance**: accept the bare enum as `PsaStyle::Null`; aggregated via
//!   `ns_record`.
//! - **Writer symmetry**: emits the standard typed `NULL_STYLE(.NULL.)`.
//! - **Code**: `entities/visualization/presentation_style_assignment.rs`
//!   (`normalize_psa_styles_attr`).
//!
//! ### NS-repr-context-unset
//! - **Source**: C3D kernel (input-shaft / shaft-238).
//! - **Schema rule broken**: `representation.context_of_items` is required in
//!   EXPRESS; the descriptive `REPRESENTATION` bound to a property is emitted
//!   with `$` (Unset).
//! - **Acceptance**: accept as no context (the descriptive representation
//!   carries no geometry context) rather than dropping the whole
//!   `REPRESENTATION` (and cascading its `PROPERTY_DEFINITION_REPRESENTATION`);
//!   aggregated via `ns_record`.
//! - **Writer symmetry**: the descriptive REPRESENTATION is emitted by the PDR
//!   writer; a `None` context re-emits `$`.
//! - **Code**: `entities/property/property_definition_representation.rs`.
//!
//! ### NS-ratio-unit-dimensions-unset
//! - **Source**: C3D kernel (input-shaft / shaft-238).
//! - **Schema rule broken**: `named_unit.dimensions` is a required
//!   `dimensional_exponents` (not OPTIONAL, not DERIVE in `ratio_unit`); the
//!   standalone simple `RATIO_UNIT` entity is emitted with `$` (Unset).
//! - **Acceptance**: accept as no explicit dimensions — the `ratio_unit` WHERE
//!   clause fixes every exponent to zero regardless — rather than dropping the
//!   unit; aggregated via `ns_record`.
//! - **Writer symmetry**: a `None` `dim_exp` on the simple form re-emits `$`.
//! - **Code**: `entities/units/ratio_unit.rs` (`RatioUnitSimpleHandler`).
//!
//! ### NS-pcurve-3d-in-pspace
//! - **Source**: grabcad / OCCT (micromachines Axle Cut Tool, Jigsaw Tshank,
//!   MicroRallyCar).
//! - **Schema rule broken**: EXPRESS `pcurve.wr3` requires
//!   `reference_to_curve.items[1].geometric_representation_item.dim = 2` — the
//!   curve in a `PCURVE`'s parameter-space `DEFINITIONAL_REPRESENTATION` must be
//!   2D. These files put a **3D** curve (e.g. a `TRIMMED_CURVE` on a 3D `CIRCLE`
//!   / `AXIS2_PLACEMENT_3D` with 3D trim points) inside a 2D
//!   `PARAMETRIC_REPRESENTATION_CONTEXT('pspace')`.
//! - **Acceptance**: step-io's pcurve-subtree partition routes the subtree to
//!   the 2D handlers, which cannot model the 3D geometry, so the `PCURVE` (and
//!   its orphaned `DEFINITIONAL_REPRESENTATION` / `CIRCLE` / `TRIMMED_CURVE` /
//!   `AXIS2_PLACEMENT_3D` subtree) is dropped. The drop is classified per
//!   dropped type via `ns_record` (`"dropped …"`), gated on
//!   `is_geometry_registered` so genuine survivors (`CARTESIAN_POINT` /
//!   `DIRECTION` / `VECTOR`, and curves shared from outside the subtree) are not
//!   counted. The 3D `SURFACE_CURVE` that owns the pcurve survives on its 3D
//!   `curve_3d`.
//! - **All-wr3 surface curve**: when *every* `associated_geometry` member of a
//!   `SURFACE_CURVE` / `SEAM_CURVE` is a wr3-dropped PCURVE (MicroRallyCar: 4
//!   curves), the body has no resolvable geometry and no arena node is pushed
//!   (the edge falls back to the `curve_3d` alias). `lower_surface_curve_data`
//!   records this as the cascade
//!   (`ns_record("SURFACE_CURVE" / "SEAM_CURVE", "dropped … pcurve.wr3
//!   cascade")`), gated on `wr3_dropped == member_refs.len()` so curves with a
//!   surviving 2D member (or a non-wr3 failure) keep their defect warning. The
//!   owned 3D `curve_3d` still survives, so the referencing `EDGE_CURVE` is not
//!   affected.
//! - **Writer symmetry**: none — the subtree is dropped, output unchanged
//!   (merkle-neutral).
//! - **Code**: `reader/mod.rs` (`ReaderContext::record_pcurve_wr3_drop`),
//!   `entities/geometry/surface_curve.rs` (per-pcurve and all-wr3 wrapper drop
//!   sites).
//!
//! ### NS-psa-styles-unset
//! - **Source**: NIST ctc_05.
//! - **Schema rule broken**: `presentation_style_assignment.styles` is a
//!   mandatory `SET[1:?] OF presentation_style_select`; emitted as `$` (Unset).
//! - **Acceptance**: accept as an empty style set rather than a defect — the
//!   PSA is still built and re-emits `()` (merkle-stable). Same spirit as the
//!   sibling NS-psa-bare-null-style.
//! - **Code**: `entities/visualization/presentation_style_assignment.rs`
//!   (`normalize_psa_styles_attr`).
//!
//! ### NS-orsi-over-ridden-unset
//! - **Source**: NIST ctc_05.
//! - **Schema rule broken**: `over_riding_styled_item.over_ridden_style :
//!   styled_item` is mandatory; emitted as `$` (Unset).
//! - **Acceptance**: without an over-ridden target the entity carries no
//!   information → drop as a `NonStandardInput` normalization (not a defect).
//!   No cascade (nothing references the dropped ORSI).
//! - **Code**: `entities/visualization/over_riding_styled_item.rs`.
//!
//! ### NS-general-datum-reference-of-shape-unset
//! - **Source**: NIST ctc_05.
//! - **Schema rule broken**: `shape_aspect.of_shape : product_definition_shape`
//!   is mandatory (and `UNIQUE id, of_shape`); a `DATUM_REFERENCE_ELEMENT` /
//!   `DATUM_REFERENCE_COMPARTMENT` (general_datum_reference subtype) emits `$`.
//! - **Acceptance**: the entity has no resolvable owning product → drop as a
//!   `NonStandardInput` (not an AttributeType defect). A `COMMON_DATUM_LIST`
//!   compartment whose member dropped for this reason cascades and is recorded
//!   too (gated on the member's own of_shape=$ so an unrelated drop stays a
//!   plain drop, not a normalization).
//! - **Code**: `entities/pmi.rs` (`read_general_datum_reference_data`).
//!
//! ### NS-tagless-parameter-value
//! - **Source**: abc (various exporters).
//! - **Schema rule broken**: `trimming_select`'s `parameter_value` (a defined
//!   REAL type) must be tagged `PARAMETER_VALUE(x)` inside the select; emitted as
//!   a bare real `( 0.0 )`.
//! - **Acceptance**: the handler normalizes bare `Real`/`Integer` in the trim
//!   slots to `PARAMETER_VALUE(value)` *before* the strict generated bind
//!   (`ir::attr::normalize_tagless_select`), so the IR holds the standard form and
//!   the writer re-emits `PARAMETER_VALUE`. Recorded as `NonStandardInput`.
//! - **Code**: `entities/geometry/trimmed_curve.rs`.
//!
//! ### NS-non-standard-enum-value
//! - **Source**: any exporter writing an enum token outside the EXPRESS
//!   enumeration (latent — not seen in the corpus).
//! - **Schema rule broken**: an ENUM field carries a token that is not one of
//!   the EXPRESS members (e.g. `surface_side`, `transition_code`,
//!   `trimming_preference`, `marker_type`).
//! - **Acceptance**: the generated `bind` is strict (no `default`/`catch_all`) —
//!   it neither guesses a default nor preserves the raw token. A required-field
//!   enum returns `ConvertError::NonStandardEnumValue`, which the dispatcher
//!   (`record_drop_or_warn`) reclassifies as a `NonStandardInput` drop (NORM);
//!   an OPTIONAL SELECT member (`marker_type`) binds to `None` so the entity's
//!   `bind` returns `Ok(None)` and the handler records the drop. Rejecting a
//!   non-standard value is correct behaviour, so it is **NORM, not LOSS**.
//! - **Writer symmetry**: absent on re-read (the entity was dropped).
//! - **Code**: `reader/dispatch.rs` (`record_drop_or_warn`) for required-field
//!   enums; `entities/visualization/point_style.rs` for the `marker_type`
//!   SELECT member. Strict bind: `early/generated/bind.rs`.
//!
//! ----------------------------------------------------------------------
//!
//! # gen-early (2-layer) normalization — principle & residual generated-bind leniencies
//!
//! **Principle**: the `bind` layer gen-early generates (L1) is a *strict* parser
//! matching the EXPRESS schema exactly. Non-standard-input tolerance lives only
//! in the hand-written layer:
//!
//! | case | where | how |
//! |---|---|---|
//! | value-form deviation (bare→TAG, typo token, empty→`$`[optional]) | handler (pre-bind) | normalize attrs → strict bind, `NonStandardInput` |
//! | resolution-dependent / post-parse | `lower` (has ctx) | decide + `NonStandardInput` |
//! | known unrecoverable | handler / lower / dispatcher | drop + `NonStandardInput("dropped")` → NORM |
//! | non-standard enum token | strict `bind` `Err`(`NonStandardEnumValue`) / SELECT `None` | dispatcher / handler reclassifies as a `NonStandardInput` drop → NORM (rejecting a non-standard value is correct, not LOSS — see `NS-non-standard-enum-value`) |
//! | unknown / malformed syntax (step-io gap) | strict `bind` `Err` | dispatcher pushes a defect warning + drops → LOSS (automatic) |
//! | entity not strict-bindable at all | — | hold back (stay hand-written) |
//!
//! `NS-tagless-parameter-value` above is the first instance (value-form, handler).
//!
//! **Residual uniform leniency still baked into the generated `bind`** (NOT a
//! `### NS-` slug, no per-instance anchor):
//! - **`Integer`→`Real` coercion** (`read_real`): e.g. `PARAMETER_VALUE(5)` accepted
//!   as a real. A lexical (number-format) tolerance — lossless, no guess — not an
//!   entity-level non-standard recovery. Universal Part21 practice, kept.
//!
//! (The former enum `default` guess and `marker_type` `catch_all` were removed —
//! both are now strict; a non-standard enum token is dropped as a NORM
//! normalization. See `NS-non-standard-enum-value`.)

// ---------------------------------------------------------------------------
// Typed non-standard case marker
// ---------------------------------------------------------------------------

/// A non-standard input case. Every NORM recording goes through
/// [`ReaderContext::ns_record`](crate::reader::ReaderContext) /
/// [`ns_push`](crate::reader::ReaderContext), both of which take one of these,
/// so `rg "NsCase::"` locates every lenient path and the compiler rejects an
/// unknown / mistyped case. Each variant maps 1:1 to one catalogue section
/// above; the `ns_case_slugs_match_catalogue` test enforces the correspondence.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum NsCase {
    ShapeAspectOfShapePd,
    CbuAngleFactor,
    CbuMassFactor,
    FilenameUnset,
    EmptyInvisibility,
    EmptyPrrpc,
    EmptyPrrpcCascade,
    DanglingReferenceDrop,
    DanglingReferenceOrphan,
    GisuUnsetUsedRep,
    SurfaceStyleRenderingMethod,
    SurfaceStyleSurfaceColour,
    PsaBareNullStyle,
    ReprContextUnset,
    RatioUnitDimensionsUnset,
    Pcurve3dInPspace,
    PsaStylesUnset,
    OrsiOverRiddenUnset,
    GeneralDatumReferenceOfShapeUnset,
    TaglessParameterValue,
    NonStandardEnumValue,
}

// `ALL` and `slug` exist only to drive the `ns_case_slugs_match_catalogue`
// drift test; the marker itself is the constructed variant at each call site.
#[cfg(test)]
impl NsCase {
    /// All cases — kept in sync with the catalogue sections by the
    /// `ns_case_slugs_match_catalogue` test.
    pub(crate) const ALL: &'static [NsCase] = &[
        NsCase::ShapeAspectOfShapePd,
        NsCase::CbuAngleFactor,
        NsCase::CbuMassFactor,
        NsCase::FilenameUnset,
        NsCase::EmptyInvisibility,
        NsCase::EmptyPrrpc,
        NsCase::EmptyPrrpcCascade,
        NsCase::DanglingReferenceDrop,
        NsCase::DanglingReferenceOrphan,
        NsCase::GisuUnsetUsedRep,
        NsCase::SurfaceStyleRenderingMethod,
        NsCase::SurfaceStyleSurfaceColour,
        NsCase::PsaBareNullStyle,
        NsCase::ReprContextUnset,
        NsCase::RatioUnitDimensionsUnset,
        NsCase::Pcurve3dInPspace,
        NsCase::PsaStylesUnset,
        NsCase::OrsiOverRiddenUnset,
        NsCase::GeneralDatumReferenceOfShapeUnset,
        NsCase::TaglessParameterValue,
        NsCase::NonStandardEnumValue,
    ];

    /// The stable `NS-<slug>` identifying this case's catalogue section.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            NsCase::ShapeAspectOfShapePd => "NS-shape-aspect-of-shape-pd",
            NsCase::CbuAngleFactor => "NS-cbu-angle-factor",
            NsCase::CbuMassFactor => "NS-cbu-mass-factor",
            NsCase::FilenameUnset => "NS-filename-unset",
            NsCase::EmptyInvisibility => "NS-empty-invisibility",
            NsCase::EmptyPrrpc => "NS-empty-prrpc",
            NsCase::EmptyPrrpcCascade => "NS-empty-prrpc-cascade",
            NsCase::DanglingReferenceDrop => "NS-dangling-reference-drop",
            NsCase::DanglingReferenceOrphan => "NS-dangling-reference-orphan",
            NsCase::GisuUnsetUsedRep => "NS-gisu-unset-used-rep",
            NsCase::SurfaceStyleRenderingMethod => "NS-surface-style-rendering-method",
            NsCase::SurfaceStyleSurfaceColour => "NS-surface-style-surface-colour",
            NsCase::PsaBareNullStyle => "NS-psa-bare-null-style",
            NsCase::ReprContextUnset => "NS-repr-context-unset",
            NsCase::RatioUnitDimensionsUnset => "NS-ratio-unit-dimensions-unset",
            NsCase::Pcurve3dInPspace => "NS-pcurve-3d-in-pspace",
            NsCase::PsaStylesUnset => "NS-psa-styles-unset",
            NsCase::OrsiOverRiddenUnset => "NS-orsi-over-ridden-unset",
            NsCase::GeneralDatumReferenceOfShapeUnset => {
                "NS-general-datum-reference-of-shape-unset"
            }
            NsCase::TaglessParameterValue => "NS-tagless-parameter-value",
            NsCase::NonStandardEnumValue => "NS-non-standard-enum-value",
        }
    }
}

#[cfg(test)]
mod ns_marker_tests {
    use super::NsCase;
    use std::collections::BTreeSet;

    /// Every `NsCase` variant must have a matching `NS-<slug>` catalogue section
    /// and vice versa — keeps the typed marker and the knowledge catalogue from
    /// drifting apart.
    #[test]
    fn ns_case_slugs_match_catalogue() {
        let src = include_str!("nonstandard.rs");
        let mut sections: BTreeSet<&str> = BTreeSet::new();
        for line in src.lines() {
            let t = line.trim_start_matches("//!").trim();
            if let Some(rest) = t.strip_prefix("### ") {
                let slug = rest.trim();
                if slug.starts_with("NS-") {
                    sections.insert(slug);
                }
            }
        }
        let variants: BTreeSet<&str> = NsCase::ALL.iter().map(|c| c.slug()).collect();
        assert_eq!(
            variants, sections,
            "NsCase variants and the NS- catalogue sections drifted apart"
        );
    }

    /// `ConvertError::NonStandardInput` may only be *constructed* in the funnel
    /// (`reader/mod.rs`) and defined in `ir/error.rs`; every other NORM
    /// recording must go through `ReaderContext::ns_record` / `ns_push` (which
    /// take an `NsCase`), so a forgotten marker is structurally impossible.
    /// Scans `src`, skipping comment lines (catalogue prose is harmless) and
    /// test files (which only pattern-match the variant).
    #[test]
    fn nonstandard_input_only_via_funnel() {
        // Assembled at runtime so this test's own source does not self-match.
        let needle = format!("{}{}", "NonStandardInput ", "{");
        let allow = ["ir/error.rs", "reader/mod.rs"];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        visit(&root, &needle, &allow, &mut offenders);
        assert!(
            offenders.is_empty(),
            "NonStandardInput constructed outside the funnel: {offenders:?}"
        );
    }

    fn visit(dir: &std::path::Path, needle: &str, allow: &[&str], out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, needle, allow, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let rel = path.to_string_lossy().replace('\\', "/");
            // Skip the variant definition / funnel home and any test file
            // (tests only pattern-match the variant, never gate production NORM).
            if allow.iter().any(|a| rel.ends_with(a)) || rel.contains("tests") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                    continue;
                }
                if line.contains(needle) {
                    out.push(rel.clone());
                    break;
                }
            }
        }
    }
}
