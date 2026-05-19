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

use super::id::{DerivedUnitId, NamedUnitId, ProductId, UnitContextId};
use super::shape_rep::DescriptiveItem;

/// Top-level container for property data extracted from
/// `PROPERTY_DEFINITION_REPRESENTATION` chains. Empty when the source file
/// had no user-defined attributes (or when a kernel adapter built the IR
/// without one).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropertyPool {
    /// Top-level property records — emit order preserved.
    pub properties: Vec<Property>,
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
    /// `PROPERTY_DEFINITION.definition` resolved to a Product. PDs whose
    /// target ref does not resolve to a `PRODUCT_DEFINITION` (e.g.
    /// `SHAPE_ASPECT`) are dropped at read time.
    pub target: ProductId,
    /// `REPRESENTATION.name` (often `''`).
    pub representation_name: String,
    /// `REPRESENTATION.context_of_items` — links to a [`UnitContext`] entry.
    /// `None` when the source omitted it (rare).
    pub context: Option<UnitContextId>,
    /// `REPRESENTATION.items` — polymorphic items in source order.
    pub items: Vec<PropertyItem>,
}

/// Polymorphic container for `REPRESENTATION.items` entries reached
/// through the property cluster.
///
/// This is a **step-io composite enum** — not to be confused with the
/// ir.toml blueprint's `RepresentationItem` enum (which has five direct
/// variants: integer / measure / qualified / real / value
/// representation items). step-io picks a subset of the schema's
/// `representation_item` ISA subtree by composing blueprint building
/// blocks ([`PropertyMeasure`] and
/// [`crate::ir::shape_rep::DescriptiveItem`]). Future phases extend
/// this enum in-place — variant name and module location stay stable.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyItem {
    Measure(PropertyMeasure),
    Descriptive(DescriptiveItem),
}

/// `MEASURE_REPRESENTATION_ITEM(name, typed_value, unit_ref)` reduced to a
/// passive value carrier. When the source MRI's `unit_component` resolves to
/// a known unit in the IR, [`unit_ref`] holds the explicit reference;
/// otherwise it is `None` and the writer falls back to resolving the unit
/// from the parent [`Property`]'s context + the [`MeasureKind`].
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyMeasure {
    /// `MEASURE_REPRESENTATION_ITEM.name` (often `''`).
    pub name: String,
    /// Typed value wrapper.
    pub kind: MeasureKind,
    pub value: f64,
    /// Explicit unit reference captured from `MEASURE_REPRESENTATION_ITEM.unit_component`.
    /// `None` when the `unit_component` ref did not resolve to a known
    /// `NamedUnit` / `DerivedUnit` arena entry (legacy fixtures, kernel-built
    /// IR). The writer falls back to dynamic `UnitContext` lookup in that case.
    pub unit_ref: Option<PropertyMeasureUnit>,
}

/// Source of a [`PropertyMeasure`]'s unit. `Named` for simple units
/// (length, mass, ...), `Derived` for composite units (e.g. kg/m³).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyMeasureUnit {
    Named(NamedUnitId),
    Derived(DerivedUnitId),
}

/// Subset of STEP measure kinds that step-io currently round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasureKind {
    Length,
    PlaneAngle,
    SolidAngle,
    PositiveRatio,
    Mass,
}
