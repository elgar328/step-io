//! `SURFACE_STYLE_BOUNDARY` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{CurveOrRender, SurfaceStyleBoundary};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleBoundaryHandler;

#[step_entity(name = "SURFACE_STYLE_BOUNDARY")]
impl SimpleEntityHandler for SurfaceStyleBoundaryHandler {
    type WriteInput = SurfaceStyleBoundary;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_style_boundary(entity_id, attrs)?;
        lower::lower_surface_style_boundary(ctx, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssb: SurfaceStyleBoundary) -> Result<u64, WriteError> {
        let step = emit_curve_or_render(buf, ssb.style_of_boundary);
        let early = lift::lift_surface_style_boundary(step);
        Ok(serialize::serialize_surface_style_boundary(buf, &early))
    }
}

/// Resolve / emit helpers shared with `SURFACE_STYLE_PARAMETER_LINE` —
/// members + probe order are generated from the enum by `StepSelect`.
pub(crate) fn resolve_curve_or_render(ctx: &ReaderContext, ref_id: u64) -> Option<CurveOrRender> {
    CurveOrRender::resolve_select(ctx, ref_id)
}
pub(crate) fn emit_curve_or_render(buf: &WriteBuffer, cor: CurveOrRender) -> u64 {
    cor.emit_select(buf)
}
