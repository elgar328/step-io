//! `COORDINATED_UNIVERSAL_TIME_OFFSET` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::CoordinatedUniversalTimeOffset;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CoordinatedUniversalTimeOffsetHandler;

#[step_entity(name = "COORDINATED_UNIVERSAL_TIME_OFFSET")]
impl SimpleEntityHandler for CoordinatedUniversalTimeOffsetHandler {
    type WriteInput = CoordinatedUniversalTimeOffset;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_coordinated_universal_time_offset(entity_id, attrs)?;
        lower::lower_coordinated_universal_time_offset(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        utc: CoordinatedUniversalTimeOffset,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_coordinated_universal_time_offset(utc);
        Ok(serialize::serialize_coordinated_universal_time_offset(
            buf, &early,
        ))
    }
}
