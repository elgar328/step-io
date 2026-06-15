//! `PLANE_ANGLE_MEASURE_WITH_UNIT` handler (units-1).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlaneAngleMeasureWithUnitHandler;

#[step_entity(name = "PLANE_ANGLE_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for PlaneAngleMeasureWithUnitHandler {
    type WriteInput = (f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if ctx.cbu_internal_mwu_refs.contains(&entity_id) {
            return Ok(());
        }
        let Some(early) = bind::bind_plane_angle_measure_with_unit(entity_id, attrs)? else {
            return Ok(());
        };
        lower::lower_plane_angle_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        let early = lift::lift_plane_angle_measure_with_unit(value, unit_step);
        Ok(serialize::serialize_plane_angle_measure_with_unit(
            buf, &early,
        ))
    }
}
