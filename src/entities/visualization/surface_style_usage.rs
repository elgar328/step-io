//! `SURFACE_STYLE_USAGE` handler — Pass 7-8. Pairs a `SURFACE_SIDE_STYLE`
//! with a side enum (Front / Back / Both).

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SurfaceSide, SurfaceStyleUsage};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_side_style::SurfaceSideStyleHandler;

pub(crate) struct SurfaceStyleUsageHandler;

impl SimpleEntityHandler for SurfaceStyleUsageHandler {
    const NAME: &'static str = "SURFACE_STYLE_USAGE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7Usage;
    type WriteInput = SurfaceStyleUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_USAGE")?;
        let side_str = read_enum(attrs, 0, entity_id, "side")?;
        let side = match side_str {
            "POSITIVE" => SurfaceSide::Front,
            "NEGATIVE" => SurfaceSide::Back,
            _ => SurfaceSide::Both, // BOTH or unknown
        };
        let style_ref = read_entity_ref(attrs, 1, entity_id, "style")?;
        let Some(style) = ctx.viz_sss_map.get(&style_ref).cloned() else {
            return Ok(());
        };
        ctx.viz_ssu_map
            .insert(entity_id, SurfaceStyleUsage { side, style });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssu: SurfaceStyleUsage) -> Result<u64, WriteError> {
        let style_ref = SurfaceSideStyleHandler::write(buf, ssu.style)?;
        let side = match ssu.side {
            SurfaceSide::Front => "POSITIVE",
            SurfaceSide::Back => "NEGATIVE",
            SurfaceSide::Both => "BOTH",
        };
        Ok(buf.push_simple(
            "SURFACE_STYLE_USAGE",
            vec![
                Attribute::Enum(side.into()),
                Attribute::EntityRef(style_ref),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SSU_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SurfaceStyleUsageHandler::NAME,
    pass_level: SurfaceStyleUsageHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SurfaceStyleUsageHandler::read,
    },
};
