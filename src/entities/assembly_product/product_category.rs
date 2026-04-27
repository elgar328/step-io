//! `PRODUCT_CATEGORY` handler — Pass 6-1b sub-pass a.
//!
//! Reader stores `(name, description)` in `pc_meta_map`; the relationship
//! pass joins this metadata with PRPC's product list to attach a
//! `ProductCategoryRoot` to each affected product. Writer emits a bare
//! `PRODUCT_CATEGORY(name, description)` line; the chain orchestrator
//! (`buffer/assembly.rs::emit_product_category_chain`) wires it together
//! with PRPC + PCR.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::shared::{optional_text, optional_text_attr};

pub(crate) struct ProductCategoryWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

pub(crate) struct ProductCategoryHandler;

impl SimpleEntityHandler for ProductCategoryHandler {
    const NAME: &'static str = "PRODUCT_CATEGORY";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6ProductCategory;
    type WriteInput = ProductCategoryWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRODUCT_CATEGORY")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let description = optional_text(attrs, 1, entity_id, "description")?;
        ctx.pc_meta_map.insert(entity_id, (name, description));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductCategoryWriteInput { name, description }: ProductCategoryWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_CATEGORY",
            vec![Attribute::String(name), optional_text_attr(description)],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PRODUCT_CATEGORY_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ProductCategoryHandler::NAME,
    pass_level: ProductCategoryHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ProductCategoryHandler::read,
    },
};
