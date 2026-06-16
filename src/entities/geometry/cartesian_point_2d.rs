//! `CARTESIAN_POINT` handler (2D variant).
//!
//! Sister handler of [`crate::entities::geometry::cartesian_point::CartesianPointHandler`].
//! Same STEP entity name; both select which arena receives the entity
//! by coordinate count. A
//! 2-coordinate point goes to `geometry.points_2d`; a 3-coordinate point
//! goes to `geometry.points`. Wrong-dimension or malformed inputs land
//! in no arena (silent skip).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::Point2dId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CartesianPoint2dHandler;

#[step_entity(name = "CARTESIAN_POINT")]
impl SimpleEntityHandler for CartesianPoint2dHandler {
    type WriteInput = Point2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: reuse the 3D sister's generated bind/serialize
        // (`CARTESIAN_POINT` is the same STEP entity); `lower_cartesian_point_2d`
        // claims the 2-coordinate form into `points_2d`.
        let early = bind::bind_cartesian_point(entity_id, attrs)?;
        lower::lower_cartesian_point_2d(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Point2dId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let p = buf
            .model
            .geometry
            .points_2d
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("Point2dId({})", id.0),
            })?;
        let n = serialize::serialize_cartesian_point(buf, &lift::lift_cartesian_point_2d(p));
        buf.set_step_id(id, n);
        Ok(n)
    }
}
