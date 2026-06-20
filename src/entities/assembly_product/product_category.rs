//! `PRODUCT_CATEGORY` handler — assembly-product domain (2-layer path).
//! `Itself` carrier; standalone PCs (no PCR linking them to a PRPC)
//! round-trip through this entry.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductCategoryWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

pub(crate) struct ProductCategoryHandler;

#[step_entity(name = "PRODUCT_CATEGORY")]
impl SimpleEntityHandler for ProductCategoryHandler {
    type WriteInput = ProductCategoryWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_category(entity_id, attrs)?;
        lower::lower_product_category(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductCategoryWriteInput { name, description }: ProductCategoryWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_category(name, description);
        Ok(serialize::serialize_product_category(buf, &early))
    }
}
