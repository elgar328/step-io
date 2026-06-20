//! `AXIS1_PLACEMENT` handler — leaf (location + axis) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::ir::error::ConvertError;
use crate::ir::id::Placement1dId;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Axis1PlacementHandler;

#[step_entity(name = "AXIS1_PLACEMENT")]
impl SimpleEntityHandler for Axis1PlacementHandler {
    type WriteInput = Placement1dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_axis1_placement(entity_id, attrs)?;
        lower::lower_axis1_placement(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: Placement1dId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let placement = buf.model.geometry.placements_1d[id];
        let loc = CartesianPointHandler::write(buf, placement.location)?;
        let dir = DirectionHandler::write(buf, placement.axis)?;
        let early = lift::lift_axis1_placement(loc, dir);
        let n = serialize::serialize_axis1_placement(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
