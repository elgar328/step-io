//! `LOCAL_TIME` handler plm. Depends on UTC offset arena
//! populated by the date-leaf handlers.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_entity_ref, read_integer, read_optional_integer, read_optional_real,
};
use crate::ir::error::ConvertError;
use crate::ir::plm::{LocalTime, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct LocalTimeHandler;

#[step_entity(name = "LOCAL_TIME")]
impl SimpleEntityHandler for LocalTimeHandler {
    type WriteInput = LocalTime;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "LOCAL_TIME")?;
        let hour_component = read_integer(attrs, 0, entity_id, "hour_component")?;
        let minute_component = read_optional_integer(attrs, 1, entity_id, "minute_component")?;
        let second_component = read_optional_real(attrs, 2, entity_id, "second_component")?;
        let zone_ref = read_entity_ref(attrs, 3, entity_id, "zone")?;
        let Some(zone) = ctx
            .id_cache
            .get::<crate::ir::CoordinatedUniversalTimeOffsetId>(zone_ref)
        else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.local_times.push(LocalTime {
            hour_component,
            minute_component,
            second_component,
            zone,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lt: LocalTime) -> Result<u64, WriteError> {
        let zone_step_id = buf.step_id(lt.zone);
        let minute_attr = match lt.minute_component {
            Some(v) => Attribute::Integer(v),
            None => Attribute::Unset,
        };
        let second_attr = match lt.second_component {
            Some(v) => Attribute::Real(v),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "LOCAL_TIME",
            vec![
                Attribute::Integer(lt.hour_component),
                minute_attr,
                second_attr,
                Attribute::EntityRef(zone_step_id),
            ],
        ))
    }
}
