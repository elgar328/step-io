//! `FACE_OUTER_BOUND` handler — face boundary wire (2-layer path). The bound loop's
//! step id is pre-resolved by the wire emitter; this handler emits the
//! boundary record.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::Orientation;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct FaceOuterBoundHandler;

#[step_entity(name = "FACE_OUTER_BOUND")]
impl SimpleEntityHandler for FaceOuterBoundHandler {
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_face_outer_bound(entity_id, attrs)?;
        lower::lower_face_outer_bound(ctx, entity_id, &early)
    }

    fn write(
        buf: &mut WriteBuffer,
        (bound, orientation): (u64, Orientation),
    ) -> Result<u64, WriteError> {
        let early = lift::lift_face_outer_bound(bound, orientation);
        Ok(serialize::serialize_face_outer_bound(buf, &early))
    }
}
