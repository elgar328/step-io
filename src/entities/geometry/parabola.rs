//! `PARABOLA` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Parabola;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ParabolaHandler;

#[step_entity(name = "PARABOLA")]
impl SimpleEntityHandler for ParabolaHandler {
    type WriteInput = Parabola;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_parabola(entity_id, attrs)?;
        lower::lower_parabola(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, p: Parabola) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, p.position)?;
        let early = lift::lift_parabola(pos, p.focal_dist);
        Ok(serialize::serialize_parabola(buf, &early))
    }
}
