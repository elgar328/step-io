//! `PRODUCT_DEFINITION_CONTEXT` handler `assembly_product` — 2-layer path.
//! STEP positional `(name, frame_of_reference, life_cycle_stage)` per
//! `AP214e3` (inherits `application_context_element`). Lands in the
//! `product_definition_contexts` arena as `ProductDefinitionContext::Itself`.
//! Emitted by `emit_application_context` (lift + generated serialize), so the
//! handler `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContext;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionContextHandler;

#[step_entity(name = "PRODUCT_DEFINITION_CONTEXT")]
impl SimpleEntityHandler for ProductDefinitionContextHandler {
    type WriteInput = ProductDefinitionContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_context(entity_id, attrs)?;
        lower::lower_product_definition_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _pdc: ProductDefinitionContext) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_DEFINITION_CONTEXT is emitted inline by emit_application_context")
    }
}
