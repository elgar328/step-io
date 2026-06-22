//! `SPHERICAL_SURFACE` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SphericalSurface;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SphericalSurfaceHandler;

#[step_entity(name = "SPHERICAL_SURFACE")]
impl SimpleEntityHandler for SphericalSurfaceHandler {
    type WriteInput = SphericalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_spherical_surface(entity_id, attrs)?;
        lower::lower_spherical_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, s: SphericalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, s.position)?;
        let early = lift::lift_spherical_surface(pos, s.radius);
        Ok(serialize::serialize_spherical_surface(buf, &early))
    }
}
