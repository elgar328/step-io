//! `RATIO_MEASURE_WITH_UNIT` handler (units-1).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RatioMeasureWithUnitHandler;

#[step_entity(name = "RATIO_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for RatioMeasureWithUnitHandler {
    type WriteInput = (f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let Some(early) = bind::bind_ratio_measure_with_unit(entity_id, attrs)? else {
            return Ok(());
        };
        lower::lower_ratio_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        let early = lift::lift_ratio_measure_with_unit(value, unit_step);
        Ok(serialize::serialize_ratio_measure_with_unit(buf, &early))
    }
}
