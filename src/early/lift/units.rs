//! Units-domain `lift` fns (derived-unit cluster). See the
//! [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyDerivedUnit, EarlyDerivedUnitElement, EarlyDimensionalExponents,
    EarlyLengthMeasureWithUnit, EarlyMassMeasureWithUnit, EarlyMeasureValue, EarlyMeasureWithUnit,
    EarlyPlaneAngleMeasureWithUnit, EarlyRatioMeasureWithUnit,
};
use crate::ir::representation_item::MeasureValue;
use crate::ir::units::DimensionalExponents;

/// Lift one `DERIVED_UNIT_ELEMENT` (unit pre-resolved).
pub(crate) fn lift_derived_unit_element(unit: u64, exponent: f64) -> EarlyDerivedUnitElement {
    EarlyDerivedUnitElement { unit, exponent }
}

/// Lift one plain `DERIVED_UNIT` (elements pre-resolved).
pub(crate) fn lift_derived_unit(elements: Vec<u64>) -> EarlyDerivedUnit {
    EarlyDerivedUnit { elements }
}

/// Lift one `DIMENSIONAL_EXPONENTS` (pure pass-through).
pub(crate) fn lift_dimensional_exponents(de: DimensionalExponents) -> EarlyDimensionalExponents {
    EarlyDimensionalExponents {
        length_exponent: de.length_exponent,
        mass_exponent: de.mass_exponent,
        time_exponent: de.time_exponent,
        electric_current_exponent: de.electric_current_exponent,
        thermodynamic_temperature_exponent: de.thermodynamic_temperature_exponent,
        amount_of_substance_exponent: de.amount_of_substance_exponent,
        luminous_intensity_exponent: de.luminous_intensity_exponent,
    }
}

/// Lift bare `MEASURE_WITH_UNIT` (carrier `Itself`). Refs pre-resolved
/// (`unit_step`); `value_component` reuses the `measure_value` bridge (the
/// measure kind round-trips as the typed literal `<MEASURE_TYPE>(value)`).
pub(crate) fn lift_measure_with_unit(
    measure_type: String,
    value: f64,
    unit_step: u64,
) -> EarlyMeasureWithUnit {
    EarlyMeasureWithUnit {
        value_component: super::shape_rep::measure_value_to_early(&MeasureValue::Real {
            type_name: measure_type,
            value,
        }),
        unit_component: unit_step,
    }
}

/// Lift `LENGTH_MEASURE_WITH_UNIT` — the inner measure is fixed to the canonical
/// `LENGTH_MEASURE` (the legacy writer hardcoded it; a non-canonical input like
/// `POSITIVE_LENGTH_MEASURE` was already normalized on read).
pub(crate) fn lift_length_measure_with_unit(
    value: f64,
    unit_step: u64,
) -> EarlyLengthMeasureWithUnit {
    EarlyLengthMeasureWithUnit {
        value_component: EarlyMeasureValue::LengthMeasure(value),
        unit_component: unit_step,
    }
}

/// Lift `MASS_MEASURE_WITH_UNIT` (inner fixed to `MASS_MEASURE`).
pub(crate) fn lift_mass_measure_with_unit(value: f64, unit_step: u64) -> EarlyMassMeasureWithUnit {
    EarlyMassMeasureWithUnit {
        value_component: EarlyMeasureValue::MassMeasure(value),
        unit_component: unit_step,
    }
}

/// Lift `PLANE_ANGLE_MEASURE_WITH_UNIT` (inner fixed to `PLANE_ANGLE_MEASURE`).
pub(crate) fn lift_plane_angle_measure_with_unit(
    value: f64,
    unit_step: u64,
) -> EarlyPlaneAngleMeasureWithUnit {
    EarlyPlaneAngleMeasureWithUnit {
        value_component: EarlyMeasureValue::PlaneAngleMeasure(value),
        unit_component: unit_step,
    }
}

/// Lift `RATIO_MEASURE_WITH_UNIT` (inner fixed to `POSITIVE_RATIO_MEASURE` —
/// the canonical the legacy `emit_mwu` hardcoded for this subtype).
pub(crate) fn lift_ratio_measure_with_unit(
    value: f64,
    unit_step: u64,
) -> EarlyRatioMeasureWithUnit {
    EarlyRatioMeasureWithUnit {
        value_component: EarlyMeasureValue::PositiveRatioMeasure(value),
        unit_component: unit_step,
    }
}
