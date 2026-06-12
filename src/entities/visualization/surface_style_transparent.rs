//! `SURFACE_STYLE_TRANSPARENT` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleTransparentHandler;

#[step_entity(name = "SURFACE_STYLE_TRANSPARENT")]
impl SimpleEntityHandler for SurfaceStyleTransparentHandler {
    type WriteInput = f64;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_style_transparent(entity_id, attrs)?;
        lower::lower_surface_style_transparent(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, transparency: f64) -> Result<u64, WriteError> {
        let early = lift::lift_surface_style_transparent(transparency);
        Ok(serialize::serialize_surface_style_transparent(buf, &early))
    }
}
