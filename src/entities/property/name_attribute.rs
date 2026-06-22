//! `NAME_ATTRIBUTE` handler — property (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::property::NameAttribute;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct NameAttributeWriteInput {
    pub(crate) attr: NameAttribute,
    pub(crate) item_step: u64,
}

pub(crate) struct NameAttributeHandler;

#[step_entity(name = "NAME_ATTRIBUTE")]
impl SimpleEntityHandler for NameAttributeHandler {
    type WriteInput = NameAttributeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_name_attribute(entity_id, attrs)?;
        lower::lower_name_attribute(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        NameAttributeWriteInput { attr, item_step }: NameAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        let _ = buf;
        let early = lift::lift_name_attribute(attr.attribute_value, item_step);
        Ok(serialize::serialize_name_attribute(buf, &early))
    }
}
