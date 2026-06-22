//! `PRODUCT_DEFINITION_FORMATION` handler.
//!
//! 2-layer path: the generated `bind` extracts the three attrs, the shared
//! `lower` (also used by the `_WITH_SPECIFIED_SOURCE` sister) maps the
//! formation onto its `PRODUCT` and records the typed
//! `EarlyProductDefinitionFormationId` → `ProductId` correspondence. Writer
//! lifts the write input and emits via the generated `serialize`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Writer input for both formation handlers: the carrier body strings + the
/// emitted PRODUCT step ref. The synthesis (kernel/empty-arena) path passes
/// empty `id`/`description`; the reader-arena path passes the faithful values.
pub(crate) struct ProductDefinitionFormationWriteInput {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) prod_entity: u64,
}

pub(crate) struct ProductDefinitionFormationHandler;

#[step_entity(name = "PRODUCT_DEFINITION_FORMATION")]
impl SimpleEntityHandler for ProductDefinitionFormationHandler {
    type WriteInput = ProductDefinitionFormationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_formation(entity_id, attrs)?;
        lower::lower_product_definition_formation(
            ctx,
            entity_id,
            early.id,
            early.description,
            early.of_product,
            None,
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionFormationWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition_formation(input);
        Ok(serialize::serialize_product_definition_formation(
            buf, &early,
        ))
    }
}
