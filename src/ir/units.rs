//! Units pool IR — per the ir.toml blueprint (`units` pool).
//!
//! Every STEP `NAMED_UNIT` complex (`LENGTH` / `PLANE_ANGLE` / `SOLID_ANGLE`
//! / `MASS`) gets a distinct [`NamedUnitId`] in [`UnitsPool::named_units`].
//! [`crate::ir::shape_rep::UnitContext`] references these by id, and
//! `MEASURE_WITH_UNIT` / `DERIVED_UNIT_ELEMENT` entries do the same — there
//! is no duplicate per-file enum carrier.
//!
//! CBU outer ↔ base SI is modelled explicitly via the `cbu_base` field on
//! [`LengthFlavor`] / [`PlaneAngleFlavor`] / [`MassFlavor`], so the writer
//! reproduces the source file's `NAMED_UNIT` entity-id ordering on round-trip
//! (no inline base-SI generation that would shift arena indices).

use super::arena::Arena;
use super::id::NamedUnitId;
use super::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};

/// Container for the `units` pool entities.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnitsPool {
    pub named_units: Arena<NamedUnit>,
    pub measure_with_units: Arena<MeasureWithUnit>,
    pub derived_unit_elements: Arena<DerivedUnitElement>,
    pub derived_units: Arena<DerivedUnit>,
    /// `DIMENSIONAL_EXPONENTS` arena (phase dim-exp-arena-a). Schema-
    /// faithful standalone arena; per-`NAMED_UNIT` wiring lands in
    /// phase dim-exp-arena-b.
    pub dimensional_exponents: Arena<DimensionalExponents>,
}

/// `DIMENSIONAL_EXPONENTS(length, mass, time, electric_current,
/// thermodynamic_temperature, amount_of_substance, luminous_intensity)`
/// — schema `single_struct`. All seven SI base-quantity exponents as
/// `REAL`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionalExponents {
    pub length_exponent: f64,
    pub mass_exponent: f64,
    pub time_exponent: f64,
    pub electric_current_exponent: f64,
    pub thermodynamic_temperature_exponent: f64,
    pub amount_of_substance_exponent: f64,
    pub luminous_intensity_exponent: f64,
}

impl UnitsPool {
    /// Push a plain SI length unit and return its arena ref.
    pub fn push_plain_length(&mut self, unit: LengthUnit) -> NamedUnitId {
        self.named_units.push(NamedUnit::Length(LengthFlavor {
            unit,
            cbu_base: None,
            dim_exp: None,
        }))
    }

    /// Push a `CONVERSION_BASED_UNIT` length outer that wraps `base`.
    /// `base` should be an SI length (Millimetre / Metre / Centimetre);
    /// it is pushed first as a plain SI entry and the outer references it.
    /// Self-wrap (outer == base) is preserved structurally via `cbu_base`.
    pub fn push_cbu_length(&mut self, outer: LengthUnit, base: LengthUnit) -> NamedUnitId {
        let base_id = self.push_plain_length(base);
        self.named_units.push(NamedUnit::Length(LengthFlavor {
            unit: outer,
            cbu_base: Some(base_id),
            dim_exp: None,
        }))
    }

    pub fn push_plain_plane_angle(&mut self, unit: AngleUnit) -> NamedUnitId {
        self.named_units
            .push(NamedUnit::PlaneAngle(PlaneAngleFlavor {
                unit,
                cbu_base: None,
                dim_exp: None,
            }))
    }

    pub fn push_cbu_plane_angle(&mut self, outer: AngleUnit, base: AngleUnit) -> NamedUnitId {
        let base_id = self.push_plain_plane_angle(base);
        self.named_units
            .push(NamedUnit::PlaneAngle(PlaneAngleFlavor {
                unit: outer,
                cbu_base: Some(base_id),
                dim_exp: None,
            }))
    }

    pub fn push_plain_solid_angle(&mut self, unit: SolidAngleUnit) -> NamedUnitId {
        self.named_units
            .push(NamedUnit::SolidAngle(SolidAngleFlavor {
                unit,
                dim_exp: None,
            }))
    }

    pub fn push_plain_mass(&mut self, unit: MassUnit) -> NamedUnitId {
        self.named_units.push(NamedUnit::Mass(MassFlavor {
            unit,
            cbu_base: None,
            dim_exp: None,
        }))
    }

    pub fn push_cbu_mass(&mut self, outer: MassUnit, base: MassUnit) -> NamedUnitId {
        let base_id = self.push_plain_mass(base);
        self.named_units.push(NamedUnit::Mass(MassFlavor {
            unit: outer,
            cbu_base: Some(base_id),
            dim_exp: None,
        }))
    }
}

/// `NAMED_UNIT` arena enum — one entry per STEP `NAMED_UNIT` complex
/// entity in the source.
///
/// `Length` / `PlaneAngle` / `SolidAngle` / `Mass` cover the four
/// dimensional flavours observed in fixtures. The blueprint's additional
/// `named_unit` variants (`si_unit`, `conversion_based_unit`,
/// `context_dependent_unit`) aren't separate variants here — the SI / CBU
/// character is folded into each flavour struct (`cbu_base`, `cbu_wrapped`).
/// `area_unit` / `volume_unit` are deferred (tiny corpus footprint).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NamedUnit {
    /// Bare `NAMED_UNIT(#dimensions)` supertype instance (carrier) — a
    /// dimensionless/count unit (e.g. all-zero `DIMENSIONAL_EXPONENTS`), the
    /// direct instantiation of the abstract `named_unit`. Blueprint
    /// `complex_supertype` + carrier (`type=NamedUnitData`, `variant=Itself`),
    /// analogous to [`MeasureWithUnit::Itself`].
    Itself(NamedUnitData),
    Length(LengthFlavor),
    PlaneAngle(PlaneAngleFlavor),
    SolidAngle(SolidAngleFlavor),
    Mass(MassFlavor),
    Ratio(RatioFlavor),
}

/// Carrier body of a bare `NAMED_UNIT` ([`NamedUnit::Itself`]). `dimensions`
/// is the `DIMENSIONAL_EXPONENTS` ref (`None` → `*` Derived); the unit has no
/// SI prefix or conversion factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NamedUnitData {
    pub dimensions: Option<super::id::DimensionalExponentsId>,
}

/// `RATIO_UNIT` flavour — always dimensionless and without a CBU variant in
/// the observed corpus. Presence in [`NamedUnit`] signals "ratio" to
/// downstream consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RatioFlavor {
    pub dim_exp: Option<super::id::DimensionalExponentsId>,
    /// `true` when read from the complex `(NAMED_UNIT()RATIO_UNIT()…)` form,
    /// `false` for the standalone simple `RATIO_UNIT(dimensions)` entity (the
    /// only form observed in the corpus). The writer reproduces the source
    /// form so round-trip stays byte-faithful.
    pub complex: bool,
}

/// `LENGTH_UNIT` complex flavour. `cbu_base` is `Some(id)` for
/// `CONVERSION_BASED_UNIT` outers and `None` for plain SI complexes.
/// The base ref makes the writer's CBU emit path lookup-driven instead
/// of inline-generating an extra `LENGTH_UNIT` entity that would otherwise
/// re-register on round-trip and shift downstream `NamedUnitId`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LengthFlavor {
    pub unit: LengthUnit,
    /// `Some(id)` → this entry is a CBU outer and `id` resolves to the
    /// base SI's `NamedUnit::Length` arena entry. `None` → plain SI.
    /// Self-wrap (`CBU('METRE', ..., base=METRE)`) is represented by
    /// `cbu_base = Some(<METRE id>)` with `unit == lookup(cbu_base).unit`.
    pub cbu_base: Option<NamedUnitId>,
    /// `NAMED_UNIT.dimensions` explicit ref (phase dim-exp-arena-b).
    /// `Some(id)` reproduces the source's `(#N)` reference; `None` emits
    /// the `*` (Derived) form. The CBU outer path still uses its dedicated
    /// `length_dim_exp_step` cache and ignores this field.
    pub dim_exp: Option<super::id::DimensionalExponentsId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaneAngleFlavor {
    pub unit: AngleUnit,
    pub cbu_base: Option<NamedUnitId>,
    pub dim_exp: Option<super::id::DimensionalExponentsId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SolidAngleFlavor {
    pub unit: SolidAngleUnit,
    // No CBU variant observed in corpus — `SOLID_ANGLE_UNIT +
    // CONVERSION_BASED_UNIT` is silently dropped on read.
    pub dim_exp: Option<super::id::DimensionalExponentsId>,
}

/// `MASS_UNIT` flavour. `AP214e3` constrains `dimensions` to
/// `(0, 1, 0, 0, 0, 0, 0)` — only the mass exponent is non-zero.
///
/// Mass currently always emits `NAMED_UNIT.dimensions = *` (Derived) so
/// no `dim_exp_explicit` flag — units-3 will revisit if any fixture
/// turns up an explicit-DE mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MassFlavor {
    pub unit: MassUnit,
    pub cbu_base: Option<NamedUnitId>,
    pub dim_exp: Option<super::id::DimensionalExponentsId>,
}

/// `MASS_UNIT` value variant. Corpus: kilogram (plain SI), gram (plain SI),
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

/// `MEASURE_WITH_UNIT` arena enum.
///
/// Per the ir.toml blueprint `measure_with_unit` is a `concrete_supertype`
/// (`shape = "carrier"`): the typed subtypes (`LENGTH_MEASURE_WITH_UNIT` etc.)
/// are the `Length` / `Mass` / `PlaneAngle` / `Ratio` variants, and the bare
/// supertype `MEASURE_WITH_UNIT(<measure_value>, <unit>)` — where the measure
/// kind lives in the typed value (`LENGTH_MEASURE(..)`), so it can carry *any*
/// measure kind — is the [`Self::Itself`] carrier variant (mirrors
/// `RepresentationRelationship::Itself`).
///
/// `Copy` is intentionally not derived: the `Itself` carrier holds a `String`.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasureWithUnit {
    /// Bare `MEASURE_WITH_UNIT` supertype instance (carrier).
    Itself(MeasureWithUnitData),
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

/// Carrier body for the bare `MEASURE_WITH_UNIT` supertype instance.
/// `measure_type` is the `value_component`'s `measure_value` SELECT member
/// name (e.g. `"LENGTH_MEASURE"`) — the measure kind, preserved verbatim so
/// the writer re-emits the same generic form rather than a typed subtype.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureWithUnitData {
    pub measure_type: String,
    pub value: f64,
    pub unit: super::id::NamedUnitId,
}

/// `DERIVED_UNIT_ELEMENT(unit, exponent)` — STEP positional order is
/// (unit, exponent); the blueprint ir.toml lists fields alphabetically
/// (exponent first), but emission must use the positional order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedUnitElement {
    pub unit: super::id::NamedUnitId,
    pub exponent: f64,
}

/// `DERIVED_UNIT(elements: SET[1:?] OF derived_unit_element)` and its
/// dimension-constrained subtypes (`AREA_UNIT`, `VOLUME_UNIT`). Per EXPRESS,
/// `AREA_UNIT` / `VOLUME_UNIT` are `SUBTYPE OF (derived_unit)` carrying the
/// same `elements` set with a `WHERE` clause fixing the dimensional
/// exponents. step-io models them as a single arena keyed by [`DerivedUnitKind`]
/// — corpus form `AREA_UNIT((#e1, #e2))` maps to `DerivedUnit { elements,
/// kind: AreaUnit }`. The schema's `SET[1:?]` cardinality is enforced at
/// read: instances whose `elements` resolve to an empty list are dropped
/// with a warning rather than admitted to the arena.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedUnit {
    pub elements: Vec<super::id::DerivedUnitElementId>,
    pub kind: DerivedUnitKind,
}

/// Concrete `derived_unit` subtype indicator. `Plain` is the bare
/// `DERIVED_UNIT`; `AreaUnit` / `VolumeUnit` are the dimension-constrained
/// subtypes from ISO 10303-41 `measure_schema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DerivedUnitKind {
    #[default]
    Plain,
    AreaUnit,
    VolumeUnit,
}
