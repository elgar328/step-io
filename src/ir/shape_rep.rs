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
    RepresentationMapId, ShellId, SolidId, ToleranceZoneFormId, UnitContextId,
};
use super::pmi::GeometricToleranceRef;
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
    pub context: Option<RepresentationContextRef>,
}

/// `(GEOMETRIC_REPRESENTATION_CONTEXT PARAMETRIC_REPRESENTATION_CONTEXT
/// REPRESENTATION_CONTEXT)` complex MI — unit-less coordinate space used
/// by 2D draughting models and pcurve parametric spaces. No
/// `GLOBAL_UNIT_ASSIGNED_CONTEXT` part, so length / `plane_angle` /
/// `solid_angle` are intentionally absent (the schema permits this).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitlessContext {
    pub identifier: String,
    pub context_type: String,
    pub coordinate_space_dimension: i64,
}

/// `context_of_items` SELECT discriminator — either a `UnitContext`
/// (unit-bearing) or a `UnitlessContext` (GRC+PRC complex MI).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepresentationContextRef {
    Unitful(UnitContextId),
    Unitless(crate::ir::id::UnitlessContextId),
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
    /// `SHAPE_DIMENSION_REPRESENTATION` (phase sdr-dcr). Unlike
    /// `PlainRepr` (which narrows `items` to a single placement frame),
    /// SDR preserves the full `items` SET — its blueprint members are
    /// `dimension_representation_item` SELECT entries (placement is not
    /// the typical case).
    ShapeDimensionRepresentation(ShapeDimensionRepresentation),
    /// `DRAUGHTING_MODEL` (phase draughting-model). Carries a `set_select`
    /// of `draughting_model_item_select` members (`STYLED_ITEM` /
    /// `ANNOTATION_PLANE` / `DRAUGHTING_CALLOUT` etc.) inside a
    /// representation wrapper. Emit is delayed (Mdgpr pattern) — skipped in
    /// the pre-pass and written after the visualization / annotation /
    /// callout passes so every `items` ref cache is populated. Complex-MI
    /// multi-inheritance forms (`(CHARACTERIZED_OBJECT ... DRAUGHTING_MODEL ...)`)
    /// are not modelled in this phase.
    DraughtingModel(DraughtingModel),
}

/// `GEOMETRIC_ITEM_SPECIFIC_USAGE(name, description, definition,
/// used_representation, identified_item)` — phase gisu. Sibling of
/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION` (both subtype
/// `item_identified_representation_usage`) but with shape-aspect-flavoured
/// `definition` and `representation_item`-flavoured `identified_item`
/// narrowing.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricItemSpecificUsage {
    pub name: String,
    pub description: Option<String>,
    pub definition: ShapeAspectRef,
    pub used_representation: RepresentationId,
    pub identified_item: crate::ir::representation_item::RepresentationItemRef,
}

/// `DRAUGHTING_MODEL(name, items, context_of_items)` — `representation`
/// subtype carrying a heterogeneous `set_select` of drafting items.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingModel {
    pub name: String,
    pub items: Vec<crate::ir::representation_item::RepresentationItemRef>,
    pub context: Option<RepresentationContextRef>,
}

/// `characterized_object` `concrete_supertype` enum (phase
/// characterized-object-ciwr). The blueprint shapes the supertype as a
/// "carrier" — the abstract body shared by every variant. The `Itself`
/// variant exists for future complex-MI sub-phases; this phase emits
/// only `CharacterizedItemWithinRepresentation` simple instances.
#[derive(Debug, Clone, PartialEq)]
pub enum CharacterizedObject {
    /// Carrier body for direct `CHARACTERIZED_OBJECT` instances. Corpus
    /// has 0 standalone instances (48 always appear as
    /// `(CHARACTERIZED_OBJECT REPRESENTATION ...)` complex MI). Reserved
    /// for a future complex-MI sub-phase.
    Itself(CharacterizedObjectData),
    /// `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` simple instance (513 of
    /// 561 corpus instances — the other 48 are complex MI parts and are
    /// dropped on read in this phase).
    CharacterizedItemWithinRepresentation(CharacterizedItemWithinRepresentation),
}

/// Carrier body shared by every [`CharacterizedObject`] variant.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterizedObjectData {
    pub name: String,
    pub description: Option<String>,
}

/// `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION(name, description, item, rep)`
/// — pairs a `representation_item` with the `representation` that
/// contains it. Either ref unresolved drops the occurrence, symmetric on
/// re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterizedItemWithinRepresentation {
    pub inherited: CharacterizedObjectData,
    pub item: RepresentationItemRef,
    pub rep: RepresentationId,
}

/// `SHAPE_DIMENSION_REPRESENTATION(name, items, context_of_items)` —
/// SUBTYPE OF `SHAPE_REPRESENTATION` SUBTYPE OF `REPRESENTATION`. `items`
/// preserved as a generic SET of [`RepresentationItemRef`]; unresolved
/// members skip silently.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDimensionRepresentation {
    pub name: String,
    pub context: Option<RepresentationContextRef>,
    pub items: Vec<RepresentationItemRef>,
}

/// `ADVANCED_BREP_SHAPE_REPRESENTATION(name, items, context)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedBrepRepr {
    pub name: String,
    pub context: Option<RepresentationContextRef>,
    pub ref_frame: Option<Placement3dId>,
    pub solids: Vec<SolidId>,
}

/// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION(name, items, context)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldSurfaceRepr {
    pub name: String,
    pub context: Option<RepresentationContextRef>,
    pub ref_frame: Option<Placement3dId>,
    pub shells: Vec<ShellId>,
}

/// Plain `SHAPE_REPRESENTATION(name, items, context)` — geometry-free
/// wrapper carrying only the coordinate frame placement.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainRepr {
    pub name: String,
    pub context: Option<RepresentationContextRef>,
    pub frame: Option<Placement3dId>,
}

/// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` /
/// `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION`.
#[derive(Debug, Clone, PartialEq)]
pub struct WireframeRepr {
    pub name: String,
    pub context: Option<RepresentationContextRef>,
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

/// `TOLERANCE_ZONE` — a `SHAPE_ASPECT` subtype naming a GD&T tolerance zone.
/// The first four attrs are the shared `shape_aspect` body (see
/// [`CompositeGroupShapeAspect`]); `defining_tolerance` is the set of
/// geometric tolerances that establish the zone and `form` references a
/// `TOLERANCE_ZONE_FORM`. A `TOLERANCE_ZONE` whose `of_shape` or `form` does
/// not resolve is silently dropped; individual `defining_tolerance` refs that
/// do not resolve are skipped — both symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceZone {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
    /// `defining_tolerance` — geometric tolerances in source order.
    pub defining_tolerance: Vec<GeometricToleranceRef>,
    /// `form` — the `TOLERANCE_ZONE_FORM` describing the zone's shape.
    pub form: ToleranceZoneFormId,
}

/// `DATUM_TARGET` — a `SHAPE_ASPECT` subtype identifying a single datum
/// target on a part. The first four attrs are the shared `shape_aspect`
/// body (see [`CompositeGroupShapeAspect`]); `target_id` is the alphabetic
/// label given to the target (e.g. `"A1"`, `"B2"`). A `DATUM_TARGET`
/// whose `of_shape` does not resolve is silently dropped, symmetric on
/// re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DatumTarget {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
    /// `target_id` — alphabetic label (`from = datum_target`).
    pub target_id: String,
}

/// `PLACED_DATUM_TARGET_FEATURE` — a `SHAPE_ASPECT` subtype tagging a
/// placed instance of a datum target on the feature it locates. Same
/// shape as [`DatumTarget`] (inherits the `target_id` field from
/// `datum_target`); ir.toml fields are identical.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedDatumTargetFeature {
    pub name: String,
    pub description: String,
    pub target: ProductId,
    pub product_definitional: bool,
    /// `target_id` — alphabetic label (`from = datum_target`).
    pub target_id: String,
}

/// Which `SHAPE_ASPECT_RELATIONSHIP` flavour an entry round-trips as.
/// `shape_aspect_relationship` is a `concrete_supertype` — the plain entity
/// and its subtypes (`SHAPE_ASPECT_DERIVING_RELATIONSHIP` /
/// `SHAPE_ASPECT_ASSOCIATIVITY` / `FEATURE_FOR_DATUM_TARGET_RELATIONSHIP`)
/// share the identical 4-attr shape and differ only by STEP entity name.
/// That name is captured here so one arena covers the whole family;
/// subtype variants are added additively.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeAspectRelationshipKind {
    /// Plain `SHAPE_ASPECT_RELATIONSHIP`.
    Plain,
    /// `SHAPE_ASPECT_ASSOCIATIVITY`.
    Associativity,
    /// `SHAPE_ASPECT_DERIVING_RELATIONSHIP`.
    DerivingRelationship,
    /// `FEATURE_FOR_DATUM_TARGET_RELATIONSHIP` — `related_shape_aspect` is
    /// narrowed to a `DATUM_TARGET` per the ir.toml blueprint
    /// (`ty = ref_datum_target`); a relationship whose `related_shape_aspect`
    /// resolves to anything else is dropped at read time and at write time.
    FeatureForDatumTarget,
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
