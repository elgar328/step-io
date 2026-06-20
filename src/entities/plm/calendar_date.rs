//! `CALENDAR_DATE` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::CalendarDate;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CalendarDateHandler;

#[step_entity(name = "CALENDAR_DATE")]
impl SimpleEntityHandler for CalendarDateHandler {
    type WriteInput = CalendarDate;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_calendar_date(entity_id, attrs)?;
        lower::lower_calendar_date(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: CalendarDate) -> Result<u64, WriteError> {
        let early = lift::lift_calendar_date(d);
        Ok(serialize::serialize_calendar_date(buf, &early))
    }
}
