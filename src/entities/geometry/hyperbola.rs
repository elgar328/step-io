//! `HYPERBOLA` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Hyperbola;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct HyperbolaHandler;

#[step_entity(name = "HYPERBOLA")]
impl SimpleEntityHandler for HyperbolaHandler {
    type WriteInput = Hyperbola;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_hyperbola(entity_id, attrs)?;
        lower::lower_hyperbola(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, h: Hyperbola) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, h.position)?;
        let early = lift::lift_hyperbola(pos, h.semi_axis, h.semi_imag_axis);
        Ok(serialize::serialize_hyperbola(buf, &early))
    }
}
