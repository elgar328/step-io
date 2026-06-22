//! `SURFACE_STYLE_BOUNDARY` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{CurveOrRender, SurfaceStyleBoundary};
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
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

/// Emit a `curve_or_render` SELECT to its output step id (members + probe order
/// generated from the enum by `StepSelect`).
pub(crate) fn emit_curve_or_render(buf: &WriteBuffer, cor: CurveOrRender) -> u64 {
    cor.emit_select(buf)
}
