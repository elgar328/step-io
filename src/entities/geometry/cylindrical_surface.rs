//! `CYLINDRICAL_SURFACE` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::CylindricalSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CylindricalSurfaceHandler;

#[step_entity(name = "CYLINDRICAL_SURFACE")]
impl SimpleEntityHandler for CylindricalSurfaceHandler {
    type WriteInput = CylindricalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cylindrical_surface(entity_id, attrs)?;
        lower::lower_cylindrical_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, s: CylindricalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, s.position)?;
        let early = lift::lift_cylindrical_surface(pos, s.radius);
        Ok(serialize::serialize_cylindrical_surface(buf, &early))
    }
}
