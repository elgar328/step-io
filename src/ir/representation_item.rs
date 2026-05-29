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
    CurveId, EdgeId, FaceId, GeometricRepresentationItemId, MappedItemId, Placement3dId, PointId,
    RepresentationId, RepresentationItemId, ShellId, SolidId, SurfaceId, TessellatedItemId,
    TypeQualifierId, ValueFormatTypeQualifierId, VertexId,
};

/// What a STEP `representation_item` reference resolved to in step-io's IR.
/// Each variant wraps the id of an existing typed arena entry. The geometry
/// and topology variants are carried over verbatim from the former
/// `StyledItemTarget`; `Representation`, `Placement3d`, and the
/// `RepresentationItem` enum arena extend the set.
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
    /// `representation_item` enum arena entry (phase repr-item-arena-1) —
    /// covers `qualified_representation_item` / `value_representation_item`
    /// variants. Future sub-phases extend to MRI / `NumericRepresentationItem`.
    RepresentationItem(RepresentationItemId),
    /// `geometric_representation_item` enum arena entry — covers SBSM and
    /// the symbol-domain variants (`DefinedSymbol`, `SymbolTarget`). Brought
    /// in so `STYLED_ITEM` can target a standalone SBSM (phase sbsm-repr-item).
    GeometricRepresentationItem(GeometricRepresentationItemId),
    /// `tessellated_item` arena entry — covers `TessellatedSolid`,
    /// `TessellatedShell`, `CoordinatesList`, and friends. `STYLED_ITEM` can
    /// target these directly (phase tessellation-repr-item).
    TessellatedItem(TessellatedItemId),
    /// `MAPPED_ITEM` arena entry (phase si-mapped-item) — `STYLED_ITEM` /
    /// `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM` can target a mapped
    /// instance directly (PMI annotation instance entry point).
    MappedItem(MappedItemId),
}

/// `representation_item` enum arena per the ir.toml blueprint (phase
/// repr-item-arena-1). Currently holds `QualifiedRepresentationItem` and
/// `ValueRepresentationItem`. MRI ([`crate::ir::property::PropertyMeasure`]
/// inline) migration is a
/// follow-up sub-phase.
#[derive(Debug, Clone, PartialEq)]
pub enum RepresentationItem {
    QualifiedRepresentationItem(QualifiedRepresentationItem),
    ValueRepresentationItem(ValueRepresentationItem),
}

/// `QUALIFIED_REPRESENTATION_ITEM(name, qualifiers)` — wraps a
/// `representation_item` with a SET of `value_qualifier` SELECT entries.
/// step-io's `ValueQualifier` enum covers the two corpus-modelled SELECT
/// members (`TypeQualifier` / `ValueFormatTypeQualifier`); other SELECT
/// members (`precision_qualifier` / `uncertainty_qualifier`) are corpus 0
/// and silently dropped on read (`ApprovalItem` precedent).
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRepresentationItem {
    pub name: String,
    pub qualifiers: Vec<QualifierRef>,
}

/// `value_qualifier` SELECT — duplicated here from
/// [`crate::ir::pmi::ValueQualifier`] because `QualifiedRepresentationItem`
/// lives in `shape_rep` while `MeasureQualification` lives in `pmi`. Same
/// 2-variant corpus subset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualifierRef {
    TypeQualifier(TypeQualifierId),
    ValueFormatTypeQualifier(ValueFormatTypeQualifierId),
}

/// `VALUE_REPRESENTATION_ITEM(name, value_component)` — wraps a
/// `representation_item` with a typed `measure_value` SELECT primitive.
/// The 40+ SELECT members (e.g. `POSITIVE_LENGTH_MEASURE`,
/// `COUNT_MEASURE`) all carry a primitive Real / Integer / Text payload,
/// so step-io models the SELECT as a 3-variant enum that preserves the
/// type-name verbatim — STEP round-trips reproduce
/// `POSITIVE_LENGTH_MEASURE(0.05)` literally.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueRepresentationItem {
    pub name: String,
    pub value_component: MeasureValue,
}

/// `measure_value` SELECT — the typed primitive payload of a
/// `VALUE_REPRESENTATION_ITEM`. `type_name` preserves the SELECT member
/// (e.g. `"POSITIVE_LENGTH_MEASURE"`) so round-trips reproduce the typed
/// STEP literal.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasureValue {
    Real { type_name: String, value: f64 },
    Integer { type_name: String, value: i64 },
    Text { type_name: String, value: String },
}
