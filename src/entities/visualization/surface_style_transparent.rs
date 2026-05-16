//! `SURFACE_STYLE_TRANSPARENT` handler — Pass 7-5. Leaf entity holding a
//! single transparency factor; populates `viz_transparent_map` for the
//! rendering pass to consume.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_real};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct SurfaceStyleTransparentHandler;

impl SimpleEntityHandler for SurfaceStyleTransparentHandler {
    const NAME: &'static str = "SURFACE_STYLE_TRANSPARENT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7Transparent;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SST_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SurfaceStyleTransparentHandler::NAME,
    pass_level: SurfaceStyleTransparentHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SurfaceStyleTransparentHandler::read,
    },
};
