//! `PRODUCT_CATEGORY_RELATIONSHIP` handler — Pass 6-1b sub-pass b.
//!
//! Reader resolves the PC and PRPC references out of `pc_meta_map` and
//! `prpc_meta_map`, then attaches a `ProductCategoryRoot` to every
//! product the PRPC referenced. Writer emits the bare PCR line with the
//! pair of entity refs.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{ProductCategoryRelationship, ProductCategoryRoot};
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

#[step_entity(name = "PRODUCT_CATEGORY_RELATIONSHIP", pass = Pass6ProductCategoryRel)]
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

        // Schema-faithful `product_category_relationships` arena push.
        // Both refs resolve through the new arena_maps populated by the
        // PC / PRPC handlers earlier in the same pass.
        if let (Some(&category_id), Some(&sub_category_id)) = (
            ctx.pc_arena_map.get(&pc_ref),
            ctx.prpc_arena_map.get(&prpc_ref),
        ) {
            ctx.product_category_relationships
                .push(ProductCategoryRelationship {
                    name,
                    description,
                    category: category_id,
                    sub_category: sub_category_id,
                });
        }

        let Some((pc_name, pc_description)) = ctx.pc_meta_map.get(&pc_ref).cloned() else {
            ctx.warnings.push(ConvertError::MissingReference {
                from: entity_id,
                to: pc_ref,
                field_name: "category",
            });
            return Ok(());
        };
        let Some((_, _, products)) = ctx.prpc_meta_map.get(&prpc_ref).cloned() else {
            ctx.warnings.push(ConvertError::MissingReference {
                from: entity_id,
                to: prpc_ref,
                field_name: "sub_category",
            });
            return Ok(());
        };

        for prod_ref in &products {
            let Some(&pid) = ctx.product_arena_map.get(prod_ref) else {
                continue;
            };
            if let Some(category) = ctx.assembly_products[pid].category.as_mut() {
                category.root = Some(ProductCategoryRoot {
                    name: pc_name.clone(),
                    description: pc_description.clone(),
                });
            }
        }
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
