//! Shape-representation IR types per the ir.toml blueprint.
//!
//! ir.toml's `shape_rep` pool covers entities whose arena resolves to
//! `representation`, `representation_context`, `shape_aspect`, or related
//! shape-bridge constructs. The handler files for these entities live in
//! `crate::entities::shape_rep`; this module holds the corresponding data
//! struct definitions.

use super::assembly::WireframeContent;
use super::id::StyledItemId;
use super::id::{NamedUnitId, Placement3dId, ProductId, ShellId, SolidId, UnitContextId};

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
