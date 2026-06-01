//! Property definitions — `PROPERTY_DEFINITION` + `REPRESENTATION` +
//! `PROPERTY_DEFINITION_REPRESENTATION` chain for user-defined attributes
//! (target = `PRODUCT_DEFINITION`).
//!
//! Geometric validation properties (target = `SHAPE_ASPECT`, with
//! `DERIVED_UNIT` and `CARTESIAN_POINT` items) are dropped at read time;
//! see ROADMAP item 4 (PMI scaffolding) for their support.
//!
//! Stored as a passive tree-inline IR — no semantic interpretation, raw
//! values preserved for kernel adapters that may inspect them. Mirrors the
//! visualization design (see `crate::ir::visualization`).

use super::arena::Arena;
use super::id::{
    AddressId, ApplicationContextId, CharacterizedObjectId, DerivedUnitId, DimensionalLocationId,
    DocumentId, GeneralPropertyId, GroupId, NamedUnitId, PersonAndOrganizationId, ProductId,
    PropertyDefinitionId,
};
use super::shape_aspect_ref::ShapeAspectRef;
use super::shape_rep::{DescriptiveItem, RepresentationContextRef};

/// Top-level container for property data extracted from
/// `PROPERTY_DEFINITION_REPRESENTATION` chains. Empty when the source file
/// had no user-defined attributes (or when a kernel adapter built the IR
/// without one).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropertyPool {
    /// `property_definition` arena per the ir.toml blueprint
    /// (`concrete_supertype`, `shape = "carrier"`). Holds the abstract
    /// supertype body (`Itself`) and its `PRODUCT_DEFINITION_SHAPE` subtype
    /// as one arena, ordered by source `#N` (BTreeMap-driven dispatch).
    /// Writer emits PD/PDS by iterating this arena — arena order ==
    /// STEP output order, so re-read populates an identical arena.
    pub property_definitions: Arena<PropertyDefinition>,
    /// Property records (PD + REPRESENTATION + PDR collapsed) — an arena so
    /// `GENERAL_PROPERTY_ASSOCIATION.derived_definition` can reference one
    /// by [`PropertyId`]. Arena (not `Vec`) order = source `#N` order.
    pub properties: Arena<Property>,
    /// `GENERAL_PROPERTY` arena — AP242 user-defined attribute definitions.
    pub general_properties: Arena<GeneralProperty>,
    /// `GENERAL_PROPERTY_ASSOCIATION` arena — links each `GeneralProperty`
    /// to the [`Property`] carrying its value.
    pub general_property_associations: Arena<GeneralPropertyAssociation>,
    /// `NAME_ATTRIBUTE` arena. Initial SELECT coverage:
    /// [`NameAttributeItem::ProductDefinition`] + [`NameAttributeItem::DerivedUnit`].
    pub name_attributes: Arena<NameAttribute>,
    /// `DESCRIPTION_ATTRIBUTE` arena. Initial SELECT coverage:
    /// [`DescriptionAttributeItem::PersonAndOrganization`].
    pub description_attributes: Arena<DescriptionAttribute>,
    /// `ID_ATTRIBUTE` arena. Initial SELECT coverage:
    /// [`IdAttributeItem::ShapeAspect`] / [`IdAttributeItem::Group`] /
    /// [`IdAttributeItem::Address`] / [`IdAttributeItem::ApplicationContext`].
    pub id_attributes: Arena<IdAttribute>,
    /// `dimensional_characteristic_representation` arena (phase sdr-dcr).
    /// Pairs a `dimensional_characteristic` SELECT (Location | Size) with
    /// a `RepresentationId` (narrowed by EXPRESS to a
    /// `SHAPE_DIMENSION_REPRESENTATION`; step-io carries the generic
    /// `RepresentationId`).
    pub dimensional_characteristic_representations: Arena<DimensionalCharacteristicRepresentation>,
    /// PROPERTY_DEFINITION-based `SHAPE_DEFINITION_REPRESENTATION`s — those
    /// whose `definition` is a `PROPERTY_DEFINITION` (geometric-validation /
    /// CATIA geometric-set PMI shapes), not the product-geometry PDS that
    /// folds into `Product.representation_id`. Phase 1 of the SDR-arena
    /// refactor; later phases fold these (and PDR) into the blueprint
    /// `property_definition_representation` arena.
    pub shape_definition_representations: Arena<ShapeDefinitionRepresentationLink>,
    /// `PROPERTY_DEFINITION_REPRESENTATION`s whose `used_representation` is an
    /// already-modelled representation (e.g. `SHAPE_REPRESENTATION`) rather
    /// than the generic descriptive-property `REPRESENTATION` folded into
    /// [`Property`]. The writer references the existing representation's step
    /// id (no fresh REPRESENTATION emitted). Mirrors
    /// [`PropertyPool::shape_definition_representations`].
    pub property_definition_representations: Arena<PropertyDefinitionRepresentationLink>,
}

/// A `SHAPE_DEFINITION_REPRESENTATION(definition, used_representation)` whose
/// `definition` resolves to a [`PropertyDefinition`] (not the product PDS).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDefinitionRepresentationLink {
    /// `definition` — a [`PropertyPool::property_definitions`] arena slot.
    pub definition: PropertyDefinitionId,
    /// `used_representation` — a `representations` arena entry.
    pub used_representation: crate::ir::id::RepresentationId,
}

/// A `PROPERTY_DEFINITION_REPRESENTATION(definition, used_representation)` whose
/// `used_representation` is an already-modelled representation (e.g.
/// `SHAPE_REPRESENTATION`). Both refs point at existing arena entries — the
/// writer reuses their cached step ids. Mirrors
/// [`ShapeDefinitionRepresentationLink`].
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDefinitionRepresentationLink {
    /// `definition` — a [`PropertyPool::property_definitions`] arena slot.
    pub definition: PropertyDefinitionId,
    /// `used_representation` — a `representations` arena entry.
    pub used_representation: crate::ir::id::RepresentationId,
}

/// `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION(dimension, representation)`.
/// `dimension` resolves to a [`DimensionalCharacteristic`] variant;
/// `representation` to a [`RepresentationId`]. Either ref unresolved drops
/// the occurrence, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionalCharacteristicRepresentation {
    pub dimension: super::pmi::DimensionalCharacteristic,
    pub representation: super::id::RepresentationId,
}

/// `NAME_ATTRIBUTE(attribute_value, named_item)` — AP242 metadata
/// annotation attaching a free-form label to another entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NameAttribute {
    pub attribute_value: String,
    pub named_item: NameAttributeItem,
}

/// SELECT target for [`NameAttribute::named_item`]. Initial coverage of
/// the broad `name_attribute_select` SELECT — unsupported variants are
/// dropped at read time with a warning; future phases expand the enum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NameAttributeItem {
    ProductDefinition(ProductId),
    DerivedUnit(DerivedUnitId),
}

/// `DESCRIPTION_ATTRIBUTE(attribute_value, described_item)` — AP242
/// metadata annotation attaching free-form descriptive text.
#[derive(Debug, Clone, PartialEq)]
pub struct DescriptionAttribute {
    pub attribute_value: String,
    pub described_item: DescriptionAttributeItem,
}

/// SELECT target for [`DescriptionAttribute::described_item`]. Initial
/// coverage of the broad `description_attribute_select` SELECT — see
/// [`NameAttributeItem`] for the same expansion policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DescriptionAttributeItem {
    PersonAndOrganization(PersonAndOrganizationId),
}

/// `ID_ATTRIBUTE(attribute_value, identified_item)` — AP242 identifier
/// annotation attaching a label to another entity.
#[derive(Debug, Clone, PartialEq)]
pub struct IdAttribute {
    pub attribute_value: String,
    pub identified_item: IdAttributeItem,
}

/// SELECT target for [`IdAttribute::identified_item`]. `ShapeAspect` carries a
/// [`ShapeAspectRef`] but the reader only constructs the variants whose arenas
/// the round-trip diff compares (plain `shape_aspect` / `composite_group` /
/// `centre_of_symmetry` / `all_around` / `datum` / `datum_feature`); the other
/// `ShapeAspectRef` members (`datum_system` / `datum_target` /
/// `placed_datum_target_feature`) and `tolerance_zone` /
/// `datum_reference_compartment` are dropped at read time with a warning.
/// `Group` / `Address` / `ApplicationContext` are plm-metadata targets;
/// `PropertyDefinition` a user-defined-attribute target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdAttributeItem {
    ShapeAspect(ShapeAspectRef),
    PropertyDefinition(PropertyDefinitionId),
    Group(GroupId),
    Address(AddressId),
    ApplicationContext(ApplicationContextId),
}

/// `property_definition` arena enum per the ir.toml blueprint
/// (`concrete_supertype`, `shape = "carrier"`). The `Itself` variant carries
/// the abstract supertype body; `ProductDefinitionShape` wraps the
/// blueprint-narrowed concrete subtype step-io currently models.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyDefinition {
    Itself(PropertyDefinitionData),
    ProductDefinitionShape(ProductDefinitionShape),
}

/// `PROPERTY_DEFINITION(name, description, definition)` carrier body —
/// shared by every [`PropertyDefinition`] variant per the blueprint's
/// `shape = "carrier"` rule.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDefinitionData {
    pub name: String,
    pub description: String,
    pub definition: CharacterizedDefinition,
}

/// `PRODUCT_DEFINITION_SHAPE` subtype body. AP242 declares no additional
/// own attributes — the wrapper exists for variant identity and future
/// subtype extension.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductDefinitionShape {
    pub inherited: PropertyDefinitionData,
}

/// SELECT target for [`PropertyDefinitionData::definition`]. step-io
/// flattens the nested `characterized_definition →
/// characterized_product_definition → product_definition` chain to keep
/// the enum shallow. Initial coverage: the `product_definition` member
/// only — PDs whose target is a `PRODUCT_DEFINITION` (or
/// `_WITH_ASSOCIATED_DOCUMENTS`) resolve to a [`ProductId`]. PDS instances
/// whose target is a `NEXT_ASSEMBLY_USAGE_OCCURRENCE` are dropped from
/// this arena (their classification still feeds the assembly chain's
/// NAUO-owned PDS emit path).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CharacterizedDefinition {
    ProductDefinition(ProductId),
    /// Pattern B: `PROPERTY_DEFINITION` whose `definition` resolves to a
    /// `SHAPE_ASPECT` or any of its subtypes. The unified
    /// [`ShapeAspectRef`] enum carries the resolved subtype.
    ShapeAspect(ShapeAspectRef),
    /// `shape_aspect_relationship` member of the SELECT — the corpus only
    /// uses the `dimensional_location` family (`DIMENSIONAL_LOCATION` /
    /// `DIRECTED_DIMENSIONAL_LOCATION` / `ANGULAR_LOCATION`).
    DimensionalLocation(DimensionalLocationId),
    /// Pattern C: `PROPERTY_DEFINITION` whose `definition` resolves to a
    /// `PRODUCT_DEFINITION_SHAPE` (the `shape_definition` member of the
    /// `characterized_definition` SELECT). The target is itself another PD
    /// arena entry of the `ProductDefinitionShape` variant.
    ProductDefinitionShape(PropertyDefinitionId),
    /// `general_property` member of `characterized_definition` — a
    /// standalone `GENERAL_PROPERTY` with no product binding.
    GeneralProperty(GeneralPropertyId),
    /// `characterized_object` member of `characterized_definition`, resolved
    /// to a `DOCUMENT_FILE` (a `characterized_object` subtype). No product
    /// binding. Plain `DOCUMENT` is not a `characterized_object` and is not
    /// accepted here.
    Document(DocumentId),
    /// `characterized_object` member resolved to a
    /// `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` (a `characterized_object`
    /// subtype) — geometric-validation property shapes. No product binding.
    CharacterizedItemWithinRepresentation(CharacterizedObjectId),
    /// `geometric_tolerance` member — `Plain` or `WithDatumReference` (the
    /// complex MI form). Both arenas live in the `pmi` pool.
    GeometricTolerance(crate::ir::pmi::GeometricToleranceRef),
    /// `dimensional_size` member (`pmi` pool).
    DimensionalSize(crate::ir::id::DimensionalSizeId),
}

/// `PROPERTY_DEFINITION` + bound `REPRESENTATION` collapsed into a single
/// IR record. Each `PROPERTY_DEFINITION_REPRESENTATION` produces one entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// `PROPERTY_DEFINITION.name` — user-facing label like `"p1"`, `"p3"`.
    pub name: String,
    /// `PROPERTY_DEFINITION.description` — often `"user defined attribute"`,
    /// occasionally empty. Empty / `$` source values both round-trip as
    /// `None`; non-empty strings as `Some(s)`.
    pub description: Option<String>,
    /// Index into the [`PropertyPool::property_definitions`] arena for the
    /// PD entry that pairs with this Property. Reader's PDR handler fills
    /// it by resolving the source PD step ref through `property_def_step_to_id`;
    /// writer's `emit_property` uses it to fetch the cached PD step id
    /// (`property_definition_step_ids[definition.0]`) and emit the PDR
    /// linking REPR ↔ PD without re-emitting the PD.
    pub definition: PropertyDefinitionId,
    /// `REPRESENTATION.name` (often `''`).
    pub representation_name: String,
    /// `REPRESENTATION.context_of_items` — a unit-bearing or unit-less
    /// representation context. `None` when the source omitted it (rare).
    pub context: Option<RepresentationContextRef>,
    /// `REPRESENTATION.items` — polymorphic items in source order.
    pub items: Vec<PropertyItem>,
}

/// Polymorphic container for `REPRESENTATION.items` entries reached
/// through the property cluster.
///
/// This is a **step-io composite enum** — not to be confused with the
/// ir.toml blueprint's `RepresentationItem` enum. A property
/// `REPRESENTATION`'s items are either a `MEASURE_REPRESENTATION_ITEM`
/// (referenced in the `representation_item` arena) or a
/// `DESCRIPTIVE_REPRESENTATION_ITEM`.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyItem {
    Descriptive(DescriptiveItem),
    /// A `MEASURE_REPRESENTATION_ITEM` referenced in the `representation_item`
    /// arena (phase measure-arena-3/4) — the property REPRESENTATION points at
    /// the arena entry rather than carrying the value inline.
    MeasureItem(crate::ir::id::RepresentationItemId),
}

/// Source of a measure's unit. `Named` for simple units (length, mass, ...),
/// `Derived` for composite units (e.g. kg/m³). Carried by a
/// [`crate::ir::representation_item::MeasureRepresentationItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyMeasureUnit {
    Named(NamedUnitId),
    Derived(DerivedUnitId),
}

/// `GENERAL_PROPERTY(id, name, description)` — AP242 user-defined attribute
/// *definition* (e.g. a part-number or material attribute). Referenced
/// only by [`GeneralPropertyAssociation::base_definition`].
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralProperty {
    /// `GENERAL_PROPERTY.id` — often `''`.
    pub id: String,
    /// `GENERAL_PROPERTY.name` — the attribute label (e.g. `"SACHNUMMER"`).
    pub name: String,
    /// `GENERAL_PROPERTY.description` — empty / `$` both round-trip as `None`.
    pub description: Option<String>,
}

/// `GENERAL_PROPERTY_ASSOCIATION(name, description, base_definition,
/// derived_definition)` — links a [`GeneralProperty`] to the property
/// occurrence that carries the attribute's value.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralPropertyAssociation {
    pub name: String,
    pub description: Option<String>,
    /// The associated attribute definition.
    pub base_definition: GeneralPropertyId,
    /// The property occurrence the attribute is bound to.
    pub derived_definition: DerivedDefinitionItem,
}

/// SELECT target for [`GeneralPropertyAssociation::derived_definition`].
/// Initial coverage of the schema's `derived_property_select`:
/// `property_definition`, resolved to the [`PropertyDefinition`] arena
/// entry (the schema-faithful surface — not the collapsed [`Property`]
/// record). Unsupported SELECT members are dropped at read time with a
/// warning — same expansion policy as [`NameAttributeItem`] /
/// [`IdAttributeItem`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DerivedDefinitionItem {
    PropertyDefinition(PropertyDefinitionId),
}
