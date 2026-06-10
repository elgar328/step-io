//! `SURFACE_STYLE_USAGE` handler. Pairs a `SURFACE_SIDE_STYLE`
//! arena ref with a side enum (Front / Back / Both). Pushes into the
//! shared `founded_item` arena as the `SurfaceStyleUsage` variant.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SurfaceSide, SurfaceStyleUsage};
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
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves `style`
        // via the typed `EarlySurfaceSideStyleId` bucket and registers the typed
        // `EarlySurfaceStyleUsageId` key (replaces viz_sss/viz_ssu maps).
        let early = bind::bind_surface_style_usage(entity_id, attrs)?;
        lower::lower_surface_style_usage(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssu: SurfaceStyleUsage) -> Result<u64, WriteError> {
        let style_ref = buf.step_id(ssu.style);
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
