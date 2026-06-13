//! `POLYLINE` handler — leaf (point list) → `Curve::Polyline` (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Polyline;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PolylineHandler;

#[step_entity(name = "POLYLINE")]
impl SimpleEntityHandler for PolylineHandler {
    type WriteInput = Polyline;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_polyline(entity_id, attrs)?;
        lower::lower_polyline(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, polyline: Polyline) -> Result<u64, WriteError> {
        let points: Vec<u64> = polyline
            .points
            .into_iter()
            .map(|pid| buf.emit_point(pid))
            .collect::<Result<_, _>>()?;
        let early = lift::lift_polyline(points);
        Ok(serialize::serialize_polyline(buf, &early))
    }
}
