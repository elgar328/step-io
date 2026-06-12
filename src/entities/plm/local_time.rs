//! `LOCAL_TIME` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::LocalTime;
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
        let early = bind::bind_local_time(entity_id, attrs)?;
        lower::lower_local_time(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lt: LocalTime) -> Result<u64, WriteError> {
        let zone_step = buf.step_id(lt.zone);
        let early = lift::lift_local_time(lt, zone_step);
        Ok(serialize::serialize_local_time(buf, &early))
    }
}
