//! `pmi` pool — Product Manufacturing Information (GD&T) entities.
//!
//! Phase pmi-primitives seeds the pool with three dependency-free leaf
//! `single_struct` entities (tolerance / qualifier primitives). Future
//! phases add the tolerance / datum / geometric-tolerance entities that
//! consume them. `SHAPE_ASPECT` and its subtypes are *not* here — the
//! ir.toml blueprint puts them in the `shape_rep` pool.

use super::arena::Arena;
use super::id::{
    AnnotationCurveOccurrenceId, AnnotationOccurrenceId, CurveId, DatumId, DatumSystemId,
    DimensionalLocationId, DimensionalSizeId, DraughtingCalloutId, GeometricToleranceId,
    GeometricToleranceWithDatumReferenceId, LimitsAndFitsId, MeasureWithUnitId,
    PresentationStyleAssignmentId, ProductId, PropertyDefinitionId, RepresentationId,
    TessellatedItemId, ToleranceValueId, ToleranceZoneId, TypeQualifierId,
    ValueFormatTypeQualifierId,
};
use super::representation_item::RepresentationItemRef;
use super::shape_aspect_ref::ShapeAspectRef;

/// Top-level container for `pmi`-pool entities. `None` when the source
/// file carried no PMI content.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PmiPool {
    /// `TOLERANCE_ZONE_FORM` arena.
    pub tolerance_zone_forms: Arena<ToleranceZoneForm>,
    /// `TYPE_QUALIFIER` arena.
    pub type_qualifiers: Arena<TypeQualifier>,
    /// `VALUE_FORMAT_TYPE_QUALIFIER` arena.
    pub value_format_type_qualifiers: Arena<ValueFormatTypeQualifier>,
    /// `annotation_occurrence` `enum_base` arena. Phase annotation-plane
    /// fills the `AnnotationPlane` variant only.
    pub annotation_occurrences: Arena<AnnotationOccurrence>,
    /// `annotation_curve_occurrence` arena (phase annotation-curve-leader,
    /// `LeaderCurve` subtype) extended (phase plain-aco) with the plain
    /// supertype occurrence.
    pub annotation_curve_occurrences: Arena<AnnotationCurveOccurrence>,
    /// `draughting_callout` `complex_supertype` arena (phase
    /// draughting-callout).
    pub draughting_callouts: Arena<DraughtingCallout>,
    /// `draughting_callout_relationship` arena (phase draughting-callout).
    pub draughting_callout_relationships: Arena<DraughtingCalloutRelationship>,
    /// `annotation_occurrence_associativity` arena (phase aoa). Pairs two
    /// annotation occurrences.
    pub annotation_occurrence_associativities: Arena<AnnotationOccurrenceAssociativity>,
    /// `geometric_tolerance_relationship` arena (phase gt-relationship).
    pub geometric_tolerance_relationships: Arena<GeometricToleranceRelationship>,
    /// `tolerance_zone_definition` arena — currently holds
    /// `ProjectedZoneDefinition` only (phase projected-zone).
    pub tolerance_zone_definitions: Arena<ProjectedZoneDefinition>,
    /// `measure_qualification` arena (phase measure-qualification).
    pub measure_qualifications: Arena<MeasureQualification>,
    /// `DATUM` arena. Phase datum.
    pub datums: Arena<Datum>,
    /// `DATUM_FEATURE` arena. Phase datum-feature.
    pub datum_features: Arena<DatumFeature>,
    /// `DRAUGHTING_PRE_DEFINED_TEXT_FONT` arena. Phase text-font.
    pub draughting_pre_defined_text_fonts: Arena<DraughtingPreDefinedTextFont>,
    /// `DIMENSIONAL_SIZE` arena. Phase dimensional-size.
    pub dimensional_sizes: Arena<DimensionalSize>,
    /// `dimensional_location` `enum_base` arena. Phase dimensional-location.
    pub dimensional_locations: Arena<DimensionalLocation>,
    /// `geometric_tolerance` `enum_base` arena. Phase geometric-tolerance
    /// fills the datum-free form-tolerance variants.
    pub geometric_tolerances: Arena<GeometricTolerance>,
    /// `general_datum_reference` `enum_base` arena. Phase
    /// general-datum-reference fills both variants.
    pub general_datum_references: Arena<GeneralDatumReference>,
    /// `geometric_tolerance_with_datum_reference` `enum_base` arena. Phase
    /// gt-datum-ref fills the seven simple-form datum-referencing tolerances.
    pub geometric_tolerance_with_datum_references: Arena<GeometricToleranceWithDatumReference>,
    /// `TOLERANCE_VALUE` arena. Phase plus-minus-tolerance.
    pub tolerance_values: Arena<ToleranceValue>,
    /// `LIMITS_AND_FITS` arena. Phase plus-minus-tolerance.
    pub limits_and_fits: Arena<LimitsAndFits>,
    /// `PLUS_MINUS_TOLERANCE` arena. Phase plus-minus-tolerance.
    pub plus_minus_tolerances: Arena<PlusMinusTolerance>,
    /// `DRAUGHTING_MODEL_ITEM_ASSOCIATION` arena (phase dmia). Binds an
    /// annotation occurrence / callout to a draughting model representation
    /// via the `item_identified_representation_usage` supertype attributes.
    pub draughting_model_item_associations: Arena<DraughtingModelItemAssociation>,
}

/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION(name, description, definition,
/// used_representation, identified_item)` — phase dmia. Subtype of
/// `item_identified_representation_usage` (abstract — not modelled).
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingModelItemAssociation {
    pub name: String,
    pub description: Option<String>,
    pub definition: DraughtingModelItemDefinition,
    /// `used_representation` — narrowed by ir.toml to `ref_draughting_model`;
    /// step-io stores the unified `RepresentationId`.
    pub used_representation: RepresentationId,
    pub identified_item: DraughtingModelIdentifiedItem,
}

/// `draughting_model_item_definition` SELECT — step-io models the six
/// members below. SELECT entries pointing to other members (assignment /
/// classification / approval items) are dropped on read with a warning
/// (`feedback_partial_select_enum`, `feedback_no_silent_skip`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DraughtingModelItemDefinition {
    Representation(RepresentationId),
    DimensionalSize(DimensionalSizeId),
    /// `shape_aspect` member — the abstract supertype, resolved to whichever
    /// concrete subtype it is via the shared [`ShapeAspectRef`].
    ShapeAspect(ShapeAspectRef),
    PropertyDefinition(PropertyDefinitionId),
    DimensionalLocation(DimensionalLocationId),
    /// `geometric_tolerance` member — `Plain` or `WithDatumReference` (the
    /// complex MI form), via [`GeometricToleranceRef`].
    GeometricTolerance(GeometricToleranceRef),
}

/// `draughting_model_item_association_select` SELECT — both members are
/// modelled in step-io.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DraughtingModelIdentifiedItem {
    AnnotationOccurrence(AnnotationOccurrenceId),
    DraughtingCallout(DraughtingCalloutId),
}

/// `TOLERANCE_VALUE(lower_bound, upper_bound)` — the value range of a
/// `PLUS_MINUS_TOLERANCE`. Both bounds are `ref_measure_with_unit`, the same
/// polymorphic reference as `geometric_tolerance.magnitude`, so each is a
/// [`ToleranceMagnitude`]. A `TOLERANCE_VALUE` whose either bound does not
/// resolve is silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceValue {
    pub lower_bound: ToleranceMagnitude,
    pub upper_bound: ToleranceMagnitude,
}

/// `LIMITS_AND_FITS(form_variance, zone_variance, grade, source)` — an
/// ISO 286 limits-and-fits designation, the other `tolerance_method_definition`
/// SELECT member besides [`ToleranceValue`].
#[derive(Debug, Clone, PartialEq)]
pub struct LimitsAndFits {
    pub form_variance: String,
    pub zone_variance: String,
    pub grade: String,
    pub source: String,
}

/// `tolerance_method_definition` SELECT — the `range` of a
/// [`PlusMinusTolerance`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToleranceMethodDefinition {
    /// `TOLERANCE_VALUE`.
    Value(ToleranceValueId),
    /// `LIMITS_AND_FITS`.
    LimitsAndFits(LimitsAndFitsId),
}

/// `dimensional_characteristic` SELECT — the `toleranced_dimension` of a
/// [`PlusMinusTolerance`]: either a `DIMENSIONAL_LOCATION` or a
/// `DIMENSIONAL_SIZE` (each id covers all of that entity's variants).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimensionalCharacteristic {
    /// A `dimensional_location` family entry.
    Location(DimensionalLocationId),
    /// A `dimensional_size` family entry.
    Size(DimensionalSizeId),
}

/// `PLUS_MINUS_TOLERANCE(range, toleranced_dimension)` — a ± value tolerance
/// applied to a dimensional characteristic. A `PLUS_MINUS_TOLERANCE` whose
/// `range` or `toleranced_dimension` does not resolve is silently dropped,
/// symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct PlusMinusTolerance {
    pub range: ToleranceMethodDefinition,
    pub toleranced_dimension: DimensionalCharacteristic,
}

/// `geometric_tolerance_with_datum_reference` `enum_base` — a GD&T tolerance
/// that references one or more datum systems. The seven concrete subtypes
/// (`ANGULARITY` / `CIRCULAR_RUNOUT` / `CONCENTRICITY` / `PARALLELISM` /
/// `PERPENDICULARITY` / `SYMMETRY` / `TOTAL_RUNOUT`) share the identical
/// 5-attr body, so one [`GeometricToleranceWithDatumReferenceData`] payload
/// serves every variant. `POSITION` / `SURFACE_PROFILE` / `LINE_PROFILE`
/// datum-referencing tolerances take a different (multiple-inheritance
/// complex) encoding and arrive in a later phase.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricToleranceWithDatumReference {
    /// `ANGULARITY_TOLERANCE`.
    Angularity(GeometricToleranceWithDatumReferenceData),
    /// `CIRCULAR_RUNOUT_TOLERANCE`.
    CircularRunout(GeometricToleranceWithDatumReferenceData),
    /// `CONCENTRICITY_TOLERANCE`.
    Concentricity(GeometricToleranceWithDatumReferenceData),
    /// `PARALLELISM_TOLERANCE`.
    Parallelism(GeometricToleranceWithDatumReferenceData),
    /// `PERPENDICULARITY_TOLERANCE`.
    Perpendicularity(GeometricToleranceWithDatumReferenceData),
    /// `SYMMETRY_TOLERANCE`.
    Symmetry(GeometricToleranceWithDatumReferenceData),
    /// `TOTAL_RUNOUT_TOLERANCE`.
    TotalRunout(GeometricToleranceWithDatumReferenceData),
    /// `POSITION_TOLERANCE` — read/written as the multiple-inheritance
    /// complex `(GEOMETRIC_TOLERANCE GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE
    /// POSITION_TOLERANCE)`.
    Position(GeometricToleranceWithDatumReferenceData),
    /// `SURFACE_PROFILE_TOLERANCE` — complex MI form, see [`Self::Position`].
    SurfaceProfile(GeometricToleranceWithDatumReferenceData),
    /// `LINE_PROFILE_TOLERANCE` — complex MI form, see [`Self::Position`].
    LineProfile(GeometricToleranceWithDatumReferenceData),
}

/// A reference to any concrete `geometric_tolerance` — used by
/// `TOLERANCE_ZONE.defining_tolerance`. `geometric_tolerance` is an abstract
/// supertype; step-io splits its concrete subtypes across two arenas (the
/// datum-free form tolerances and the datum-referencing ones), so a
/// `ref_geometric_tolerance` resolves into one of those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeometricToleranceRef {
    /// An entry in the `geometric_tolerances` arena (form tolerance).
    Plain(GeometricToleranceId),
    /// An entry in the `geometric_tolerance_with_datum_references` arena.
    WithDatumReference(GeometricToleranceWithDatumReferenceId),
}

/// Shared 5-attr body of the datum-referencing tolerances — the four
/// `geometric_tolerance` attrs plus the `datum_system` set. A tolerance
/// whose `magnitude` or `toleranced_shape_aspect` does not resolve is
/// silently dropped; individual `datum_system` refs that do not resolve are
/// skipped — both symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricToleranceWithDatumReferenceData {
    pub name: String,
    pub description: String,
    /// `magnitude` — a `ref_measure_with_unit`.
    pub magnitude: ToleranceMagnitude,
    /// `toleranced_shape_aspect` — a `ref_shape_aspect`.
    pub toleranced_shape_aspect: ShapeAspectRef,
    /// `datum_system` — `DATUM_SYSTEM` refs in source order.
    pub datum_system: Vec<DatumSystemId>,
    /// `GEOMETRIC_TOLERANCE_WITH_MODIFIERS.modifiers` part — empty when
    /// the source instance carries no modifier part; non-empty values
    /// trigger 4-part complex MI emit (GT + WDR + WM + LEAF).
    pub modifiers: Vec<GeometricToleranceModifier>,
    /// `UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE.displacement` part — when
    /// `Some`, the writer emits an additional `UD` part between WM and
    /// the leaf. Corpus pairs UD with `SURFACE_PROFILE_TOLERANCE` only.
    pub displacement: Option<MeasureWithUnitId>,
}

/// `general_datum_reference` `enum_base` — a GD&T datum reference in the
/// `pmi` pool. `general_datum_reference` is an abstract supertype; its two
/// concrete subtypes `DATUM_REFERENCE_COMPARTMENT` and
/// `DATUM_REFERENCE_ELEMENT` share the identical 6-attr body, so one
/// [`GeneralDatumReferenceData`] payload serves both.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneralDatumReference {
    /// `DATUM_REFERENCE_COMPARTMENT`.
    Compartment(GeneralDatumReferenceData),
    /// `DATUM_REFERENCE_ELEMENT`.
    Element(GeneralDatumReferenceData),
}

/// Shared 6-attr body of `general_datum_reference`. The first four attrs are
/// the inherited `shape_aspect` body (`of_shape` resolved to the owning
/// product); `base` is the `datum_or_common_datum` SELECT. The 6th attr
/// `modifiers` (an optional `datum_reference_modifier` set) is not modelled —
/// the reader ignores it and the writer emits `$`, so a source instance
/// carrying modifiers loses them (a textual loss, not a round-trip break).
/// A `general_datum_reference` whose `of_shape` or `base` does not resolve
/// is silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralDatumReferenceData {
    pub name: String,
    pub description: String,
    /// `of_shape` resolved to the owning product.
    pub target: ProductId,
    pub product_definitional: bool,
    /// `base` — the `datum_or_common_datum` SELECT.
    pub base: GeneralDatumBase,
}

/// `datum_or_common_datum` SELECT target for [`GeneralDatumReferenceData::base`]
/// (the blueprint models `base` as `ty = "select"`). Both SELECT members are
/// modelled. A `base` outside these forms drops the owning
/// `general_datum_reference` at read time — same expansion policy as
/// [`ShapeAspectRef`](crate::ir::ShapeAspectRef).
#[derive(Debug, Clone, PartialEq)]
pub enum GeneralDatumBase {
    /// `DATUM`.
    Datum(DatumId),
    /// `COMMON_DATUM_LIST` — `LIST [2:?] OF datum_reference_element` (an "A-B"
    /// composite datum). Each id refers to a
    /// [`GeneralDatumReference::Element`] in the same arena.
    CommonDatumList(Vec<crate::ir::id::GeneralDatumReferenceId>),
}

/// `geometric_tolerance` `enum_base` — a GD&T tolerance applied to a shape
/// aspect. This phase covers the four datum-free *form* tolerances
/// (`FLATNESS` / `STRAIGHTNESS` / `ROUNDNESS` / `CYLINDRICITY`); they share
/// the identical 4-attr body, so one [`GeometricToleranceData`] payload
/// serves every variant. Datum-referencing tolerances arrive in a later
/// phase as additional variants.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricTolerance {
    /// `FLATNESS_TOLERANCE`.
    Flatness(GeometricToleranceData),
    /// `STRAIGHTNESS_TOLERANCE`.
    Straightness(GeometricToleranceData),
    /// `ROUNDNESS_TOLERANCE`.
    Roundness(GeometricToleranceData),
    /// `CYLINDRICITY_TOLERANCE`.
    Cylindricity(GeometricToleranceData),
}

/// Shared 4-attr body of the form tolerances — inherited from
/// `geometric_tolerance`. A tolerance whose `magnitude` or
/// `toleranced_shape_aspect` does not resolve is silently dropped, symmetric
/// on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricToleranceData {
    pub name: String,
    pub description: String,
    /// `magnitude` — a `ref_measure_with_unit`.
    pub magnitude: ToleranceMagnitude,
    /// `toleranced_shape_aspect` — a `ref_shape_aspect`.
    pub toleranced_shape_aspect: ShapeAspectRef,
    /// `GEOMETRIC_TOLERANCE_WITH_MODIFIERS.modifiers` part — empty when
    /// the source instance is a plain form tolerance; non-empty values
    /// trigger 3-part complex MI emit (GT + WM + LEAF).
    pub modifiers: Vec<GeometricToleranceModifier>,
    /// `GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT.unit_size` part — when
    /// `Some`, the writer emits the WDU part after GT. Corpus pairs WDU
    /// with `FLATNESS` / `STRAIGHTNESS` form tolerances.
    pub unit_size: Option<MeasureWithUnitId>,
    /// `GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT` part — cascades from
    /// WDU per EXPRESS schema (always `Some` only when `unit_size` is
    /// also `Some`).
    pub defined_area_unit: Option<DefinedAreaUnit>,
}

/// `GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT` body — `area_type` plus an
/// optional `second_unit_size` whose presence is constrained by the
/// EXPRESS WHERE clause `EXISTS(second_unit_size) XOR (area_type =
/// rectangular)` (rectangular requires it, others omit it).
#[derive(Debug, Clone, PartialEq)]
pub struct DefinedAreaUnit {
    pub area_type: AreaUnitType,
    /// `second_unit_size` — `length_measure_with_unit`. `Some` only when
    /// `area_type == Rectangular` per the schema's WHERE clause.
    pub second_unit_size: Option<MeasureWithUnitId>,
}

/// AP242 `area_unit_type` ENUMERATION — `circular`, `rectangular`,
/// `square`. Unknown tokens land in `Other(raw)` for verbatim round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AreaUnitType {
    Circular,
    Rectangular,
    Square,
    Other(String),
}

/// Subset of AP242's `limit_condition` (and adjacent modifier enums)
/// observed in corpus `GEOMETRIC_TOLERANCE_WITH_MODIFIERS.modifiers`
/// SETs. Unknown raw enum tokens land in [`Other`] verbatim so round-trip
/// is lossless even before a fixture forces a named variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeometricToleranceModifier {
    MaximumMaterialRequirement,
    LeastMaterialRequirement,
    ReciprocityRequirement,
    /// Verbatim source token (uppercase, no leading/trailing `.`) for any
    /// modifier value step-io does not yet name.
    Other(String),
}

/// Resolved target of a `geometric_tolerance.magnitude` (`ref_measure_with_unit`).
/// The reference is polymorphic in real files: a plain `*_MEASURE_WITH_UNIT`
/// lives in the `units` pool and is emitted once there ([`MeasureWithUnit`]
/// variant — the writer references its cached step id); a
/// `MEASURE_REPRESENTATION_ITEM` (simple or complex) lives in the
/// `representation_item` arena ([`RepresentationItem`] variant). Keeping the
/// two cases distinct avoids double-emitting a units-pool `MEASURE_WITH_UNIT`.
#[derive(Debug, Clone, PartialEq)]
pub enum ToleranceMagnitude {
    /// A plain `*_MEASURE_WITH_UNIT` already held in the `units` pool.
    MeasureWithUnit(MeasureWithUnitId),
    /// A `MEASURE_REPRESENTATION_ITEM` referenced in the
    /// `representation_item` arena (phase measure-arena-2) — the tolerance
    /// points at the faithful multi-part form rather than re-emitting a
    /// downgraded simple measure.
    RepresentationItem(crate::ir::id::RepresentationItemId),
}

/// `dimensional_location` `enum_base` — a located dimension between two
/// shape aspects. Plain `DIMENSIONAL_LOCATION` and the
/// `DIRECTED_DIMENSIONAL_LOCATION` subtype share the identical 4-attr body;
/// the `ANGULAR_LOCATION` subtype adds an `angle_selection` attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum DimensionalLocation {
    /// Plain `DIMENSIONAL_LOCATION`.
    Plain(DimensionalLocationData),
    /// `DIRECTED_DIMENSIONAL_LOCATION`.
    Directed(DimensionalLocationData),
    /// `ANGULAR_LOCATION`.
    Angular(AngularLocationData),
}

/// Shared 4-attr body of `DIMENSIONAL_LOCATION` / `DIRECTED_DIMENSIONAL_LOCATION`
/// — inherited from `shape_aspect_relationship`. A location whose either
/// endpoint does not resolve is silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionalLocationData {
    pub name: String,
    pub description: String,
    pub relating_shape_aspect: ShapeAspectRef,
    pub related_shape_aspect: ShapeAspectRef,
}

/// `ANGULAR_LOCATION` body — [`DimensionalLocationData`] plus the
/// `angle_selection` attribute the subtype adds.
#[derive(Debug, Clone, PartialEq)]
pub struct AngularLocationData {
    pub name: String,
    pub description: String,
    pub relating_shape_aspect: ShapeAspectRef,
    pub related_shape_aspect: ShapeAspectRef,
    pub angle_selection: AngleSelection,
}

/// Which `DIMENSIONAL_SIZE` flavour an entry round-trips as.
/// EXPRESS `angle_relator` — selects which angle an `ANGULAR_SIZE` /
/// `ANGULAR_LOCATION` denotes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AngleSelection {
    Equal,
    Large,
    Small,
}

/// Which `DIMENSIONAL_SIZE` flavour an entry round-trips as.
/// `dimensional_size` is a `concrete_supertype`; the `ANGULAR_SIZE` subtype
/// adds an `angle_selection` attribute, carried by the `Angular` variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimensionalSizeKind {
    /// Plain `DIMENSIONAL_SIZE`.
    Plain,
    /// `ANGULAR_SIZE` — carries the `angle_selection`.
    Angular(AngleSelection),
}

/// `DIMENSIONAL_SIZE(applies_to, name)` — names a size dimension applied to
/// a shape aspect. `applies_to` is a `shape_aspect`-typed reference; a
/// `DIMENSIONAL_SIZE` whose `applies_to` does not resolve is silently
/// dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionalSize {
    /// `applies_to` — a `ref_shape_aspect`.
    pub applies_to: ShapeAspectRef,
    pub name: String,
    pub kind: DimensionalSizeKind,
}

/// `DRAUGHTING_PRE_DEFINED_TEXT_FONT(name)` — a stock text-font reference
/// (a `pre_defined_item`). Common names: `'standard'`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingPreDefinedTextFont {
    pub name: String,
}

/// `DATUM(name, description, of_shape, product_definitional, identification)`
/// — a `shape_aspect` subtype naming a datum feature for GD&T. `of_shape`
/// resolves through `PRODUCT_DEFINITION_SHAPE` to a [`ProductId`], exactly
/// as `SHAPE_ASPECT` does. A `DATUM` whose `of_shape` does not resolve is
/// silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct Datum {
    pub name: String,
    pub description: String,
    /// `of_shape` resolved to the owning product.
    pub target: ProductId,
    pub product_definitional: bool,
    pub identification: String,
}

/// `DATUM_FEATURE(name, description, of_shape, product_definitional)` — a
/// `shape_aspect` subtype naming a physical feature that realises a datum.
/// Same 4-attr `shape_aspect` body as [`ShapeAspect`](crate::ir::ShapeAspect);
/// `of_shape` resolves through `PRODUCT_DEFINITION_SHAPE` to a [`ProductId`].
/// A `DATUM_FEATURE` whose `of_shape` does not resolve is silently dropped,
/// symmetric on re-read.
///
/// The ir.toml blueprint classifies the `datum_feature` family as flat
/// `in_enum` members under `shape_aspect`; step-io keeps a single arena and
/// tags the entry with the [`DatumFeature`] variant so consumers reach it
/// through the unified
/// [`ShapeAspectRef`](crate::ir::ShapeAspectRef)`::DatumFeature` variant.
#[derive(Debug, Clone, PartialEq)]
pub struct DatumFeatureData {
    pub name: String,
    pub description: String,
    /// `of_shape` resolved to the owning product.
    pub target: ProductId,
    pub product_definitional: bool,
}

/// A datum feature, tagged with the source STEP entity. AP242 declares
/// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` as the only subtype and it adds no
/// own attributes over the 4-attr `shape_aspect` body, so the variant alone
/// round-trips the source entity name.
#[derive(Debug, Clone, PartialEq)]
pub enum DatumFeature {
    /// Plain `DATUM_FEATURE`.
    Itself(DatumFeatureData),
    /// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`.
    DimensionalSizeWithDatumFeature(DatumFeatureData),
}

impl DatumFeature {
    /// The shared `shape_aspect`-body payload, regardless of source entity.
    #[must_use]
    pub fn data(&self) -> &DatumFeatureData {
        match self {
            DatumFeature::Itself(d) | DatumFeature::DimensionalSizeWithDatumFeature(d) => d,
        }
    }
}

/// `annotation_occurrence` `enum_base` — STEP `styled_item` PMI subtypes that
/// carry presentation annotations.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationOccurrence {
    /// Plain `ANNOTATION_OCCURRENCE` (the non-abstract supertype) — same
    /// `(name, styles, item)` shape as the subtypes below.
    Plain(PlainAnnotationOccurrence),
    AnnotationPlane(AnnotationPlane),
    TessellatedAnnotationOccurrence(TessellatedAnnotationOccurrence),
    AnnotationSymbolOccurrence(AnnotationSymbolOccurrence),
    AnnotationTextOccurrence(AnnotationTextOccurrence),
    DraughtingAnnotationOccurrence(DraughtingAnnotationOccurrence),
    TerminatorSymbol(TerminatorSymbol),
    LeaderTerminator(LeaderTerminator),
}

/// `ANNOTATION_OCCURRENCE(name, styles, item)` — the plain `styled_item`
/// supertype, instantiated directly in some PMI corpora (e.g. as a
/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION.identified_item`). `item` is the
/// inherited `styled_item` SELECT, carried as [`RepresentationItemRef`];
/// an unresolved `item` drops the occurrence, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainAnnotationOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `LEADER_CURVE(name, styles, item)` — sole occupant of the
/// `annotation_curve_occurrence` arena entry. EXPRESS models
/// `annotation_curve_occurrence` as the (instantiable) supertype of
/// `dimension_curve` / `leader_curve` / `projection_curve`. step-io captures
/// the plain supertype occurrence and the `leader_curve` subtype (the two
/// forms seen in the corpus); the `item` narrowing differs between them.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationCurveOccurrence {
    /// Plain `ANNOTATION_CURVE_OCCURRENCE` — `item` is the supertype
    /// `curve_or_curve_set` SELECT, carried as `RepresentationItemRef`.
    Plain(PlainAnnotationCurveOccurrence),
    /// `LEADER_CURVE` subtype — `item` narrowed to a `Curve`.
    LeaderCurve(LeaderCurve),
}

/// Plain `ANNOTATION_CURVE_OCCURRENCE(name, styles, item)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainAnnotationCurveOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    /// `styled_item.item` — `curve_or_curve_set` (a `Curve` or e.g.
    /// `GEOMETRIC_CURVE_SET`), carried as `RepresentationItemRef`.
    pub item: RepresentationItemRef,
}

/// `LEADER_CURVE` — `annotation_curve_occurrence` subtype whose `item` is
/// narrowed to `ref_curve` (a `Curve` arena id).
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderCurve {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    /// `styled_item.item` narrowed (per `annotation_curve_occurrence`) to
    /// `ref_curve` — a `Curve` arena id.
    pub item: CurveId,
}

/// `TERMINATOR_SYMBOL(name, styles, item, annotated_curve)` — an
/// `annotation_symbol_occurrence` subtype whose `annotated_curve` points
/// at an `annotation_curve_occurrence` entry. `item` is the inherited
/// `styled_item` SELECT, carried as `RepresentationItemRef`. Unresolved
/// `item` or `annotated_curve` drops the occurrence, symmetric on
/// re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminatorSymbol {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
    pub annotated_curve: AnnotationCurveOccurrenceId,
}

/// `LEADER_TERMINATOR(name, styles, item, annotated_curve)` — a
/// `terminator_symbol` subtype. WHERE narrows `annotated_curve` to a
/// `LEADER_CURVE` instance; the IR carries the same shape as
/// `TerminatorSymbol` (the WHERE narrowing is not encoded in the IR
/// type — fixture data is assumed to satisfy it).
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderTerminator {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
    pub annotated_curve: AnnotationCurveOccurrenceId,
}

/// `ANNOTATION_PLANE(name, styles, item, elements)` — a `styled_item`
/// subtype locating the plane that PMI annotations are drawn on. `elements`
/// (a list of `DRAUGHTING_CALLOUT`) is dropped — that entity is not
/// modelled; the field is optional so the writer emits `$`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationPlane {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `TESSELLATED_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is a tessellated geometric
/// set. A `TESSELLATED_ANNOTATION_OCCURRENCE` whose `item` does not resolve
/// is silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedAnnotationOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    /// `ref_tessellated_geometric_set` — resolved to the `tessellated_item`
    /// arena.
    pub item: TessellatedItemId,
}

/// `ANNOTATION_SYMBOL_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is the
/// `annotation_symbol_occurrence_item` SELECT (`annotation_symbol` |
/// `defined_symbol`). step-io does not model those select members as
/// dedicated arenas yet; `item` resolves through the generic
/// `RepresentationItemRef` fallback (`resolve_representation_item_ref`),
/// and an occurrence whose `item` does not resolve is silently dropped,
/// symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationSymbolOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `ANNOTATION_TEXT_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is the
/// `annotation_text_occurrence_item` SELECT (`text_literal` |
/// `annotation_text` | `annotation_text_character` |
/// `defined_character_glyph` | `composite_text`).
/// Same fallback / drop policy as `AnnotationSymbolOccurrence`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationTextOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `DRAUGHTING_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is narrowed (via WHERE
/// constraints) to `ref_representation_item`. Carried as
/// `RepresentationItemRef` directly. Unresolved items are silently
/// dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingAnnotationOccurrence {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `value_qualifier` SELECT — step-io models only the two SELECT members
/// that appear in the NIST 53,000-fixture corpus
/// (`type_qualifier` and `value_format_type_qualifier`). The other two —
/// `precision_qualifier` / `uncertainty_qualifier` — have corpus count 0
/// and are intentionally pruned from `ir.toml`; references to them are
/// silently dropped on read, mirroring the [`ApprovalItem`] precedent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueQualifier {
    TypeQualifier(TypeQualifierId),
    ValueFormatTypeQualifier(ValueFormatTypeQualifierId),
}

/// `MEASURE_QUALIFICATION(name, description, qualified_measure, qualifiers)`
/// — attaches a SET of `value_qualifier` SELECT entries to a measure.
/// `qualified_measure` unresolved drops the occurrence; individual
/// `qualifiers` members not in the modelled SELECT subset (e.g.
/// `precision_qualifier`) skip silently. EXPRESS WHERE requires
/// `SET[1:?]`, but step-io tolerates empty IR Vecs (round-trip safe
/// even if the write produces an empty SET literal).
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureQualification {
    pub name: String,
    pub description: String,
    pub qualified_measure: MeasureWithUnitId,
    pub qualifiers: Vec<ValueQualifier>,
}

/// `PROJECTED_ZONE_DEFINITION(zone, boundaries, projection_end, projected_length)`
/// — sole occupant of the `tolerance_zone_definition` arena. The
/// blueprint's `tolerance_zone_definition` SUPERTYPE is abstract in the
/// ir.toml (no entity entry); step-io uses `Arena<ProjectedZoneDefinition>`
/// directly, mirroring the `annotation_curve_occurrence` / `LeaderCurve`
/// pattern. Future phases may promote to an enum if other variants
/// (`non_uniform_zone_definition` / `runout_zone_definition`) are
/// introduced.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedZoneDefinition {
    pub zone: ToleranceZoneId,
    pub boundaries: Vec<ShapeAspectRef>,
    pub projection_end: ShapeAspectRef,
    pub projected_length: MeasureWithUnitId,
}

/// `GEOMETRIC_TOLERANCE_RELATIONSHIP(name, description, relating, related)`
/// — pairs two `geometric_tolerance` arena entries (each one can be a
/// `Plain` GT or a `WithDatumReference` GT, hence
/// [`GeometricToleranceRef`]).
/// Either side unresolved drops the relationship, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricToleranceRelationship {
    pub name: String,
    pub description: String,
    pub relating: GeometricToleranceRef,
    pub related: GeometricToleranceRef,
}

/// `draughting_callout` `complex_supertype` enum. The `Plain` variant
/// covers direct `DRAUGHTING_CALLOUT` instances (the blueprint marks the
/// supertype non-abstract, and fixtures carry many such base instances);
/// `LeaderDirected` covers the sole `in_enum` leaf currently introduced
/// (`LEADER_DIRECTED_CALLOUT`). Future phases may add more leaf
/// variants — the EXPRESS schema lists 10.
#[derive(Debug, Clone, PartialEq)]
pub enum DraughtingCallout {
    Plain(DraughtingCalloutData),
    LeaderDirected(DraughtingCalloutData),
}

/// Shared shape for every `DraughtingCallout` variant: a name plus a
/// `contents` SET of `draughting_callout_element` SELECT members.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingCalloutData {
    pub name: String,
    pub contents: Vec<DraughtingCalloutElement>,
}

/// `draughting_callout_element` SELECT member, narrowed to the kinds
/// step-io currently models. `annotation_fill_area_occurrence` is not
/// represented and is silently dropped on read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DraughtingCalloutElement {
    AnnotationCurveOccurrence(AnnotationCurveOccurrenceId),
    AnnotationOccurrence(AnnotationOccurrenceId),
}

/// `DRAUGHTING_CALLOUT_RELATIONSHIP(name, description, relating, related)`
/// — pairs two `draughting_callout` instances. Resolved through
/// `draughting_callout_id_map`; if either ref does not resolve, the
/// relationship is dropped (symmetric on re-read).
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingCalloutRelationship {
    pub name: String,
    pub description: String,
    pub relating: DraughtingCalloutId,
    pub related: DraughtingCalloutId,
}

/// `annotation_occurrence` reference, narrowed to step-io's two annotation
/// occurrence arenas. The plain occurrence / its subtypes live in the
/// [`AnnotationOccurrence`] enum (`annotation_occurrence_id_map`);
/// `annotation_curve_occurrence` is a separate arena
/// (`annotation_curve_occurrence_id_map`). An unmodelled member
/// (`annotation_fill_area_occurrence`) is not represented.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationOccurrenceRef {
    AnnotationOccurrence(AnnotationOccurrenceId),
    AnnotationCurveOccurrence(AnnotationCurveOccurrenceId),
}

/// `ANNOTATION_OCCURRENCE_ASSOCIATIVITY(name, description, relating, related)`
/// — pairs two `annotation_occurrence` instances. Each ref resolves through
/// [`AnnotationOccurrenceRef`]; if either does not resolve, the associativity
/// is dropped (symmetric on re-read).
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationOccurrenceAssociativity {
    pub name: String,
    pub description: String,
    pub relating: AnnotationOccurrenceRef,
    pub related: AnnotationOccurrenceRef,
}

/// `TOLERANCE_ZONE_FORM(name)` — names a tolerance zone's geometric form
/// (e.g. `'cylindrical or circular'`).
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceZoneForm {
    pub name: String,
}

/// `TYPE_QUALIFIER(name)` — a qualifier naming a measure's type
/// (e.g. `'maximum'`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeQualifier {
    pub name: String,
}

/// `VALUE_FORMAT_TYPE_QUALIFIER(format_type)` — a qualifier giving a
/// value's display format (e.g. `'NR2 1.3'`).
#[derive(Debug, Clone, PartialEq)]
pub struct ValueFormatTypeQualifier {
    pub format_type: String,
}
