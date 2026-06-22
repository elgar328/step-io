//! `CIRCLE` handler — 2D sister (pcurve subtree) → `Curve2d::Circle` (2-layer
//! path; reuses the 3D `CIRCLE` generated bind/serialize).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_2d::Axis2Placement2dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Circle2;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Circle2dHandler;

#[step_entity(name = "CIRCLE", is_2d)]
impl SimpleEntityHandler for Circle2dHandler {
    type WriteInput = Circle2;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_circle(entity_id, attrs)?;
        lower::lower_circle_2d(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: Circle2) -> Result<u64, WriteError> {
        let pos = Axis2Placement2dHandler::write(buf, c.position)?;
        let early = lift::lift_circle(pos, c.radius);
        Ok(serialize::serialize_circle(buf, &early))
    }
}
