//! PMI scaffolding — `SHAPE_ASPECT` layer that anchors future PMI work
//! (Tolerance / Datum / GD&T per ROADMAP Phase 2).
//!
//! Currently only a passive round-trip of `SHAPE_ASPECT` entities. Pattern
//! B `PROPERTY_DEFINITION` chains (target = `SHAPE_ASPECT`, with
//! `DERIVED_UNIT` items) and `ID_ATTRIBUTE` / `NAME_ATTRIBUTE` standalone
//! annotations are out of scope until the PMI layer expands. The
//! `PmiPool` namespace is reserved here so that future arenas
//! (tolerances, datums, dimensional locations, annotations) plug in
//! without reshaping `StepModel`.
//!
//! The `Arena<ShapeAspect>` shape mirrors the geometry / topology pools —
//! once a kernel adapter wants to attach a tolerance to a shape aspect,
//! the `ShapeAspectId` ref is stable.

use super::arena::Arena;
use super::id::ProductId;

/// Top-level container for PMI data. `None` on `StepModel` when the source
/// file had no PMI (the common case for non-NIST / non-stepcode tiers).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PmiPool {
    /// `SHAPE_ASPECT` entries — emit order preserved.
    pub shape_aspects: Arena<ShapeAspect>,
}

/// `SHAPE_ASPECT(name, description, of_shape, product_definitional)`.
///
/// `of_shape` is a `PRODUCT_DEFINITION_SHAPE` reference resolved to a
/// `ProductId` at read time via the existing `pdef_shape_to_pdef` and
/// `pdef_to_product` maps. SAs whose `of_shape` does not resolve are
/// silently dropped on read (symmetric ignorance preserves round-trip
/// equality for fixtures with non-standard targets).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeAspect {
    /// `SHAPE_ASPECT.name` — typically `''`.
    pub name: String,
    /// `SHAPE_ASPECT.description` — typically `''`.
    pub description: String,
    /// `SHAPE_ASPECT.of_shape` resolved through
    /// `PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId`.
    pub target: ProductId,
    /// `SHAPE_ASPECT.product_definitional` — boolean enum (`.T.` / `.F.`),
    /// mostly `.F.` in observed NIST fixtures.
    pub product_definitional: bool,
}
