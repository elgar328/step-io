//! `AXIS2_PLACEMENT_2D` handler.
//!
//! Mirrors the legacy `convert_axis2_placement_2d` and
//! `emit_axis2_placement_2d`. Distinct STEP entity name (no 3D
//! counterpart sharing it), so dispatch is straightforward.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::geometry::direction_2d::Direction2dHandler;
use crate::ir::Placement2dId;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Axis2Placement2dHandler;

#[step_entity(name = "AXIS2_PLACEMENT_2D", is_2d)]
impl SimpleEntityHandler for Axis2Placement2dHandler {
    type WriteInput = Placement2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_axis2_placement_2d(entity_id, attrs)?;
        lower::lower_axis2_placement_2d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: Placement2dId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let placement = buf.model.geometry.placements_2d[id];
        let loc = CartesianPoint2dHandler::write(buf, placement.location)?;
        let ref_direction = match placement.ref_direction {
            Some(dir) => Some(Direction2dHandler::write(buf, dir)?),
            None => None,
        };
        let early = lift::lift_axis2_placement_2d(loc, ref_direction);
        let n = serialize::serialize_axis2_placement_2d(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
