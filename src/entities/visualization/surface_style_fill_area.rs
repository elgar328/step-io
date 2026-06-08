//! `SURFACE_STYLE_FILL_AREA` handler. Wraps a `FILL_AREA_STYLE`
//! into one of the `SURFACE_SIDE_STYLE` entry variants. Pushes into the
//! shared `founded_item` arena as the `SurfaceStyleFillArea` variant.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{FoundedItem, SurfaceStyleFillArea, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleFillAreaHandler;

#[step_entity(name = "SURFACE_STYLE_FILL_AREA")]
impl SimpleEntityHandler for SurfaceStyleFillAreaHandler {
    type WriteInput = SurfaceStyleFillArea;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_FILL_AREA")?;
        let fas_ref = read_entity_ref(attrs, 0, entity_id, "fill_area")?;
        let Some(&fill_area) = ctx.viz_fas_id_map.get(&fas_ref) else {
            return Ok(());
        };
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .founded_items
            .push(FoundedItem::SurfaceStyleFillArea(SurfaceStyleFillArea {
                fill_area,
            }));
        ctx.viz_ssfa_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssfa: SurfaceStyleFillArea) -> Result<u64, WriteError> {
        let fas_step_id = buf.founded_item_step_ids[ssfa.fill_area.0 as usize];
        Ok(buf.push_simple(
            "SURFACE_STYLE_FILL_AREA",
            vec![Attribute::EntityRef(fas_step_id)],
        ))
    }
}
