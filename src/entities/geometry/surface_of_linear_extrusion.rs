//! `SURFACE_OF_LINEAR_EXTRUSION` handler — leaf (swept curve + extrusion
//! VECTOR) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::vector::VectorHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SurfaceOfLinearExtrusion;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceOfLinearExtrusionHandler;

#[step_entity(name = "SURFACE_OF_LINEAR_EXTRUSION")]
impl SimpleEntityHandler for SurfaceOfLinearExtrusionHandler {
    type WriteInput = SurfaceOfLinearExtrusion;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_of_linear_extrusion(entity_id, attrs)?;
        lower::lower_surface_of_linear_extrusion(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, e: SurfaceOfLinearExtrusion) -> Result<u64, WriteError> {
        let swept = buf.emit_curve(e.swept_curve)?;
        let vector = VectorHandler::write(buf, (e.extrusion_direction, e.depth))?;
        let early = lift::lift_surface_of_linear_extrusion(swept, vector);
        Ok(serialize::serialize_surface_of_linear_extrusion(
            buf, &early,
        ))
    }
}
