//! `PRODUCT_RELATED_PRODUCT_CATEGORY` handler — Pass 6-1b sub-pass a.
//!
//! PRPC carries the product side of the category chain: a `kind` label, an
//! optional description, and the list of products it applies to. Reader
//! attaches a half-filled `ProductCategoryChain` (without `root`) to each
//! referenced product immediately so the optional supertype side can fill
//! in `root` later, even when no PCR exists.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::ProductCategoryChain;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::shared::{optional_text, optional_text_attr};

pub(crate) struct ProductRelatedProductCategoryWriteInput {
    pub(crate) kind: String,
    pub(crate) kind_description: Option<String>,
    pub(crate) product_refs: Vec<u64>,
}

pub(crate) struct ProductRelatedProductCategoryHandler;

impl SimpleEntityHandler for ProductRelatedProductCategoryHandler {
    const NAME: &'static str = "PRODUCT_RELATED_PRODUCT_CATEGORY";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6ProductCategory;
    type WriteInput = ProductRelatedProductCategoryWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PRODUCT_RELATED_PRODUCT_CATEGORY")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let description = optional_text(attrs, 1, entity_id, "description")?;
        let product_refs = read_entity_ref_list(attrs, 2, entity_id, "products")?;

        // Attach the PRPC half (kind / kind_description) to each referenced
        // product immediately. The PCR pass will fill in `root` if a PCR
        // entity links this PRPC to a PC.
        for prod_ref in &product_refs {
            if let Some(&pid) = ctx.product_arena_map.get(prod_ref) {
                ctx.assembly_products[pid].category = Some(ProductCategoryChain {
                    kind: name.clone(),
                    kind_description: description.clone(),
                    root: None,
                });
            }
        }
        ctx.prpc_meta_map
            .insert(entity_id, (name, description, product_refs));
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PRPC_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ProductRelatedProductCategoryHandler::NAME,
    pass_level: ProductRelatedProductCategoryHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ProductRelatedProductCategoryHandler::read,
    },
};
