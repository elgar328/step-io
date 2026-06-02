//! `CALENDAR_DATE` handler plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_integer};
use crate::ir::error::ConvertError;
use crate::ir::plm::{CalendarDate, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CALENDAR_DATE")?;
        let year_component = read_integer(attrs, 0, entity_id, "year_component")?;
        let month_component = read_integer(attrs, 1, entity_id, "month_component")?;
        let day_component = read_integer(attrs, 2, entity_id, "day_component")?;
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.dates.push(CalendarDate {
            year_component,
            month_component,
            day_component,
        });
        ctx.plm_date_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: CalendarDate) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "CALENDAR_DATE",
            vec![
                Attribute::Integer(d.year_component),
                Attribute::Integer(d.month_component),
                Attribute::Integer(d.day_component),
            ],
        ))
    }
}
