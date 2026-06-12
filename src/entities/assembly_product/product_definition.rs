//! `PRODUCT_DEFINITION` handler.
//!
//! 2-layer path: the generated `bind` extracts the four attrs, the shared
//! `lower` (also used by the `_WITH_ASSOCIATED_DOCUMENTS` subtype) follows
//! the formation onto its product, stores the canonical arena entry, and
//! records the typed `EarlyProductDefinitionId` → `ProductId` correspondence.
//! Writer emits the bare PDEF line linking a formation and the per-file
//! `PRODUCT_DEFINITION_CONTEXT`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionWriteInput {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) formation: u64,
    pub(crate) pdef_ctx: u64,
}

pub(crate) struct ProductDefinitionHandler;

#[step_entity(name = "PRODUCT_DEFINITION")]
impl SimpleEntityHandler for ProductDefinitionHandler {
    type WriteInput = ProductDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition(entity_id, attrs)?;
        lower::lower_product_definition(
            ctx,
            entity_id,
            early.id,
            early.description,
            early.formation,
            early.frame_of_reference,
            None,
        )
    }

    fn write(buf: &mut WriteBuffer, input: ProductDefinitionWriteInput) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition(input);
        Ok(serialize::serialize_product_definition(buf, &early))
    }
}
