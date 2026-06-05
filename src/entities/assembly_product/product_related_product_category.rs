//! `PRODUCT_RELATED_PRODUCT_CATEGORY` handler sub-pass a.
//!
//! PRPC carries the product side of the category chain: a `kind` label, an
//! optional description, and the list of products it applies to. Reader
//! attaches a half-filled `ProductCategoryChain` (without `root`) to each
//! referenced product immediately so the optional supertype side can fill
//! in `root` later, even when no PCR exists.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::shared::{optional_text, optional_text_attr};
use step_io_macros::step_entity;

pub(crate) struct ProductRelatedProductCategoryWriteInput {
    pub(crate) kind: String,
    pub(crate) kind_description: Option<String>,
    pub(crate) product_refs: Vec<u64>,
}

pub(crate) struct ProductRelatedProductCategoryHandler;

#[step_entity(name = "PRODUCT_RELATED_PRODUCT_CATEGORY")]
impl SimpleEntityHandler for ProductRelatedProductCategoryHandler {
    type WriteInput = ProductRelatedProductCategoryWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PRODUCT_RELATED_PRODUCT_CATEGORY")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = optional_text(attrs, 1, entity_id, "description")?;
        let product_refs = read_entity_ref_list(attrs, 2, entity_id, "products")?;

        // [NS-empty-prrpc] CATIA / Autodesk: products is SET[1:?] but an empty
        // `()` relates no products → drop as a normalization and record the id
        // so the referencing PRODUCT_CATEGORY_RELATIONSHIP cascades
        // (NS-empty-prrpc-cascade). See reader::nonstandard.
        if product_refs.is_empty() {
            ctx.warnings.push(ConvertError::NonStandardInput {
                field: "PRODUCT_RELATED_PRODUCT_CATEGORY".into(),
                count: 1,
                normalized_to: "dropped (empty products, non-standard SET[1:?])".into(),
            });
            ctx.empty_prrpc_refs.insert(entity_id);
            return Ok(());
        }

        // Schema-faithful `product_categories` arena push — PRPC variant.
        // Resolve every product ref into a ProductId so the arena entry
        // carries the typed reference. Unknown refs (cross-file or
        // dropped) are silently filtered.
        let mut resolved_products: Vec<crate::ir::ProductId> =
            Vec::with_capacity(product_refs.len());
        for prod_ref in &product_refs {
            if let Some(&pid) = ctx.product_arena_map.get(prod_ref) {
                resolved_products.push(pid);
            }
        }
        let pc_id = ctx.product_categories.push(
            crate::ir::assembly::ProductCategory::ProductRelatedProductCategory(
                crate::ir::assembly::ProductRelatedProductCategoryData {
                    name: name.clone(),
                    description: description.clone(),
                    products: resolved_products,
                },
            ),
        );
        ctx.prpc_arena_map.insert(entity_id, pc_id);

        let _ = (name, description, product_refs);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductRelatedProductCategoryWriteInput {
            kind,
            kind_description,
            product_refs,
        }: ProductRelatedProductCategoryWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_RELATED_PRODUCT_CATEGORY",
            vec![
                Attribute::String(kind),
                optional_text_attr(kind_description),
                Attribute::List(product_refs.into_iter().map(Attribute::EntityRef).collect()),
            ],
        ))
    }
}
