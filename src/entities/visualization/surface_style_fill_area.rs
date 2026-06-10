//! `SURFACE_STYLE_FILL_AREA` handler. Wraps a `FILL_AREA_STYLE`
//! into one of the `SURFACE_SIDE_STYLE` entry variants. Pushes into the
//! shared `founded_item` arena as the `SurfaceStyleFillArea` variant.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::SurfaceStyleFillArea;
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
        // 2-layer path: bind → L1, then lower → L2. `lower` registers the typed
        // `EarlySurfaceStyleFillAreaId` cache key so `surface_side_style`
        // disambiguates this member by L1 type (replaces `viz_ssfa_id_map`).
        let early = bind::bind_surface_style_fill_area(entity_id, attrs)?;
        lower::lower_surface_style_fill_area(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssfa: SurfaceStyleFillArea) -> Result<u64, WriteError> {
        let fas_step_id = buf.step_id(ssfa.fill_area);
        Ok(buf.push_simple(
            "SURFACE_STYLE_FILL_AREA",
            vec![Attribute::EntityRef(fas_step_id)],
        ))
    }
}
