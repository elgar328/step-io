//! `UNCERTAINTY_MEASURE_WITH_UNIT` handler.

// DOMAIN_TBD: catalog ENTITY_GROUPS.md marks this as X but the reader handles length-flavour uncertainty. Catalog 갱신은 별도 작업.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::LengthUncertainty;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct UncertaintyMeasureWithUnitHandler;

#[step_entity(name = "UNCERTAINTY_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for UncertaintyMeasureWithUnitHandler {
    /// `(LengthUncertainty, unit_step_id, measure_type_name)` — caller
    /// (`emit_measure_with_unit`, the `measure_with_units` arena loop) resolves
    /// the unit's already-emitted STEP id and derives the measure type from the
    /// unit's `NamedUnit` category; the `LengthUncertainty` carries the numeric
    /// value plus original `name` / `description` strings; the measure type name
    /// is one of `"LENGTH_MEASURE"`, `"PLANE_ANGLE_MEASURE"`, `"SOLID_ANGLE_MEASURE"`.
    type WriteInput = (LengthUncertainty, u64, &'static str);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(early) = bind::bind_uncertainty_measure_with_unit(entity_id, attrs)? else {
            return Ok(()); // value_component (measure_value) did not bind — drop
        };
        lower::lower_uncertainty_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (unc, unit_ref, measure_type): (LengthUncertainty, u64, &'static str),
    ) -> Result<u64, WriteError> {
        let early = lift::lift_uncertainty_measure_with_unit(unc, unit_ref, measure_type);
        Ok(serialize::serialize_uncertainty_measure_with_unit(
            buf, &early,
        ))
    }
}
