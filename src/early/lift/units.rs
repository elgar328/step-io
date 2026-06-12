//! Units-domain `lift` fns (derived-unit cluster). See the
//! [module docs](super) for the lifting contract.

use crate::early::model::{EarlyDerivedUnit, EarlyDerivedUnitElement, EarlyDimensionalExponents};
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
