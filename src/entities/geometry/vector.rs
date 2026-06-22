//! `VECTOR` handler — leaf (direction + magnitude) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::ir::DirectionId;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct VectorHandler;

#[step_entity(name = "VECTOR")]
impl SimpleEntityHandler for VectorHandler {
    type WriteInput = (DirectionId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_vector(entity_id, attrs)?;
        lower::lower_vector(ctx, entity_id, &early)
    }

    fn write(
        buf: &mut WriteBuffer,
        (direction, magnitude): (DirectionId, f64),
    ) -> Result<u64, WriteError> {
        let dir_n = DirectionHandler::write(buf, direction)?;
        let early = lift::lift_vector(dir_n, magnitude);
        Ok(serialize::serialize_vector(buf, &early))
    }
}
