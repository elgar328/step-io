//! `PRESENTATION_LAYER_ASSIGNMENT` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PresentationLayerAssignment;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationLayerAssignmentHandler;

#[step_entity(name = "PRESENTATION_LAYER_ASSIGNMENT")]
impl SimpleEntityHandler for PresentationLayerAssignmentHandler {
    type WriteInput = PresentationLayerAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_presentation_layer_assignment(entity_id, attrs)?;
        lower::lower_presentation_layer_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pla: PresentationLayerAssignment) -> Result<u64, WriteError> {
        let assigned_items: Vec<u64> = pla
            .assigned_items
            .into_iter()
            .map(|item| item.emit_select(buf))
            .collect();
        let early =
            lift::lift_presentation_layer_assignment(pla.name, pla.description, assigned_items);
        Ok(serialize::serialize_presentation_layer_assignment(
            buf, &early,
        ))
    }
}
