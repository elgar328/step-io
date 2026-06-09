//! `ShapeAspectRef` — unified reference to a STEP `shape_aspect`.
//!
//! `shape_aspect` is an abstract EXPRESS supertype. The ir.toml blueprint
//! models it as a single arena; step-io instead keeps per-subtype arenas
//! (plain `shape_aspect` plus `COMPOSITE_GROUP_SHAPE_ASPECT` /
//! `CENTRE_OF_SYMMETRY` / `ALL_AROUND_SHAPE_ASPECT`) and expresses the
//! abstract supertype as this thin *reference enum* — a sum over the
//! existing typed arena id newtypes. A `shape_aspect`-typed field
//! (`SHAPE_ASPECT_RELATIONSHIP.relating_shape_aspect`, a future
//! `GEOMETRIC_TOLERANCE.toleranced_shape_aspect`, ...) holds one
//! `ShapeAspectRef`. New variants are added additively as real consumers
//! and fixtures require them — `DATUM` / `DATUM_FEATURE` / `DATUM_SYSTEM` /
//! `DATUM_TARGET` / `PLACED_DATUM_TARGET_FEATURE` / `TOLERANCE_ZONE` /
//! `GENERAL_DATUM_REFERENCE` are covered; the remaining subtypes (e.g.
//! `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`) follow as their phases land.

use step_io_macros::StepSelect;

use super::id::{
    CompositeShapeAspectId, ContinuousShapeAspectId, DatumFeatureId, DatumId, DatumSystemId,
    DatumTargetId, DerivedShapeAspectId, GeneralDatumReferenceId, PlacedDatumTargetFeatureId,
    PropertyDefinitionId, ShapeAspectId, ToleranceZoneId,
};

/// What a STEP `shape_aspect` reference resolved to in step-io's IR. Each
/// variant wraps the id of an existing typed shape-aspect arena entry.
///
/// All variants are `Variant(XId)`, so `#[derive(StepSelect)]` generates the
/// reader `resolve_select` (probes each member id map in declaration order)
/// and writer `emit_select` (match on the variant) — the member list lives
/// only here, keeping read and write sides from drifting.
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum ShapeAspectRef {
    /// Plain `SHAPE_ASPECT`.
    ShapeAspect(ShapeAspectId),
    /// `COMPOSITE_GROUP_SHAPE_ASPECT`.
    CompositeGroupShapeAspect(CompositeShapeAspectId),
    /// `CENTRE_OF_SYMMETRY`.
    CentreOfSymmetry(DerivedShapeAspectId),
    /// `ALL_AROUND_SHAPE_ASPECT`.
    AllAroundShapeAspect(ContinuousShapeAspectId),
    /// `DATUM`.
    Datum(DatumId),
    /// `DATUM_FEATURE`.
    DatumFeature(DatumFeatureId),
    /// `DATUM_SYSTEM`.
    DatumSystem(DatumSystemId),
    /// `DATUM_TARGET`.
    DatumTarget(DatumTargetId),
    /// `PLACED_DATUM_TARGET_FEATURE`.
    PlacedDatumTargetFeature(PlacedDatumTargetFeatureId),
    /// `TOLERANCE_ZONE` — a `shape_aspect` subtype in its own arena.
    ToleranceZone(ToleranceZoneId),
    /// `GENERAL_DATUM_REFERENCE` — a `shape_aspect` subtype; the id covers both
    /// `DATUM_REFERENCE_COMPARTMENT` and `DATUM_REFERENCE_ELEMENT`.
    GeneralDatumReference(GeneralDatumReferenceId),
}

/// `geometric_tolerance_target` SELECT — the target of
/// `GEOMETRIC_TOLERANCE.toleranced_shape_aspect`. EXPRESS:
/// `SELECT(dimensional_location, dimensional_size, product_definition_shape,
/// shape_aspect)`. The blueprint collapses this to `ref_shape_aspect` because
/// the corpus is shape_aspect-dominated; step-io adds members additively as
/// fixtures require them. Only `shape_aspect` (the common case) and
/// `product_definition_shape` (NIST `ftc_07` / `ftc_10`) are observed; the two
/// dimensional members never appear as tolerance targets in the corpus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometricToleranceTarget {
    /// A `shape_aspect` subtype (the dominant case).
    ShapeAspect(ShapeAspectRef),
    /// `PRODUCT_DEFINITION_SHAPE`, materialized in the `property_definitions`
    /// arena as a `ProductDefinitionShape` variant.
    ProductDefinitionShape(PropertyDefinitionId),
}

impl From<ShapeAspectRef> for GeometricToleranceTarget {
    fn from(r: ShapeAspectRef) -> Self {
        GeometricToleranceTarget::ShapeAspect(r)
    }
}
