//! `VECTOR` handler (2D, intermediate map).
//!
//! Sister handler of [`crate::entities::geometry::vector::VectorHandler`].
//! Like its 3D counterpart, the reader stores the resolved
//! `(Direction2dId, magnitude)` pair in `vector_2d_map` keyed by entity
//! id rather than allocating an arena entry.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::direction_2d::Direction2dHandler;
use crate::ir::Direction2dId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Vector2dHandler;

#[step_entity(name = "VECTOR", is_2d)]
impl SimpleEntityHandler for Vector2dHandler {
    type WriteInput = (Direction2dId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: reuse the 3D sister's generated bind/serialize/lift
        // (`VECTOR` is the same STEP entity); `lower_vector_2d` claims the
        // 2D-oriented form into `vector_2d_map`.
        let early = bind::bind_vector(entity_id, attrs)?;
        lower::lower_vector_2d(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (direction, magnitude): (Direction2dId, f64),
    ) -> Result<u64, WriteError> {
        let dir_n = Direction2dHandler::write(buf, direction)?;
        Ok(serialize::serialize_vector(
            buf,
            &lift::lift_vector(dir_n, magnitude),
        ))
    }
}
