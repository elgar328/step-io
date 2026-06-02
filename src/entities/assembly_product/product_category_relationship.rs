//! `PRODUCT_CATEGORY_RELATIONSHIP` handler.
//!
//! Reader pushes the relationship into the schema-faithful
//! `product_category_relationships` arena, resolving the PC and PRPC
//! refs through arena maps the `PRODUCT_CATEGORY` and
//! `PRODUCT_RELATED_PRODUCT_CATEGORY` handlers fill upstream.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductCategoryRelationship;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductCategoryRelationshipWriteInput {
    pub(crate) pc_ref: u64,
    pub(crate) prpc_ref: u64,
}

pub(crate) struct ProductCategoryRelationshipHandler;

#[step_entity(name = "PRODUCT_CATEGORY_RELATIONSHIP")]
impl SimpleEntityHandler for ProductCategoryRelationshipHandler {
    type WriteInput = ProductCategoryRelationshipWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "PRODUCT_CATEGORY_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let pc_ref = read_entity_ref(attrs, 2, entity_id, "category")?;
        let prpc_ref = read_entity_ref(attrs, 3, entity_id, "sub_category")?;

        let (Some(&category_id), Some(&sub_category_id)) = (
            ctx.pc_arena_map.get(&pc_ref),
            ctx.prpc_arena_map.get(&prpc_ref),
        ) else {
            ctx.warnings.push(ConvertError::MissingReference {
                from: entity_id,
                to: if ctx.pc_arena_map.contains_key(&pc_ref) {
                    prpc_ref
                } else {
                    pc_ref
                },
                field_name: "category",
            });
            return Ok(());
        };
        ctx.product_category_relationships
            .push(ProductCategoryRelationship {
                name,
                description,
                category: category_id,
                sub_category: sub_category_id,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductCategoryRelationshipWriteInput { pc_ref, prpc_ref }: ProductCategoryRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_CATEGORY_RELATIONSHIP",
            vec![
                Attribute::String(" ".into()),
                Attribute::String(" ".into()),
                Attribute::EntityRef(pc_ref),
                Attribute::EntityRef(prpc_ref),
            ],
        ))
    }
}
