//! `DIRECTION` handler (2D, pcurve subtree).
//!
//! Sister handler of [`crate::entities::geometry::direction::DirectionHandler`].

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::Direction2dId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Direction2dHandler;

#[step_entity(name = "DIRECTION")]
impl SimpleEntityHandler for Direction2dHandler {
    type WriteInput = Direction2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: reuse the 3D sister's generated bind/serialize
        // (`DIRECTION` is the same STEP entity); `lower_direction_2d` claims the
        // 2-component form into `directions_2d`.
        let early = bind::bind_direction(entity_id, attrs)?;
        lower::lower_direction_2d(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Direction2dId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let d = buf
            .model
            .geometry
            .directions_2d
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("Direction2dId({})", id.0),
            })?;
        let n = serialize::serialize_direction(buf, &lift::lift_direction_2d(d));
        buf.set_step_id(id, n);
        Ok(n)
    }
}
