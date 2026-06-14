//! `QUASI_UNIFORM_SURFACE` handler — simple, non-rational B-spline surface with
//! u/v knots derived from `(degree, cp_count)` (STEP `QuasiUniformKnots` rule).
//! 2-layer read path (`bind`→`lower`, derivation in `lower`). `read_only`:
//! `emit_nurbs_surface` normalizes every `Surface::Nurbs` to
//! `B_SPLINE_SURFACE_WITH_KNOTS`, so this handler's `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct QuasiUniformSurfaceHandler;

#[step_entity(name = "QUASI_UNIFORM_SURFACE")]
impl SimpleEntityHandler for QuasiUniformSurfaceHandler {
    type WriteInput = NurbsSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_quasi_uniform_surface(entity_id, attrs)?;
        lower::lower_quasi_uniform_surface(ctx, entity_id, &early)
    }

    fn write(_buf: &mut WriteBuffer, _nurbs: NurbsSurface) -> Result<u64, WriteError> {
        // `read_only`: `emit_nurbs_surface` always normalizes `Surface::Nurbs` to
        // `B_SPLINE_SURFACE_WITH_KNOTS`, so QUASI_UNIFORM_SURFACE is never emitted.
        unreachable!(
            "QUASI_UNIFORM_SURFACE is never emitted; emit_nurbs_surface normalizes to B_SPLINE_SURFACE_WITH_KNOTS"
        )
    }
}
