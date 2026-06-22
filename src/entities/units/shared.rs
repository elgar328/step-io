//! Helpers shared by the unit-leaf handlers. All `CONVERSION_BASED_UNIT`
//! reading/identification is now 2-layer (generated bind + hand
//! `lower_*`/`lower_*_cbu`); this module keeps only the small reader-side
//! helpers the remaining hand paths still need.

use crate::parser::entity::Attribute;

/// Normalize a bare (untyped) numeric `value_component` to the typed measure
/// form the generated `bind_measure_value` (a `measure_value` SELECT bind)
/// expects. The typed `*_MEASURE_WITH_UNIT` subtypes redeclare `value_component`
/// to a concrete measure REAL, so some exporters write the standard plain
/// `25.4` rather than the supertype-style `LENGTH_MEASURE(25.4)` (e.g. a
/// `CONVERSION_BASED_UNIT` conversion factor, NIST `fillet_box`). The SELECT
/// bind only matches the typed form, so the handler rewrites the plain
/// real/integer to the subtype's measure type before binding (reader-side
/// leniency; the generated bind stays strict). Typed inputs pass through
/// unchanged.
pub(super) fn normalize_bare_measure_attrs(
    attrs: &[Attribute],
    measure_type: &str,
) -> Vec<Attribute> {
    let mut out = attrs.to_vec();
    if let Some(first @ (Attribute::Real(_) | Attribute::Integer(_))) = out.first().cloned() {
        out[0] = Attribute::Typed {
            type_name: measure_type.into(),
            value: Box::new(first),
        };
    }
    out
}
