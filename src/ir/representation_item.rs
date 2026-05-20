//! `RepresentationItemRef` — unified reference to a STEP `representation_item`.
//!
//! `representation_item` is the abstract supertype of virtually every
//! geometric entity in STEP. The ir.toml blueprint models it as a single
//! `enum_base` arena; step-io instead keeps its mature per-type arenas
//! (points, curves, surfaces, topology, solids, representations, ...) and
//! expresses the abstract supertype as this thin *reference enum* — a sum
//! over the existing typed arena id newtypes. A `representation_item`-typed
//! field (`STYLED_ITEM.item`, `MAPPED_ITEM.mapping_target`, ...) holds one
//! `RepresentationItemRef`. New variants are added additively as real
//! consumers and fixtures require them.

use super::id::{
    CurveId, EdgeId, FaceId, Placement3dId, PointId, RepresentationId, ShellId, SolidId, SurfaceId,
    VertexId,
};

/// What a STEP `representation_item` reference resolved to in step-io's IR.
/// Each variant wraps the id of an existing typed arena entry. The geometry
/// and topology variants are carried over verbatim from the former
/// `StyledItemTarget`; `Representation` and `Placement3d` extend the set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepresentationItemRef {
    Solid(SolidId),
    Face(FaceId),
    Edge(EdgeId),
    Curve(CurveId),
    Point(PointId),
    Surface(SurfaceId),
    Vertex(VertexId),
    Shell(ShellId),
    Representation(RepresentationId),
    Placement3d(Placement3dId),
}
