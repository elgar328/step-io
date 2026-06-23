//! `PRODUCT_RELATED_PRODUCT_CATEGORY` handler sub-pass a.
//!
//! PRPC carries the product side of the category chain: a `kind` label, an
//! optional description, and the list of products it applies to. Reader lowers
//! it into the `product_categories` arena (the canonical store); the writer
//! emits the chain from that arena (`emit_product_categories_arena`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_related_product_category(entity_id, attrs)?;
        lower::lower_product_related_product_category(ctx, entity_id, early);
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
        let early =
            lift::lift_product_related_product_category(kind, kind_description, product_refs);
        Ok(serialize::serialize_product_related_product_category(
            buf, &early,
        ))
    }
}
