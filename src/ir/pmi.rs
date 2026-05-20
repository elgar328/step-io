//! `pmi` pool — Product Manufacturing Information (GD&T) entities.
//!
//! Phase pmi-primitives seeds the pool with three dependency-free leaf
//! `single_struct` entities (tolerance / qualifier primitives). Future
//! phases add the tolerance / datum / geometric-tolerance entities that
//! consume them. `SHAPE_ASPECT` and its subtypes are *not* here — the
//! ir.toml blueprint puts them in the `shape_rep` pool.

use super::arena::Arena;
use super::id::{PresentationStyleAssignmentId, ProductId};
use super::representation_item::RepresentationItemRef;

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
    /// `DRAUGHTING_PRE_DEFINED_TEXT_FONT` arena. Phase text-font.
    pub draughting_pre_defined_text_fonts: Arena<DraughtingPreDefinedTextFont>,
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

/// `annotation_occurrence` `enum_base` — STEP `styled_item` PMI subtypes that
/// carry presentation annotations. Only [`AnnotationPlane`] is modelled;
/// the symbol / text / draughting / tessellated siblings are deferred
/// (added as variants when fixtures or modelling support arrive).
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationOccurrence {
    AnnotationPlane(AnnotationPlane),
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
