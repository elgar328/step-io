//! `DESCRIPTION_ATTRIBUTE` handler — property (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::property::DescriptionAttribute;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DescriptionAttributeWriteInput {
    pub(crate) attr: DescriptionAttribute,
    pub(crate) item_step: u64,
}

pub(crate) struct DescriptionAttributeHandler;

#[step_entity(name = "DESCRIPTION_ATTRIBUTE")]
impl SimpleEntityHandler for DescriptionAttributeHandler {
    type WriteInput = DescriptionAttributeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_description_attribute(entity_id, attrs)?;
        lower::lower_description_attribute(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        DescriptionAttributeWriteInput { attr, item_step }: DescriptionAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        let _ = buf;
        let early = lift::lift_description_attribute(attr.attribute_value, item_step);
        Ok(serialize::serialize_description_attribute(buf, &early))
    }
}
