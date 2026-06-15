//! Rational `QUASI_UNIFORM_SURFACE` complex (`ComplexEntityHandler`, 2-layer
//! path). A synthetic-name complex (no single early.toml entity): the parts drop
//! `B_SPLINE_SURFACE_WITH_KNOTS` and add `QUASI_UNIFORM_SURFACE`; the u/v knot
//! vectors are derived in `lower`. `read_only` — `emit_nurbs_surface` normalizes
//! every rational `Surface::Nurbs` to the `RATIONAL_B_SPLINE_SURFACE`-with-knots
//! form, so this handler's `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsSurface;
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct RationalQuasiUniformSurfaceHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_SURFACE",
    cases = [[
        "BOUNDED_SURFACE", "B_SPLINE_SURFACE", "GEOMETRIC_REPRESENTATION_ITEM",
        "QUASI_UNIFORM_SURFACE", "RATIONAL_B_SPLINE_SURFACE", "REPRESENTATION_ITEM", "SURFACE"
    ]]
)]
impl ComplexEntityHandler for RationalQuasiUniformSurfaceHandler {
    type WriteInput = NurbsSurface;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rational_quasi_uniform_surface(entity_id, parts)?;
        lower::lower_rational_quasi_uniform_surface(ctx, entity_id, &early)
    }

    fn write(_buf: &mut WriteBuffer, _nurbs: NurbsSurface) -> Result<u64, WriteError> {
        // read_only: emit_nurbs_surface normalizes rational Surface::Nurbs to the
        // RATIONAL_B_SPLINE_SURFACE-with-knots form, so this is never reached.
        unreachable!(
            "rational QUASI_UNIFORM_SURFACE is never emitted; emit_nurbs_surface normalizes to RATIONAL_B_SPLINE_SURFACE"
        )
    }
}
