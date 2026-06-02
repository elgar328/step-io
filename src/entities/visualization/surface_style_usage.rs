//! `SURFACE_STYLE_USAGE` handler — Pass 7-8. Pairs a `SURFACE_SIDE_STYLE`
//! arena ref with a side enum (Front / Back / Both). Pushes into the
//! shared `founded_item` arena as the `SurfaceStyleUsage` variant.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{FoundedItem, SurfaceSide, SurfaceStyleUsage, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleUsageHandler;

#[step_entity(name = "SURFACE_STYLE_USAGE")]
impl SimpleEntityHandler for SurfaceStyleUsageHandler {
    type WriteInput = SurfaceStyleUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_USAGE")?;
        let side_str = read_enum(attrs, 0, entity_id, "side")?;
        let side = match side_str {
            "POSITIVE" => SurfaceSide::Front,
            "NEGATIVE" => SurfaceSide::Back,
            _ => SurfaceSide::Both, // BOTH or unknown
        };
        let style_ref = read_entity_ref(attrs, 1, entity_id, "style")?;
        let Some(&style) = ctx.viz_sss_id_map.get(&style_ref) else {
            return Ok(());
        };
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .founded_items
            .push(FoundedItem::SurfaceStyleUsage(SurfaceStyleUsage {
                side,
                style,
            }));
        ctx.viz_ssu_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssu: SurfaceStyleUsage) -> Result<u64, WriteError> {
        let style_ref = buf.founded_item_step_ids[ssu.style.0 as usize];
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
