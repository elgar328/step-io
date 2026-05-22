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
//! and fixtures require them — `DATUM` and `DATUM_FEATURE` are covered; the
//! remaining subtypes (`DATUM_SYSTEM`, `DATUM_TARGET`, ...) are deferred.

use super::id::{
    CompositeShapeAspectId, ContinuousShapeAspectId, DatumFeatureId, DatumId, DerivedShapeAspectId,
    ShapeAspectId,
};

/// What a STEP `shape_aspect` reference resolved to in step-io's IR. Each
/// variant wraps the id of an existing typed shape-aspect arena entry.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}
