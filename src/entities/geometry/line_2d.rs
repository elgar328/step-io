//! `LINE` handler — 2D sister (pcurve subtree) → `Curve2d::Line` (2-layer path;
//! reuses the 3D `LINE` generated bind/serialize).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::geometry::vector_2d::Vector2dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Line2;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Line2dHandler;

#[step_entity(name = "LINE", is_2d)]
impl SimpleEntityHandler for Line2dHandler {
    type WriteInput = Line2;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_line(entity_id, attrs)?;
        lower::lower_line_2d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, line: Line2) -> Result<u64, WriteError> {
        let pnt = CartesianPoint2dHandler::write(buf, line.point)?;
        let vec = Vector2dHandler::write(buf, (line.direction, line.magnitude))?;
        let early = lift::lift_line(pnt, vec);
        Ok(serialize::serialize_line(buf, &early))
    }
}
