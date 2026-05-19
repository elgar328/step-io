//! `DESCRIPTION_ATTRIBUTE` handler — Pass 9-25b.
//!
//! `(attribute_value, described_item)` — SELECT target. Initial coverage:
//! `person_and_organization` only. Other variants
//! (`representation`, `application_context`, `approval_role`, …) are
//! silently dropped with a warning; future phases expand
//! [`DescriptionAttributeItem`](crate::ir::DescriptionAttributeItem).

use crate::entities::SimpleEntityHandler;
use crate::ir::PropertyPool;
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::property::{DescriptionAttribute, DescriptionAttributeItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DescriptionAttributeWriteInput {
    pub(crate) attr: DescriptionAttribute,
    pub(crate) item_step: u64,
}

pub(crate) struct DescriptionAttributeHandler;

#[step_entity(name = "DESCRIPTION_ATTRIBUTE", pass = Pass9PlmAttributes)]
impl SimpleEntityHandler for DescriptionAttributeHandler {
    type WriteInput = DescriptionAttributeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DESCRIPTION_ATTRIBUTE")?;
        let attribute_value = read_string(attrs, 0, entity_id, "attribute_value")?.to_owned();
        let item_ref = read_entity_ref(attrs, 1, entity_id, "described_item")?;

        let described_item = if let Some(&pao_id) = ctx.plm_p_and_o_id_map.get(&item_ref) {
            DescriptionAttributeItem::PersonAndOrganization(pao_id)
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "DESCRIPTION_ATTRIBUTE.described_item #{item_ref} target type unsupported"
                ),
            });
            return Ok(());
        };

        let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
        pool.description_attributes.push(DescriptionAttribute {
            attribute_value,
            described_item,
        });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        DescriptionAttributeWriteInput { attr, item_step }: DescriptionAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "DESCRIPTION_ATTRIBUTE",
            vec![
                Attribute::String(attr.attribute_value),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}
