//! `CIRCLE` handler — leaf (placement + radius) → `Curve::Circle` (2-layer
//! path; 2D sister self-claims a 2D placement).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Circle3;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CircleHandler;

#[step_entity(name = "CIRCLE")]
impl SimpleEntityHandler for CircleHandler {
    type WriteInput = Circle3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_circle(entity_id, attrs)?;
        lower::lower_circle(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, circle: Circle3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, circle.position)?;
        let early = lift::lift_circle(pos, circle.radius);
        Ok(serialize::serialize_circle(buf, &early))
    }
}
