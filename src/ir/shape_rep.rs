//! Shape-representation IR types per the ir.toml blueprint.
//!
//! ir.toml's `shape_rep` pool covers entities whose arena resolves to
//! `representation`, `representation_context`, `shape_aspect`, or related
//! shape-bridge constructs. The handler files for these entities live in
//! `crate::entities::shape_rep`; this module holds the corresponding data
//! struct definitions.

use super::assembly::WireframeContent;
use super::id::StyledItemId;
use super::id::{
    GeneralDatumReferenceId, NamedUnitId, Placement3dId, ProductId, RepresentationId,
    RepresentationMapId, ShellId, SolidId, UnitContextId,
};
use super::representation_item::RepresentationItemRef;
use super::shape_aspect_ref::ShapeAspectRef;

/// Units declared in the STEP file's HEADER section.
///
/// The IR preserves original units — numeric values are **not** normalized.
/// Kernel adapters inspect `UnitContext` and convert if needed.
///
/// `length_uncertainty` is `Some` when the source file carried a
/// `UNCERTAINTY_MEASURE_WITH_UNIT` referenced through
/// `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT`. The numeric value is in the
/// source's length unit (mm / inch / ...) — no normalization. The
/// `name` / `description` strings are preserved verbatim so round-trip
/// reproduces the original metadata (writers no longer hardcode
/// `'distance_accuracy_value'` / `'confusion accuracy'`).
/// units-2: `length` / `plane_angle` / `solid_angle` are `NamedUnitId`
/// refs into `StepModel.units_pool.named_units`. Presentation flags
/// (`cbu_wrapped`, `dim_exp_explicit`) moved into the per-flavour
/// [`crate::ir::units::LengthFlavor`] / [`crate::ir::units::PlaneAngleFlavor`]
/// structs, so a single source `NAMED_UNIT` entity round-trips to a single
/// output entity (no dual-emit).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitContext {
    pub length: NamedUnitId,
    pub plane_angle: NamedUnitId,
    pub solid_angle: NamedUnitId,
    pub length_uncertainty: Option<LengthUncertainty>,
    /// Optional plane-angle uncertainty (e.g. `'angle_accuracy'` in some
    /// CAD exports). `None` when the source carried no angle-typed
    /// `UNCERTAINTY_MEASURE_WITH_UNIT`. Value is in the source's plane
    /// angle unit (radian / degree).
    pub plane_angle_uncertainty: Option<LengthUncertainty>,
    /// Optional solid-angle uncertainty. `None` for the typical case.
    pub solid_angle_uncertainty: Option<LengthUncertainty>,
}

/// `UNCERTAINTY_MEASURE_WITH_UNIT(value, unit_ref, name, description)`.
///
/// Carries the numeric uncertainty plus the metadata strings observed
/// in the source file. The two strings vary across CAD vendors — Fusion
/// 360 / `FreeCAD` emit `'distance_accuracy_value'` / `'confusion accuracy'`,
/// OCCT samples emit `'CONFUSED CURVE UNCERTAINTY'`, ABC-tier fixtures
/// emit empty strings. The reader preserves them verbatim and the
/// writer re-emits them as-is.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthUncertainty {
    pub value: f64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    Millimetre,
    Metre,
    Centimetre,
    Inch,
    Foot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AngleUnit {
    Radian,
    Degree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolidAngleUnit {
    Steradian,
}

/// `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION(name, items, context)`.
///
/// Top-level visualization wrapper. Lives in `shape_rep` per the ir.toml
/// blueprint (its arena is `representation`), even though the items it
/// wraps belong to the visualization domain — the container
/// [`crate::ir::VisualizationPool`] still owns the `Vec<Mdgpr>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mdgpr {
    pub name: String,
    pub items: Vec<StyledItemId>,
    /// Unit / uncertainty context referenced by this MDGPR. `Some(id)` indexes
    /// into [`crate::ir::model::StepModel::units`]. Fusion 360 typically uses
    /// a separate context here (different uncertainty than the geometry rep).
    /// `None` → writer emits `Attribute::Unset` for `context_of_items`
    /// (allowed by the spec for kernel-built IR with no context info).
    pub context: Option<UnitContextId>,
}

/// Unified `REPRESENTATION` arena entry — one variant per concrete
/// subtype. The representation-refactor (expand-migrate-contract) replaces
/// the legacy scattered storage (`absr_solid_map`, `mssr_shells_map`,
/// `ProductContent` geometry variants, …) with this single arena so that
/// `MAPPED_ITEM` / `REPRESENTATION_MAP` and other typed REPRESENTATION
/// references resolve uniformly. Phase A-1 dual-writes here while the
/// legacy maps still exist; later sub-phases migrate consumers.
#[derive(Debug, Clone, PartialEq)]
pub enum Representation {
    AdvancedBrep(AdvancedBrepRepr),
    ManifoldSurface(ManifoldSurfaceRepr),
    Plain(PlainRepr),
    /// Covers both `GBWSR` and `GBSSR` — the `content.repr_kind` field
    /// discriminates wireframe vs surface-bounded.
    Wireframe(WireframeRepr),
    Mdgpr(Mdgpr),
}

/// `ADVANCED_BREP_SHAPE_REPRESENTATION(name, items, context)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedBrepRepr {
    pub name: String,
    pub context: Option<UnitContextId>,
    pub ref_frame: Option<Placement3dId>,
    pub solids: Vec<SolidId>,
}

/// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION(name, items, context)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldSurfaceRepr {
    pub name: String,
    pub context: Option<UnitContextId>,
    pub ref_frame: Option<Placement3dId>,
    pub shells: Vec<ShellId>,
}

/// Plain `SHAPE_REPRESENTATION(name, items, context)` — geometry-free
/// wrapper carrying only the coordinate frame placement.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainRepr {
    pub name: String,
    pub context: Option<UnitContextId>,
    pub frame: Option<Placement3dId>,
}

/// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` /
/// `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION`.
#[derive(Debug, Clone, PartialEq)]
pub struct WireframeRepr {
    pub name: String,
    pub context: Option<UnitContextId>,
    pub ref_frame: Option<Placement3dId>,
    pub content: WireframeContent,
}

/// `REPRESENTATION_MAP(mapping_origin, mapped_representation)` — a reusable
/// mapping target that a [`MappedItem`] instantiates. ir.toml models it as a
/// `concrete_supertype` enum; the `camera_usage` subtype (18 instances) is
/// deferred, so only the `Itself` carrier is modelled for now.
#[derive(Debug, Clone, PartialEq)]
pub enum RepresentationMap {
    Itself(RepresentationMapData),
}

/// Carrier for the base `REPRESENTATION_MAP` entity.
#[derive(Debug, Clone, PartialEq)]
pub struct RepresentationMapData {
    /// The `representation_item` that consumers of this map place.
    pub mapping_origin: RepresentationItemRef,
    /// The representation this map points into.
    pub mapped_representation: RepresentationId,
}

/// `MAPPED_ITEM(name, mapping_source, mapping_target)` — an instance of a
/// [`RepresentationMap`] placed through a `representation_item` transform.
/// Itself a `representation_item` in STEP. ir.toml models it as a
/// `concrete_supertype` enum; the `camera_image` / `camera_image_3d_with_scale`
/// subtypes are deferred, so only the `Itself` carrier is modelled for now.
#[derive(Debug, Clone, PartialEq)]
pub enum MappedItem {
    Itself(MappedItemData),
}

/// Carrier for the base `MAPPED_ITEM` entity.
#[derive(Debug, Clone, PartialEq)]
pub struct MappedItemData {
    pub name: String,
    pub mapping_source: RepresentationMapId,
    pub mapping_target: RepresentationItemRef,
}

/// `representation_item` value-item — `INTEGER_REPRESENTATION_ITEM` or
/// `REAL_REPRESENTATION_ITEM`. Both are `SUBTYPE OF (representation_item,
/// literal_number)`; `name` comes from `representation_item`, `the_value`
/// from `literal_number`. step-io has no unified `representation_item`
/// arena, so these standalone value items get their own arena.
#[derive(Debug, Clone, PartialEq)]
pub enum NumericRepresentationItem {
    Integer(IntegerRepresentationItem),
    Real(RealRepresentationItem),
}

/// `INTEGER_REPRESENTATION_ITEM(name, the_value)`. Fixtures encode
/// `the_value` as a real literal (`8.`) even though the schema types it
/// `INTEGER`; the reader accepts both forms and stores a standard `i64`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerRepresentationItem {
    pub name: String,
    pub the_value: i64,
}

/// `REAL_REPRESENTATION_ITEM(name, the_value)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RealRepresentationItem {
    pub name: String,
    pub the_value: f64,
}

/// `SHAPE_ASPECT(name, description, of_shape, product_definitional)`.
///
/// `of_shape` is a `PRODUCT_DEFINITION_SHAPE` reference resolved to a
/// `ProductId` at read time via the existing `pdef_shape_to_pdef` and
/// `pdef_to_product` maps. SAs whose `of_shape` does not resolve are
/// silently dropped on read (symmetric ignorance preserves round-trip
/// equality for fixtures with non-standard targets).
///
/// Future PMI work (Tolerance / Datum / GD&T per ROADMAP Phase 2) adds
/// further structs alongside this one — all share the `shape_rep` pool
/// per the ir.toml blueprint.
/// `DESCRIPTIVE_REPRESENTATION_ITEM(name, description)` — free-form
/// textual property item.
///
/// Per the ir.toml blueprint (`descriptive_representation_item`,
/// `SingleStruct`, pool = `shape_rep`). Lives in `shape_rep` so that any
/// future consumer beyond [`crate::ir::property::PropertyItem`] can
/// reference it without crossing pool boundaries. Common usage:
/// `('Material', 'Steel')` label/value pairs inside a property
/// representation.
#[derive(Debug, Clone, PartialEq)]
pub struct DescriptiveItem {
    pub name: String,
    pub description: String,
}

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

/// `COMPOSITE_GROUP_SHAPE_ASPECT` — a `SHAPE_ASPECT` subtype. Same 4-attr
/// shape as [`ShapeAspect`]; the distinct STEP entity name (and so the
/// distinct arena per the ir.toml blueprint) is what round-trips.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeGroupShapeAspect {
    pub name: String,
    pub description: String,
    /// `of_shape` resolved to a `ProductId` — see [`ShapeAspect::target`].
    pub target: ProductId,
    pub product_definitional: bool,
}

/// `CENTRE_OF_SYMMETRY` — a `SHAPE_ASPECT` subtype. See
/// [`CompositeGroupShapeAspect`].
#[derive(Debug, Clone, PartialEq)]
pub struct CentreOfSymmetry {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
}

/// `ALL_AROUND_SHAPE_ASPECT` — a `SHAPE_ASPECT` subtype. See
/// [`CompositeGroupShapeAspect`].
#[derive(Debug, Clone, PartialEq)]
pub struct AllAroundShapeAspect {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
}

/// `DATUM_SYSTEM` — a `SHAPE_ASPECT` subtype carrying an ordered set of
/// datum references. The first four attrs are the shared `shape_aspect`
/// body (see [`CompositeGroupShapeAspect`]); `constituents` is the
/// `DATUM_REFERENCE_COMPARTMENT` list resolved to the
/// `general_datum_reference` arena. A `DATUM_SYSTEM` whose `of_shape` does
/// not resolve is silently dropped; individual `constituents` refs that do
/// not resolve are skipped — both symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DatumSystem {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
    /// `constituents` — `DATUM_REFERENCE_COMPARTMENT` refs in source order.
    pub constituents: Vec<GeneralDatumReferenceId>,
}

/// Which `SHAPE_ASPECT_RELATIONSHIP` flavour an entry round-trips as.
/// `shape_aspect_relationship` is a `concrete_supertype` — the plain entity
/// and its subtypes (`SHAPE_ASPECT_DERIVING_RELATIONSHIP` /
/// `SHAPE_ASPECT_ASSOCIATIVITY`) share the identical 4-attr shape and
/// differ only by STEP entity name. That name is captured here so one
/// arena covers the whole family; subtype variants are added additively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeAspectRelationshipKind {
    /// Plain `SHAPE_ASPECT_RELATIONSHIP`.
    Plain,
    /// `SHAPE_ASPECT_ASSOCIATIVITY`.
    Associativity,
    /// `SHAPE_ASPECT_DERIVING_RELATIONSHIP`.
    DerivingRelationship,
}

/// `SHAPE_ASPECT_RELATIONSHIP(name, description, relating_shape_aspect,
/// related_shape_aspect)` — a directed relation between two shape aspects.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeAspectRelationship {
    pub name: String,
    pub description: String,
    pub relating_shape_aspect: ShapeAspectRef,
    pub related_shape_aspect: ShapeAspectRef,
    pub kind: ShapeAspectRelationshipKind,
}
