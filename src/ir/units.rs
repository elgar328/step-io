//! Units pool IR — per the ir.toml blueprint (`units` pool).
//!
//! Coexists with [`crate::ir::shape_rep::UnitContext`] (per-file bundled
//! enums) during the dual-tracking period. `UnitContext` keeps a single
//! `LengthUnit` / `AngleUnit` / `SolidAngleUnit` per representation context
//! and is what the writer's existing unit-emit path consumes; the
//! [`UnitsPool`] introduced here is per-instance — every STEP `NAMED_UNIT`
//! complex (`LENGTH` / `PLANE_ANGLE` / `SOLID_ANGLE` / `MASS`) gets a
//! distinct [`NamedUnitId`].
//!
//! A future units-2 refactor will collapse [`UnitContext`] into named-unit
//! refs; this phase ships the arenas alongside it.

use super::arena::Arena;
use super::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};

/// Container for the `units` pool entities introduced by units-1.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnitsPool {
    pub named_units: Arena<NamedUnit>,
    pub measure_with_units: Arena<MeasureWithUnit>,
    pub derived_unit_elements: Arena<DerivedUnitElement>,
}

/// `NAMED_UNIT` arena enum — one entry per STEP `NAMED_UNIT` complex
/// entity in the source.
///
/// **In-enum variant scope for units-1**: `Length` / `PlaneAngle` /
/// `SolidAngle` / `Mass`. The blueprint's additional `named_unit` in-enum
/// variants (`si_unit` — 92% of named-unit instances,
/// `conversion_based_unit`, `context_dependent_unit`) are intentionally
/// **not** separate variants: step-io's existing complex-entity handlers
/// fold the SI / CBU / CDU character into the dimensional flavour (e.g.
/// `LengthUnit::Millimetre` already carries the SI characterisation).
/// Separating those is a units-2 refactor concern.
///
/// `area_unit` / `volume_unit` / `ratio_unit` are deferred (tiny corpus
/// footprint).
#[derive(Debug, Clone, PartialEq)]
pub enum NamedUnit {
    Length(LengthUnit),
    PlaneAngle(AngleUnit),
    SolidAngle(SolidAngleUnit),
    Mass(MassUnit),
}

/// `MASS_UNIT` flavour. `AP214e3` constrains `dimensions` to
/// `(0, 1, 0, 0, 0, 0, 0)` — only the mass exponent is non-zero.
///
/// Variants reflect the corpus: kilogram (plain SI), gram (plain SI), and
/// pound (`CONVERSION_BASED_UNIT` wrapper). Unknown CBU names fall back to
/// [`MassUnit::Kilogram`] (lossy round-trip) — observed names are added as
/// variants when the corpus reveals them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MassUnit {
    #[default]
    Kilogram,
    Gram,
    Pound,
}

/// `MEASURE_WITH_UNIT` arena enum — one entry per concrete MWU subtype
/// (`LENGTH` / `MASS` / `PLANE_ANGLE` / `RATIO`) observed in the source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeasureWithUnit {
    Length {
        value: f64,
        unit: super::id::NamedUnitId,
    },
    Mass {
        value: f64,
        unit: super::id::NamedUnitId,
    },
    PlaneAngle {
        value: f64,
        unit: super::id::NamedUnitId,
    },
    Ratio {
        value: f64,
        unit: super::id::NamedUnitId,
    },
}

/// `DERIVED_UNIT_ELEMENT(unit, exponent)` — STEP positional order is
/// (unit, exponent); the blueprint ir.toml lists fields alphabetically
/// (exponent first), but emission must use the positional order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedUnitElement {
    pub unit: super::id::NamedUnitId,
    pub exponent: f64,
}
