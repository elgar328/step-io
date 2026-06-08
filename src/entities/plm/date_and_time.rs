//! `DATE_AND_TIME` handler plm. Depends on date arena
//! (the date-leaf handlers) and local-time arena (the `LOCAL_TIME` handler).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::plm::{DateAndTime, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DATE_AND_TIME")?;
        let date_ref = read_entity_ref(attrs, 0, entity_id, "date_component")?;
        let time_ref = read_entity_ref(attrs, 1, entity_id, "time_component")?;
        let Some(date_component) = ctx.id_cache.get::<crate::ir::DateId>(date_ref) else {
            return Ok(());
        };
        let Some(time_component) = ctx.id_cache.get::<crate::ir::LocalTimeId>(time_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.date_and_times.push(DateAndTime {
            date_component,
            time_component,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dt: DateAndTime) -> Result<u64, WriteError> {
        let date_step_id = buf.step_id(dt.date_component);
        let time_step_id = buf.step_id(dt.time_component);
        Ok(buf.push_simple(
            "DATE_AND_TIME",
            vec![
                Attribute::EntityRef(date_step_id),
                Attribute::EntityRef(time_step_id),
            ],
        ))
    }
}
