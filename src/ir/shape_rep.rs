//! Shape-representation IR types per the ir.toml blueprint.
//!
//! ir.toml's `shape_rep` pool covers entities whose arena resolves to
//! `representation`, `representation_context`, `shape_aspect`, or related
//! shape-bridge constructs. The handler files for these entities live in
//! `crate::entities::shape_rep`; this module holds the corresponding data
//! struct definitions.

use super::id::{ProductId, UnitContextId};
use super::visualization::StyledItem;

/// Units declared in the STEP file's HEADER section.
///
/// The IR preserves original units — numeric values are **not** normalized.
/// Kernel adapters inspect `UnitContext` and convert if needed.
///
/// `length_uncertainty` is `Some` when the source file carried a
/// `distance_accuracy_value` via `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT`,
/// `None` otherwise. The value is in the source's length unit (mm / inch
/// / ...) — no normalization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitContext {
    pub length: LengthUnit,
    pub plane_angle: AngleUnit,
    pub solid_angle: SolidAngleUnit,
    pub length_uncertainty: Option<f64>,
    /// `true` when the source file wrapped the length unit in
    /// `CONVERSION_BASED_UNIT` even though the unit is SI (e.g. ABC tier
    /// emits `CBU('METRE', 1.0, base=METRE)` instead of plain SI METRE).
    /// Writer re-emits the CBU wrapper when set; non-SI units (Inch / Foot)
    /// always emit CBU regardless of this flag.
    pub length_cbu_wrapped: bool,
    /// `true` when the source file wrapped the plane-angle unit (Radian)
    /// in a self-conversion `CONVERSION_BASED_UNIT`. Degree is non-SI and
    /// always emits CBU regardless of this flag.
    pub plane_angle_cbu_wrapped: bool,
    /// `true` when the source file emits explicit `DIMENSIONAL_EXPONENTS`
    /// references in plain SI unit complexes' `NAMED_UNIT.dimensions` slot
    /// (ABC-tier convention — every plain SI shares one length DE and one
    /// dimensionless DE entity). `false` when the source uses `*` (Derived) —
    /// the OCCT / `Fusion 360` / `FreeCAD` convention. Writer threads this
    /// flag through every plain-SI and CBU base-SI emit path. CBU outer
    /// complexes always carry an explicit DE regardless of this flag
    /// (existing emit behaviour, matches every observed fixture).
    pub dim_exp_explicit: bool,
}

impl Default for UnitContext {
    /// Default unit context — millimetre / radian / steradian, no uncertainty,
    /// no CBU wrapping. Used by the writer to synthesize a context when a
    /// kernel-built IR has products/visualization but no explicit `units`
    /// entry, so the emitted STEP file is still well-formed.
    fn default() -> Self {
        Self {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        }
    }
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
    pub items: Vec<StyledItem>,
    /// Unit / uncertainty context referenced by this MDGPR. `Some(id)` indexes
    /// into [`crate::ir::model::StepModel::units`]. Fusion 360 typically uses
    /// a separate context here (different uncertainty than the geometry rep).
    /// `None` → writer emits `Attribute::Unset` for `context_of_items`
    /// (allowed by the spec for kernel-built IR with no context info).
    pub context: Option<UnitContextId>,
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
