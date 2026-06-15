//! `MASS_MEASURE_WITH_UNIT` handler (units-1).
//!
//! Sister of [`crate::entities::units::length_measure_with_unit`]; same
//! attribute shape with `MASS_MEASURE(real)` as the typed value component.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MassMeasureWithUnitHandler;

#[step_entity(name = "MASS_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for MassMeasureWithUnitHandler {
    type WriteInput = (f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if ctx.cbu_internal_mwu_refs.contains(&entity_id) {
            return Ok(());
        }
        let Some(early) = bind::bind_mass_measure_with_unit(entity_id, attrs)? else {
            return Ok(());
        };
        lower::lower_mass_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        let early = lift::lift_mass_measure_with_unit(value, unit_step);
        Ok(serialize::serialize_mass_measure_with_unit(buf, &early))
    }
}
