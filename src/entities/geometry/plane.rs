//! `PLANE` handler — leaf (placement) → `Surface::Plane` (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Plane3;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlaneHandler;

#[step_entity(name = "PLANE")]
impl SimpleEntityHandler for PlaneHandler {
    type WriteInput = Plane3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_plane(entity_id, attrs)?;
        lower::lower_plane(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, p: Plane3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, p.position)?;
        let early = lift::lift_plane(pos);
        Ok(serialize::serialize_plane(buf, &early))
    }
}
