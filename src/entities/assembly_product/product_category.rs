//! `PRODUCT_CATEGORY` handler sub-pass a.
//!
//! Reader stores `(name, description)` in `pc_meta_map`; the relationship
//! pass joins this metadata with PRPC's product list to attach a
//! `ProductCategoryRoot` to each affected product. Writer emits a bare
//! `PRODUCT_CATEGORY(name, description)` line; the chain orchestrator
//! (`buffer/assembly.rs::emit_product_category_chain`) wires it together
//! with PRPC + PCR.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::shared::{optional_text, optional_text_attr};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRODUCT_CATEGORY")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = optional_text(attrs, 1, entity_id, "description")?;
        // Schema-faithful `product_categories` arena push — PC `Itself`
        // variant. Standalone PCs (no PCR connecting them to a PRPC)
        // round-trip through this entry.
        let pc_id = ctx
            .product_categories
            .push(crate::ir::assembly::ProductCategory::Itself(
                crate::ir::assembly::ProductCategoryData { name, description },
            ));
        ctx.pc_arena_map.insert(entity_id, pc_id);
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
