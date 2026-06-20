//! `PRODUCT_DEFINITION_CONTEXT_ROLE` handler `assembly_product` — 2-layer path.
//! STEP positional `(name, description)` per `AP214e3`. Leaf entity; emitted by
//! `emit_pdca_cluster` (lift + generated serialize), so `write` is unreachable.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContextRole;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionContextRoleHandler;

#[step_entity(name = "PRODUCT_DEFINITION_CONTEXT_ROLE")]
impl SimpleEntityHandler for ProductDefinitionContextRoleHandler {
    type WriteInput = ProductDefinitionContextRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_context_role(entity_id, attrs)?;
        lower::lower_product_definition_context_role(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _r: ProductDefinitionContextRole) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_DEFINITION_CONTEXT_ROLE is emitted inline by emit_pdca_cluster")
    }
}
