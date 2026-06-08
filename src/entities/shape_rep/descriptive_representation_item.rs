//! `DESCRIPTIVE_REPRESENTATION_ITEM` handler (property-1).
//!
//! AP214: `DESCRIPTIVE_REPRESENTATION_ITEM SUBTYPE OF (representation_item)`
//! with own field `description: text`. STEP positional inherits the parent
//! `representation_item.name`, so the line is `(name, description)`.
//!
//! Reader stores the resolved [`DescriptiveItem`] keyed by STEP entity id;
//! the PDR pass collects these into the parent `Property.items` alongside
//! `MEASURE_REPRESENTATION_ITEM` entries.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DescriptiveItem;
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DESCRIPTIVE_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        ctx.descriptive_item_map
            .insert(entity_id, DescriptiveItem { name, description });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: DescriptiveItem) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "DESCRIPTIVE_REPRESENTATION_ITEM",
            vec![
                Attribute::String(item.name),
                Attribute::String(item.description),
            ],
        ))
    }
}
