//! `CURVE_BOUNDED_SURFACE` handler — phase cbs.
//!
//! `bounded_surface` SUBTYPE. 4 attr: `name` + `basis_surface` /
//! `boundaries` / `implicit_outer`. `boundary_curve` is not yet modelled
//! in step-io; the boundaries SET is narrowed to generic `CurveId` via
//! `curve_map`. Corpus 0 instances per `ir.toml` — round-trip test only.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::CurveBoundedSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CurveBoundedSurfaceHandler;

#[step_entity(name = "CURVE_BOUNDED_SURFACE")]
impl SimpleEntityHandler for CurveBoundedSurfaceHandler {
    type WriteInput = CurveBoundedSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_curve_bounded_surface(entity_id, attrs)?;
        lower::lower_curve_bounded_surface(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cbs: CurveBoundedSurface) -> Result<u64, WriteError> {
        let basis_step = buf.emit_surface(cbs.basis_surface)?;
        let mut boundary_steps = Vec::with_capacity(cbs.boundaries.len());
        for c in cbs.boundaries {
            boundary_steps.push(buf.emit_curve(c)?);
        }
        let early = lift::lift_curve_bounded_surface(
            cbs.name,
            basis_step,
            boundary_steps,
            cbs.implicit_outer,
        );
        Ok(serialize::serialize_curve_bounded_surface(buf, &early))
    }
}
