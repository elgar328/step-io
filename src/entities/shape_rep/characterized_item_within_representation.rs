//! `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` handler — shape-rep domain
//! (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CharacterizedItemWithinRepresentation;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CharacterizedItemWithinRepresentationHandler;

#[step_entity(name = "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION")]
impl SimpleEntityHandler for CharacterizedItemWithinRepresentationHandler {
    type WriteInput = CharacterizedItemWithinRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_characterized_item_within_representation(entity_id, attrs)?;
        lower::lower_characterized_item_within_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ciwr: CharacterizedItemWithinRepresentation,
    ) -> Result<u64, WriteError> {
        let item_step = buf.emit_representation_item_ref(ciwr.item)?;
        let rep_step = buf.step_id(ciwr.rep);
        let early = lift::lift_characterized_item_within_representation(
            ciwr.inherited.name,
            ciwr.inherited.description,
            item_step,
            rep_step,
        );
        Ok(serialize::serialize_characterized_item_within_representation(buf, &early))
    }
}
