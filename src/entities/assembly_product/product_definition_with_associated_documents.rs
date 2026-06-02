//! `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` handler — Pass 6-3.
//!
//! Standard STEP subtype of `PRODUCT_DEFINITION` (AP203 / AP242) that
//! tags the definition with a list of documentation refs. The reader
//! treats the entity exactly like the base type — reusing
//! [`super::product_definition::read_product_definition_body`] — and
//! ignores the extra `documentation_ids` attribute. Writer support is
//! out of scope (round-trip downgrades to plain `PRODUCT_DEFINITION`).

use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::product_definition::{ProductDefinitionWriteInput, read_product_definition_body};
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionWithAssociatedDocumentsHandler;

#[step_entity(name = "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS")]
impl SimpleEntityHandler for ProductDefinitionWithAssociatedDocumentsHandler {
    type WriteInput = ProductDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_body(ctx, entity_id, attrs)
    }

    fn write(
        _buf: &mut WriteBuffer,
        _input: ProductDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        // Writer downgrades to plain PRODUCT_DEFINITION: the IR carries no
        // documentation_ids loyalty field today, so re-emitting the subtype
        // would require synthesising fake document refs. Round-trip is
        // semantically faithful at the PRODUCT_DEFINITION level — the
        // entity-type rename is recorded in reference-check's added/missing
        // type diff. Loyalty for the subtype is a separate plan.
        unreachable!(
            "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS is read-only; \
             writer emits plain PRODUCT_DEFINITION via ProductDefinitionHandler"
        )
    }
}
