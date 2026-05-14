//! Shape-representation IR types per the ir.toml blueprint.
//!
//! ir.toml's `shape_rep` pool covers entities whose arena resolves to
//! `representation`, `representation_context`, `shape_aspect`, or related
//! shape-bridge constructs. The handler files for these entities live in
//! `crate::entities::shape_rep`; this module holds the corresponding data
//! struct definitions.

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
