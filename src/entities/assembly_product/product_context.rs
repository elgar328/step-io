//! `PRODUCT_CONTEXT` handler `assembly_product` — 2-layer path. STEP
//! positional `(name, frame_of_reference, discipline_type)` per `AP214e3`
//! (inherits `application_context_element`). Lands in the `product_contexts`
//! arena as `ProductContext::Itself`. Emitted by `emit_application_context`
//! (which calls `lift_product_context` + the generated serialize), so the
//! handler `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductContext;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductContextHandler;

#[step_entity(name = "PRODUCT_CONTEXT")]
impl SimpleEntityHandler for ProductContextHandler {
    type WriteInput = ProductContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_context(entity_id, attrs)?;
        lower::lower_product_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _pc: ProductContext) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_CONTEXT is emitted inline by emit_application_context")
    }
}
