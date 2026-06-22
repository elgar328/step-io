//! `DIRECTION` handler — 3D leaf (2-layer path; the 2D sister handler
//! self-claims 2-ratio directions).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::DirectionId;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::geometry::direction_at;
use step_io_macros::step_entity;

pub(crate) struct DirectionHandler;

#[step_entity(name = "DIRECTION")]
impl SimpleEntityHandler for DirectionHandler {
    type WriteInput = DirectionId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_direction(entity_id, attrs)?;
        lower::lower_direction(ctx, entity_id, early)
    }

    fn write(buf: &mut WriteBuffer, id: DirectionId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let d = direction_at(buf.model, id)?;
        let early = lift::lift_direction(d);
        let n = serialize::serialize_direction(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
