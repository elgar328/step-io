//! `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` handler.
//!
//! Standard STEP subtype of `PRODUCT_DEFINITION` (AP203 / AP242) that tags
//! the definition with a list of documentation refs. 2-layer path: the
//! generated `bind` extracts the base attrs + `documentation_ids`, the shared
//! `lower` resolves the docs onto the canonical entry and the product's
//! `associated_documents` loyalty field, so the writer re-emits this subtype
//! (via `emit_pdef`) instead of downgrading to plain `PRODUCT_DEFINITION`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Writer input for the subtype: the same formation / context refs as a plain
/// `PRODUCT_DEFINITION` plus the resolved `DOCUMENT` step ids for
/// `documentation_ids`.
pub(crate) struct ProductDefinitionWithAssociatedDocumentsWriteInput {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) formation: u64,
    pub(crate) pdef_ctx: u64,
    pub(crate) documentation: Vec<u64>,
}

pub(crate) struct ProductDefinitionWithAssociatedDocumentsHandler;

#[step_entity(name = "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS")]
impl SimpleEntityHandler for ProductDefinitionWithAssociatedDocumentsHandler {
    type WriteInput = ProductDefinitionWithAssociatedDocumentsWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_with_associated_documents(entity_id, attrs)?;
        lower::lower_product_definition(
            ctx,
            entity_id,
            early.id,
            early.description,
            early.formation,
            early.frame_of_reference,
            Some(early.documentation_ids),
        )
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionWithAssociatedDocumentsWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition_with_associated_documents(input);
        Ok(serialize::serialize_product_definition_with_associated_documents(buf, &early))
    }
}
