//! `SURFACE_STYLE_BOUNDARY` handler — phase ssb.
//!
//! `founded_item` SUBTYPE with one attribute `style_of_boundary` typed
//! by the `curve_or_render` SELECT — a `CURVE_STYLE` or a
//! `curve_style_rendering` (`SurfaceStyleRendering` variant). The SELECT
//! is narrowed at read time: the source ref is looked up in both id maps
//! (`viz_curve_style_id_map`, `viz_ssr_id_map`); a double-miss drops the
//! entity, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    CurveOrRender, FoundedItem, SurfaceStyleBoundary, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleBoundaryHandler;

#[step_entity(name = "SURFACE_STYLE_BOUNDARY", pass = Pass7SurfaceStyleBoundary)]
impl SimpleEntityHandler for SurfaceStyleBoundaryHandler {
    type WriteInput = SurfaceStyleBoundary;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_BOUNDARY")?;
        let ref_id = read_entity_ref(attrs, 0, entity_id, "style_of_boundary")?;
        let Some(style) = resolve_curve_or_render(ctx, ref_id) else {
            return Ok(());
        };
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .founded_items
            .push(FoundedItem::SurfaceStyleBoundary(SurfaceStyleBoundary {
                style_of_boundary: style,
            }));
        ctx.viz_ssb_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssb: SurfaceStyleBoundary) -> Result<u64, WriteError> {
        let step = emit_curve_or_render(buf, ssb.style_of_boundary);
        Ok(buf.push_simple("SURFACE_STYLE_BOUNDARY", vec![Attribute::EntityRef(step)]))
    }
}

/// Narrow a raw `#N` ref into a [`CurveOrRender`] by probing the two
/// id maps that supply the SELECT's members. Returns `None` if neither
/// map covers the ref — caller drops the carrier entity.
pub(crate) fn resolve_curve_or_render(ctx: &ReaderContext, ref_id: u64) -> Option<CurveOrRender> {
    if let Some(&id) = ctx.viz_curve_style_id_map.get(&ref_id) {
        return Some(CurveOrRender::CurveStyle(id));
    }
    if let Some(&id) = ctx.viz_ssr_id_map.get(&ref_id) {
        return Some(CurveOrRender::SurfaceStyleRendering(id));
    }
    None
}

/// Look up the STEP id of a [`CurveOrRender`] member by dispatching to
/// the appropriate writer cache. Reused by sibling SELECT consumers
/// (e.g. `SURFACE_STYLE_PARAMETER_LINE`).
pub(crate) fn emit_curve_or_render(buf: &WriteBuffer, cor: CurveOrRender) -> u64 {
    match cor {
        CurveOrRender::CurveStyle(id) => buf.curve_style_step_ids[id.0 as usize],
        CurveOrRender::SurfaceStyleRendering(id) => buf.ssr_step_ids[id.0 as usize],
    }
}
