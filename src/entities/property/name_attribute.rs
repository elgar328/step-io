//! `NAME_ATTRIBUTE` handler — Pass 9-25b.
//!
//! `(attribute_value, named_item)` — SELECT target. Initial coverage:
//! `product_definition` (via `pdef_to_product` → `product_arena_map`) and
//! `derived_unit` (via `derived_unit_id_map`). Other variants of the
//! schema SELECT (`address`, `effectivity`, `CDSR`, etc.) are silently
//! dropped with a warning; future phases expand
//! [`NameAttributeItem`](crate::ir::NameAttributeItem).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{NameAttribute, NameAttributeItem};
use crate::ir::{ProductId, PropertyPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "NAME_ATTRIBUTE")?;
        let attribute_value =
            read_string_or_unset(attrs, 0, entity_id, "attribute_value")?.to_owned();
        let item_ref = read_entity_ref(attrs, 1, entity_id, "named_item")?;

        let named_item = if let Some(product_id) = resolve_product_definition(ctx, item_ref) {
            NameAttributeItem::ProductDefinition(product_id)
        } else if let Some(&du_id) = ctx.derived_unit_id_map.get(&item_ref) {
            NameAttributeItem::DerivedUnit(du_id)
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("NAME_ATTRIBUTE.named_item #{item_ref} target type unsupported"),
            });
            return Ok(());
        };

        let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
        pool.name_attributes.push(NameAttribute {
            attribute_value,
            named_item,
        });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        NameAttributeWriteInput { attr, item_step }: NameAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "NAME_ATTRIBUTE",
            vec![
                Attribute::String(attr.attribute_value),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}

fn resolve_product_definition(ctx: &ReaderContext, item_ref: u64) -> Option<ProductId> {
    let product_step = ctx.pdef_to_product.get(&item_ref).copied()?;
    ctx.product_arena_map.get(&product_step).copied()
}
