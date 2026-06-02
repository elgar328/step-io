//! `SURFACE_STYLE_TRANSPARENT` handler. Leaf entity holding a
//! single transparency factor; populates `viz_transparent_map` for the
//! rendering pass to consume.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real};
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
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_TRANSPARENT")?;
        let transparency = read_real(attrs, 0, entity_id, "transparency")?;
        ctx.viz_transparent_map.insert(entity_id, transparency);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, transparency: f64) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "SURFACE_STYLE_TRANSPARENT",
            vec![Attribute::Real(transparency)],
        ))
    }
}
