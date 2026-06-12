//! Units-domain `lower` fns (derived-unit cluster). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{EarlyDerivedUnit, EarlyDerivedUnitElement, EarlyDimensionalExponents};
use crate::ir::error::ConvertError;
use crate::ir::units::{DerivedUnit, DerivedUnitElement, DerivedUnitKind, DimensionalExponents};
use crate::reader::ReaderContext;

/// Lower one `DERIVED_UNIT_ELEMENT` (unresolved unit = silent drop).
pub(crate) fn lower_derived_unit_element(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDerivedUnitElement,
) {
    let Some(unit_id) = ctx.id_cache.get::<crate::ir::id::NamedUnitId>(early.unit) else {
        return;
    };
    let id = ctx.due_arena.push(DerivedUnitElement {
        unit: unit_id,
        exponent: early.exponent,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DERIVED_UNIT`: resolve the element refs (unresolved
/// members skip; an all-empty result warns and drops — schema WHERE SET[1:?]).
pub(crate) fn lower_derived_unit(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDerivedUnit) {
    let mut elements = Vec::with_capacity(early.elements.len());
    for r in early.elements {
        if let Some(due_id) = ctx.id_cache.get::<crate::ir::id::DerivedUnitElementId>(r) {
            elements.push(due_id);
        }
    }
    if elements.is_empty() {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "DERIVED_UNIT has no resolvable elements (schema WHERE: SET[1:?])".into(),
        });
        return;
    }
    let id = ctx.derived_unit_arena.push(DerivedUnit {
        elements,
        kind: DerivedUnitKind::Plain,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DIMENSIONAL_EXPONENTS` (pure pass-through).
pub(crate) fn lower_dimensional_exponents(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDimensionalExponents,
) {
    let id = ctx.dimensional_exponents.push(DimensionalExponents {
        length_exponent: early.length_exponent,
        mass_exponent: early.mass_exponent,
        time_exponent: early.time_exponent,
        electric_current_exponent: early.electric_current_exponent,
        thermodynamic_temperature_exponent: early.thermodynamic_temperature_exponent,
        amount_of_substance_exponent: early.amount_of_substance_exponent,
        luminous_intensity_exponent: early.luminous_intensity_exponent,
    });
    ctx.id_cache.insert(entity_id, id);
}
