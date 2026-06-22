//! `DESCRIPTIVE_REPRESENTATION_ITEM` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DescriptiveItem;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DescriptiveRepresentationItemHandler;

#[step_entity(name = "DESCRIPTIVE_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for DescriptiveRepresentationItemHandler {
    type WriteInput = DescriptiveItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_descriptive_representation_item(entity_id, attrs)?;
        lower::lower_descriptive_representation_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: DescriptiveItem) -> Result<u64, WriteError> {
        let early = lift::lift_descriptive_representation_item(item.name, item.description);
        Ok(serialize::serialize_descriptive_representation_item(
            buf, &early,
        ))
    }
}
