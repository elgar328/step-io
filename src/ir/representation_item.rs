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
    AnnotationCurveOccurrenceId, AnnotationOccurrenceId, CameraModelId, CompositeTextId, CurveId,
    DraughtingCalloutId, EdgeId, FaceId, GeometricRepresentationItemId, MappedItemId,
    Placement2dId, Placement3dId, PlanarExtentId, PointId, RepresentationId, RepresentationItemId,
    ShellId, SolidId, StyledItemId, SurfaceId, TessellatedFaceId, TessellatedItemId, TextLiteralId,
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
    /// `AXIS2_PLACEMENT_2D` (geometry `placements_2d` arena). A
    /// `representation_item` subtype; a `MAPPED_ITEM.mapping_target` or a
    /// `PRESENTATION_AREA`/`PRESENTATION_VIEW` item references it in the
    /// AP242 draughting-presentation corpus (phase presentation-cluster).
    Placement2d(Placement2dId),
    /// `PLANAR_BOX` / `PLANAR_EXTENT` (geometry `planar_extents` arena). A
    /// `representation_item` subtype listed in `PRESENTATION_AREA.items`; the
    /// writer's `emit_planar_extent` dispatches the inner variant.
    PlanarExtent(PlanarExtentId),
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
    /// `tessellated_face` arena entry (`COMPLEX_TRIANGULATED_FACE`). A
    /// `geometric_representation_item` subtype, so `STYLED_ITEM` styles it
    /// per-face directly (phase styled-item-tess-face).
    TessellatedFace(TessellatedFaceId),
    /// `MAPPED_ITEM` arena entry (phase si-mapped-item) — `STYLED_ITEM` /
    /// `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM` can target a mapped
    /// instance directly (PMI annotation instance entry point).
    MappedItem(MappedItemId),
    /// `annotation_occurrence` enum arena entry — covers
    /// `ANNOTATION_PLANE`, `TESSELLATED_ANNOTATION_OCCURRENCE`,
    /// `ANNOTATION_SYMBOL_OCCURRENCE`, `ANNOTATION_TEXT_OCCURRENCE`,
    /// `DRAUGHTING_ANNOTATION_OCCURRENCE`, `TERMINATOR_SYMBOL`,
    /// `LEADER_TERMINATOR`. AP242 MBD `DRAUGHTING_MODEL.items` routinely
    /// references these (phase rir-pmi-variants).
    AnnotationOccurrence(AnnotationOccurrenceId),
    /// `annotation_curve_occurrence` arena entry (plain ACO / `LEADER_CURVE`).
    /// A `styled_item` subtype; `STYLED_ITEM` / CIWR can target it (phase plain-aco).
    AnnotationCurveOccurrence(AnnotationCurveOccurrenceId),
    /// `DRAUGHTING_CALLOUT` arena entry — composite annotation grouping.
    /// Same AP242 MBD context as `AnnotationOccurrence`.
    DraughtingCallout(DraughtingCalloutId),
    /// `CAMERA_MODEL` enum arena entry (`CameraModelD3` / `D3WithHlhsr`
    /// / `D3MultiClipping`). Viewpoint definition routinely referenced by
    /// `DRAUGHTING_MODEL.items` in the AP242 MBD corpus.
    CameraModel(CameraModelId),
    /// `TEXT_LITERAL` (visualization pool). A styled `ANNOTATION_TEXT_OCCURRENCE`
    /// (and other `STYLED_ITEM` consumers) targets the text content directly.
    TextLiteral(TextLiteralId),
    /// `COMPOSITE_TEXT` (visualization pool) — a group of `TEXT_LITERAL`s, same
    /// `annotation_text_occurrence_item` SELECT role as `TextLiteral`.
    CompositeText(CompositeTextId),
    /// `STYLED_ITEM` (visualization pool). `STYLED_ITEM` is itself a
    /// `representation_item` subtype, so the AP242 MBD `DRAUGHTING_MODEL.items`
    /// SET routinely lists styled annotations directly (phase
    /// dm-styled-item). Without this the draughting model's items resolve to
    /// empty and the whole model is dropped.
    StyledItem(StyledItemId),
}

/// `representation_item` enum arena per the ir.toml blueprint (phase
/// repr-item-arena-1). Holds `QualifiedRepresentationItem`,
/// `ValueRepresentationItem`, and `MeasureRepresentationItem` (both the
/// simple and complex MI forms; all measure representations live in this
/// arena so property / GD&T / `SHAPE_DIMENSION_REPRESENTATION` consumers
/// resolve them here).
#[derive(Debug, Clone, PartialEq)]
pub enum RepresentationItem {
    QualifiedRepresentationItem(QualifiedRepresentationItem),
    ValueRepresentationItem(ValueRepresentationItem),
    MeasureRepresentationItem(MeasureRepresentationItem),
}

/// Whether a `MeasureRepresentationItem` serializes as the bare 3-attribute
/// `MEASURE_REPRESENTATION_ITEM(name, <typed>, #unit)` form (`Simple`) or the
/// complex-MI form (`Complex`). An explicit marker rather than inference: a
/// schema-legal complex MRI can carry neither a typed supertype nor a
/// qualifier, so `measure_supertype.is_none() && qualifiers.is_empty()` would
/// misclassify it (phase measure-arena-4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeasureForm {
    Simple,
    Complex,
}

/// A `MEASURE_REPRESENTATION_ITEM` — either the bare 3-attr form
/// `MEASURE_REPRESENTATION_ITEM(name, <typed>MEASURE(v), #unit)` (`Simple`) or
/// the complex-MI form `(<X>_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
/// MEASURE_WITH_UNIT(<typed value>, #unit) [QUALIFIED_REPRESENTATION_ITEM((..))]
/// REPRESENTATION_ITEM(name))` (`Complex`). `value` keeps the `measure_value`
/// type-name verbatim (e.g. `"POSITIVE_LENGTH_MEASURE"`) so the emit reproduces
/// the literal; `measure_supertype` records the typed `<X>_MEASURE_WITH_UNIT`
/// supertype part (complex only). `qualifiers` are empty for the simple form.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureRepresentationItem {
    pub form: MeasureForm,
    pub name: String,
    pub value: MeasureValue,
    pub unit_ref: Option<crate::ir::property::PropertyMeasureUnit>,
    pub qualifiers: Vec<QualifierRef>,
    pub measure_supertype: Option<String>,
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
