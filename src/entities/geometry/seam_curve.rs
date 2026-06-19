//! `SEAM_CURVE` handler — 2-layer arena path (sister of `SURFACE_CURVE`).
//! Same `SurfaceCurveData` body; only the emitted entity name differs.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SurfaceCurveData;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SeamCurveHandler;

#[step_entity(name = "SEAM_CURVE")]
impl SimpleEntityHandler for SeamCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_seam_curve(entity_id, attrs)?;
        lower::lower_seam_curve(ctx, entity_id, early, graph);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        let early = lift::lift_seam_curve(buf, body)?;
        Ok(serialize::serialize_seam_curve(buf, &early))
    }
}
