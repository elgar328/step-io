//! `CARTESIAN_POINT` handler — 3D leaf point (2-layer path; the 2D sister
//! handler `cartesian_point_2d.rs` stays separate and self-claims 2-coord
//! points).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::PointId;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CartesianPointHandler;

#[step_entity(name = "CARTESIAN_POINT")]
impl SimpleEntityHandler for CartesianPointHandler {
    type WriteInput = PointId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cartesian_point(entity_id, attrs)?;
        lower::lower_cartesian_point(ctx, entity_id, early)
    }

    fn write(buf: &mut WriteBuffer, id: PointId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let p = buf
            .model
            .geometry
            .points
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("PointId({})", id.0),
            })?;
        let early = lift::lift_cartesian_point(p);
        let n = serialize::serialize_cartesian_point(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
