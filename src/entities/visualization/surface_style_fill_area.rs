//! `SURFACE_STYLE_FILL_AREA` handler — Pass 7-4. Wraps a `FILL_AREA_STYLE`
//! into one of the `SURFACE_SIDE_STYLE` entry variants.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SurfaceSideStyleEntry, SurfaceStyleFillArea};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::fill_area_style::FillAreaStyleHandler;

pub(crate) struct SurfaceStyleFillAreaHandler;

impl SimpleEntityHandler for SurfaceStyleFillAreaHandler {
    const NAME: &'static str = "SURFACE_STYLE_FILL_AREA";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7SurfaceFill;
    type WriteInput = SurfaceStyleFillArea;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_FILL_AREA")?;
        let fas_ref = read_entity_ref(attrs, 0, entity_id, "fill_area")?;
        let Some(fill_area) = ctx.viz_fas_map.get(&fas_ref).cloned() else {
            return Ok(());
        };
        ctx.viz_sss_entry_map.insert(
            entity_id,
            SurfaceSideStyleEntry::FillArea(SurfaceStyleFillArea { fill_area }),
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssfa: SurfaceStyleFillArea) -> Result<u64, WriteError> {
        let fas_id = FillAreaStyleHandler::write(buf, ssfa.fill_area)?;
        Ok(buf.push_simple(
            "SURFACE_STYLE_FILL_AREA",
            vec![Attribute::EntityRef(fas_id)],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SSFA_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SurfaceStyleFillAreaHandler::NAME,
    pass_level: SurfaceStyleFillAreaHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SurfaceStyleFillAreaHandler::read,
    },
};
