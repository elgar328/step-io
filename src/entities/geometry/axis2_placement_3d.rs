//! `AXIS2_PLACEMENT_3D` handler — leaf (location + optional `axis`/`ref_direction`)
//! (2-layer path; 2D sister self-claims a 2D location).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::ir::error::ConvertError;
use crate::ir::id::Placement3dId;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Axis2Placement3dHandler;

#[step_entity(name = "AXIS2_PLACEMENT_3D")]
impl SimpleEntityHandler for Axis2Placement3dHandler {
    type WriteInput = Placement3dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_axis2_placement_3d(entity_id, attrs)?;
        lower::lower_axis2_placement_3d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: Placement3dId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let placement = buf.model.geometry.placements[id];
        let loc = CartesianPointHandler::write(buf, placement.location)?;
        let axis = match placement.axis {
            Some(dir) => Some(DirectionHandler::write(buf, dir)?),
            None => None,
        };
        let ref_direction = match placement.ref_direction {
            Some(dir) => Some(DirectionHandler::write(buf, dir)?),
            None => None,
        };
        let early = lift::lift_axis2_placement_3d(loc, axis, ref_direction);
        let n = serialize::serialize_axis2_placement_3d(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
