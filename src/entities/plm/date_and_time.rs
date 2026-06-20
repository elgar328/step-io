//! `DATE_AND_TIME` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DateAndTime;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DateAndTimeHandler;

#[step_entity(name = "DATE_AND_TIME")]
impl SimpleEntityHandler for DateAndTimeHandler {
    type WriteInput = DateAndTime;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_date_and_time(entity_id, attrs)?;
        lower::lower_date_and_time(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dt: DateAndTime) -> Result<u64, WriteError> {
        let date_step = buf.step_id(dt.date_component);
        let time_step = buf.step_id(dt.time_component);
        let early = lift::lift_date_and_time(date_step, time_step);
        Ok(serialize::serialize_date_and_time(buf, &early))
    }
}
