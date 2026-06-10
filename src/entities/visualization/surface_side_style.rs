//! `SURFACE_SIDE_STYLE` handler. Aggregates one or more
//! `SURFACE_SIDE_STYLE` entries (each a fill-area or rendering style)
//! into a named composite. Pushes into the shared `founded_item` arena
//! as the `SurfaceSideStyle` variant.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SurfaceSideStyle, SurfaceSideStyleEntry};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct SurfaceSideStyleHandler;

#[step_entity(name = "SURFACE_SIDE_STYLE")]
impl SimpleEntityHandler for SurfaceSideStyleHandler {
    type WriteInput = SurfaceSideStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` disambiguates each
        // style member by L1 type — `FillArea` via the typed
        // `EarlySurfaceStyleFillAreaId` cache bucket (no `viz_ssfa_id_map`),
        // `Rendering` via the existing `SurfaceStyleRenderingId` bucket.
        let early = bind::bind_surface_side_style(entity_id, attrs)?;
        lower::lower_surface_side_style(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sss: SurfaceSideStyle) -> Result<u64, WriteError> {
        let mut style_refs = Vec::with_capacity(sss.styles.len());
        for entry in sss.styles {
            let entry_id = match entry {
                SurfaceSideStyleEntry::FillArea(ssfa_id) => buf.step_id(ssfa_id),
                SurfaceSideStyleEntry::Rendering(ssr_id) => buf.step_id(ssr_id),
            };
            style_refs.push(Attribute::EntityRef(entry_id));
        }
        Ok(buf.push_simple(
            "SURFACE_SIDE_STYLE",
            vec![Attribute::String(sss.name), Attribute::List(style_refs)],
        ))
    }
}
