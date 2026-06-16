//! `POLYLINE` handler — 2D sister (pcurve subtree) → `Curve2d::Polyline`
//! (2-layer path; reuses the 3D `POLYLINE` generated bind/serialize).
//!
//! The first point ref's arena discriminates: 2D points dispatch here, 3D fall
//! through to the 3D handler.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Polyline2d;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Polyline2dHandler;

#[step_entity(name = "POLYLINE", is_2d)]
impl SimpleEntityHandler for Polyline2dHandler {
    type WriteInput = Polyline2d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_polyline(entity_id, attrs)?;
        lower::lower_polyline_2d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, polyline: Polyline2d) -> Result<u64, WriteError> {
        let points: Vec<u64> = polyline
            .points
            .into_iter()
            .map(|pid| buf.emit_point_2d(pid))
            .collect::<Result<_, _>>()?;
        let early = lift::lift_polyline(points);
        Ok(serialize::serialize_polyline(buf, &early))
    }
}
