//! `CONICAL_SURFACE` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::ConicalSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ConicalSurfaceHandler;

#[step_entity(name = "CONICAL_SURFACE")]
impl SimpleEntityHandler for ConicalSurfaceHandler {
    type WriteInput = ConicalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_conical_surface(entity_id, attrs)?;
        lower::lower_conical_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, s: ConicalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, s.position)?;
        let early = lift::lift_conical_surface(pos, s.radius, s.semi_angle);
        Ok(serialize::serialize_conical_surface(buf, &early))
    }
}
