//! NormCase hand-hook demonstration on the generic model.
//!
//! Validates that step-io's CBU-factor identification ([[project_cbu_factor_
//! identification]], `src/early/lower/units.rs`) ports cleanly ON TOP OF the
//! schema-faithful generic part-bag — i.e. a normalize hook reads the generic
//! `UnitPart::ConversionBasedUnit` + its `MeasureWithUnit` and classifies the
//! unit WITHOUT any combination/exact-case knowledge. This is a *read-time
//! analysis* (does not mutate the round-trip model), mirroring how the real
//! lower stage derives a canonical unit identity.

// Some identity variants (Pound/Ton/Degree) are modeled for completeness but
// not exercised by the available fixtures — kept to show the hook scales.
#![allow(dead_code)]

use crate::generated::*;

/// A classified physical unit identity derived from a CBU conversion factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CbuIdentity {
    Inch,
    Foot,
    Pound,
    Ton,
    Degree,
    /// Recognized neither by name nor factor — real step-io drops + warns.
    Unrecognized,
}

/// Relative-tolerance compare (source files vary in printed precision), copied
/// in spirit from `src/early/lower/units.rs::factor_eq`. [HAND]
fn factor_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}

/// Identify a length unit by its conversion factor to SI metre (the CBU's
/// conversion_factor MWU value, expressed in the referenced SI unit). Here the
/// fillet_box CBU is `25.4` in MILLIMETRE → inch. [HAND]
fn match_length_by_factor(factor: f64) -> CbuIdentity {
    if factor_eq(factor, 25.4) {
        CbuIdentity::Inch // 25.4 mm
    } else if factor_eq(factor, 0.0254) {
        CbuIdentity::Inch // 0.0254 m
    } else if factor_eq(factor, 304.8) || factor_eq(factor, 0.3048) {
        CbuIdentity::Foot
    } else {
        CbuIdentity::Unrecognized
    }
}

/// Classify every ConversionBasedUnit in the model from the generic part-bag.
/// Returns `(name_label, identity)` per CBU found. The hook walks the generic
/// representation only — no combination enumeration, no exact-case match.
pub fn classify_cbus(model: &SpikeUnits) -> Vec<(String, CbuIdentity)> {
    let mut out = Vec::new();
    for cu in &model.complex_units.items {
        for part in &cu.parts {
            if let UnitPart::ConversionBasedUnit {
                name,
                conversion_factor,
            } = part
            {
                let mwu = model.measures.get(conversion_factor.0);
                let factor = mwu.value_component.value;
                // The unit *kind* comes from the sibling marker part in the same
                // bag (LENGTH_UNIT / MASS_UNIT / ...) — read generically.
                let is_length = cu.parts.iter().any(|p| matches!(p, UnitPart::LengthUnit));
                let id = if is_length {
                    match_length_by_factor(factor)
                } else {
                    CbuIdentity::Unrecognized
                };
                out.push((name.clone(), id));
            }
        }
    }
    out
}
