//! `SURFACE_OF_REVOLUTION` handler — leaf (swept curve + axis) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis1_placement::Axis1PlacementHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SurfaceOfRevolution;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceOfRevolutionHandler;

#[step_entity(name = "SURFACE_OF_REVOLUTION")]
impl SimpleEntityHandler for SurfaceOfRevolutionHandler {
    type WriteInput = SurfaceOfRevolution;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_of_revolution(entity_id, attrs)?;
        lower::lower_surface_of_revolution(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, r: SurfaceOfRevolution) -> Result<u64, WriteError> {
        let swept = buf.emit_curve(r.swept_curve)?;
        let axis = Axis1PlacementHandler::write(buf, r.axis_placement)?;
        let early = lift::lift_surface_of_revolution(swept, axis);
        Ok(serialize::serialize_surface_of_revolution(buf, &early))
    }
}
