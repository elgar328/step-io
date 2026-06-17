//! Helpers shared by the unit-leaf handlers. All `CONVERSION_BASED_UNIT`
//! reading/identification is now 2-layer (generated bind + hand
//! `lower_*`/`lower_*_cbu`); this module keeps only the small reader-side
//! helpers the remaining hand paths still need.

use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, has_all_parts};

pub(super) fn has_part(parts: &[RawEntityPart], name: &str) -> bool {
    has_all_parts(parts, &[name])
}

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

/// Read `NAMED_UNIT.dimensions` field of a unit complex (phase
/// dim-exp-arena-c). Returns `Some(id)` when the source emitted an
/// explicit `DIMENSIONAL_EXPONENTS` ref, `None` for the `*` (Derived)
/// form. Unknown refs (cross-cascade) silently degrade to `None`.
pub(crate) fn read_named_unit_dim_exp(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
) -> Option<crate::ir::DimensionalExponentsId> {
    let attrs = find_part_attrs(parts, "NAMED_UNIT")?;
    match attrs.first()? {
        Attribute::EntityRef(n) => ctx
            .id_cache
            .get::<crate::ir::id::DimensionalExponentsId>(*n),
        _ => None,
    }
}
