//! `LINE` handler — leaf (point + VECTOR) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::vector::VectorHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Line3;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct LineHandler;

#[step_entity(name = "LINE")]
impl SimpleEntityHandler for LineHandler {
    type WriteInput = Line3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_line(entity_id, attrs)?;
        lower::lower_line(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, line: Line3) -> Result<u64, WriteError> {
        let pnt = CartesianPointHandler::write(buf, line.point)?;
        let vec = VectorHandler::write(buf, (line.direction, line.magnitude))?;
        let early = lift::lift_line(pnt, vec);
        Ok(serialize::serialize_line(buf, &early))
    }
}
