//! `pmi` pool — Product Manufacturing Information (GD&T) entities.
//!
//! Phase pmi-primitives seeds the pool with three dependency-free leaf
//! `single_struct` entities (tolerance / qualifier primitives). Future
//! phases add the tolerance / datum / geometric-tolerance entities that
//! consume them. `SHAPE_ASPECT` and its subtypes are *not* here — the
//! ir.toml blueprint puts them in the `shape_rep` pool.

use super::arena::Arena;
use super::id::{PresentationStyleAssignmentId, ProductId, TessellatedItemId};
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
/// The ir.toml blueprint classifies `datum_feature` as a `concrete_supertype`
/// (its only subtype, `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`, is deferred).
/// step-io models every `shape_aspect`-family member as a plain per-subtype
/// struct joined through [`ShapeAspectRef`](crate::ir::ShapeAspectRef), so
/// `DatumFeature` is a plain struct like its siblings.
#[derive(Debug, Clone, PartialEq)]
pub struct DatumFeature {
    pub name: String,
    pub description: String,
    /// `of_shape` resolved to the owning product.
    pub target: ProductId,
    pub product_definitional: bool,
}

/// `annotation_occurrence` `enum_base` — STEP `styled_item` PMI subtypes that
/// carry presentation annotations. `AnnotationPlane` and
/// `TessellatedAnnotationOccurrence` are modelled; the symbol / text /
/// draughting siblings are deferred.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationOccurrence {
    AnnotationPlane(AnnotationPlane),
    TessellatedAnnotationOccurrence(TessellatedAnnotationOccurrence),
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
