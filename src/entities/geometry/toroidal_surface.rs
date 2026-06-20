//! `TOROIDAL_SURFACE` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::ToroidalSurface;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ToroidalSurfaceHandler;

#[step_entity(name = "TOROIDAL_SURFACE")]
impl SimpleEntityHandler for ToroidalSurfaceHandler {
    type WriteInput = ToroidalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_toroidal_surface(entity_id, attrs)?;
        lower::lower_toroidal_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, s: ToroidalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, s.position)?;
        let early = lift::lift_toroidal_surface(pos, s.major_radius, s.minor_radius);
        Ok(serialize::serialize_toroidal_surface(buf, &early))
    }
}
