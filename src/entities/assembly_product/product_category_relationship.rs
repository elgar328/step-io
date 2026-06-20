//! `PRODUCT_CATEGORY_RELATIONSHIP` handler.
//!
//! Reader pushes the relationship into the schema-faithful
//! `product_category_relationships` arena, resolving the PC and PRPC
//! refs through arena maps the `PRODUCT_CATEGORY` and
//! `PRODUCT_RELATED_PRODUCT_CATEGORY` handlers fill upstream.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductCategoryRelationshipWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_category_relationship(entity_id, attrs)?;
        lower::lower_product_category_relationship(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductCategoryRelationshipWriteInput {
            name,
            description,
            pc_ref,
            prpc_ref,
        }: ProductCategoryRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_category_relationship(name, description, pc_ref, prpc_ref);
        Ok(serialize::serialize_product_category_relationship(
            buf, &early,
        ))
    }
}
