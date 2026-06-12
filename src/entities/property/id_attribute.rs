//! `ID_ATTRIBUTE` handler — property (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::property::IdAttribute;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct IdAttributeWriteInput {
    pub(crate) attr: IdAttribute,
    pub(crate) item_step: u64,
}

pub(crate) struct IdAttributeHandler;

#[step_entity(name = "ID_ATTRIBUTE")]
impl SimpleEntityHandler for IdAttributeHandler {
    type WriteInput = IdAttributeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_id_attribute(entity_id, attrs)?;
        lower::lower_id_attribute(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        IdAttributeWriteInput { attr, item_step }: IdAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        let _ = buf;
        let early = lift::lift_id_attribute(attr.attribute_value, item_step);
        Ok(serialize::serialize_id_attribute(buf, &early))
    }
}
