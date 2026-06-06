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
//! This module is **documentation only** (no code). It is the single place
//! that answers, for every lenient path: *which CAD produced what
//! non-standard form, which schema rule it broke, and how the reader accepts
//! it.* Each catalogued case has a stable slug `NS-<slug>`; the corresponding
//! handler code carries a `// [NS-<slug>] …` anchor comment. To audit
//! coverage, compare the slugs:
//!
//! ```text
//! grep -rn "\[NS-" src/        # every anchor in the code
//! ```
//!
//! against the `### NS-<slug>` sections below — the two slug sets must match.
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
//! stays where it runs, joined by the `NS-` anchors.
//!
//! # Two recording mechanisms (note — call convention is inconsistent)
//!
//! Most sites push `ConvertError::NonStandardInput { count: 1, .. }`
//! immediately; the surface-style sites instead call
//! [`ReaderContext::record_nonstandard`](crate::reader::ReaderContext) which
//! aggregates into a per-file tally flushed once at the end of `convert`. The
//! GISU post-pass pushes a single aggregated `count`. Unifying every site on
//! the aggregating path would change the emitted warning `count`s (and thus
//! byte output), so it is intentionally **out of scope** for this catalogue —
//! tracked as a possible follow-up.
//!
//! ----------------------------------------------------------------------
//!
//! # ① Fallback-branch cases (try standard → accept non-standard variant)
//!
//! ### NS-shape-aspect-of-shape-pd
//! - **Source**: C3D kernel.
//! - **Schema rule broken**: `SHAPE_ASPECT.of_shape` must reference a
//!   `PRODUCT_DEFINITION_SHAPE`; C3D references a `PRODUCT_DEFINITION` directly.
//! - **Acceptance**: lookup chain falls through `pdef_shape_to_pdef` to
//!   `pdef_to_product`, resolving to the product; the PDS link is reconstructed.
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
//! ### NS-dangling-person-org
//! - **Source**: anonymizing tools / grabcad (scrub a person, leaving a
//!   `#18446744073709551615` (u64::MAX) sentinel undefined in the file).
//! - **Schema rule broken**: required `the_person` / `the_organization`
//!   reference is dangling (points to no defined entity).
//! - **Acceptance**: when the ref is genuinely dangling (vs. merely an
//!   unmodelled but defined PERSON/ORGANIZATION, which stays a silent drop),
//!   drop as a normalization and record `nonstd_person_org_refs` so
//!   assignments / approvals cascade (see `NS-dangling-person-org-cascade`).
//! - **Writer symmetry**: absent on re-read.
//! - **Code**: `entities/plm/person_and_organization.rs`.
//! - **Fixtures**: grabcad (2 files).
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
//! ### NS-dangling-person-org-cascade
//! - **Source / rule**: parent of `NS-dangling-person-org`.
//! - **Acceptance**: an `APPROVAL_PERSON_ORGANIZATION` or
//!   `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT` referencing a dropped
//!   non-standard P&O (`nonstd_person_org_refs`) drops as a normalization too.
//!   (A reference to a P&O dropped for *other* reasons stays a silent drop.)
//! - **Writer symmetry**: absent on re-read.
//! - **Code**: `entities/plm/approval_person_organization.rs`,
//!   `entities/plm/cc_design_person_and_organization_assignment.rs` (two sites,
//!   one slug).
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
//! # ⑤ Aggregated (`record_nonstandard`) cases
//!
//! ### NS-surface-style-rendering-method
//! - **Source**: exporters that omit or mis-spell the shading method.
//! - **Schema rule broken**: `SURFACE_STYLE_RENDERING(.._WITH_PROPERTIES)
//!   .rendering_method` is a required enum; some files write `$` or an
//!   unrecognized value.
//! - **Acceptance**: normalize to `NORMAL_SHADING`; aggregated via
//!   `record_nonstandard`.
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
//!   specific colour; aggregated via `record_nonstandard`.
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
//!   `record_nonstandard`.
//! - **Writer symmetry**: emits the standard typed `NULL_STYLE(.NULL.)`.
//! - **Code**: `entities/visualization/presentation_style_assignment.rs`
//!   (`parse_psa_styles`).
//!
//! ### NS-repr-context-unset
//! - **Source**: C3D kernel (input-shaft / shaft-238).
//! - **Schema rule broken**: `representation.context_of_items` is required in
//!   EXPRESS; the descriptive `REPRESENTATION` bound to a property is emitted
//!   with `$` (Unset).
//! - **Acceptance**: accept as no context (the descriptive representation
//!   carries no geometry context) rather than dropping the whole
//!   `REPRESENTATION` (and cascading its `PROPERTY_DEFINITION_REPRESENTATION`);
//!   aggregated via `record_nonstandard`.
//! - **Writer symmetry**: the descriptive REPRESENTATION is emitted by the PDR
//!   writer; a `None` context re-emits `$`.
//! - **Code**: `entities/property/property_definition_representation.rs`.
