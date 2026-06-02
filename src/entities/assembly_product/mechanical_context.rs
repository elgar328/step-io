//! `MECHANICAL_CONTEXT` handler `assembly_product`.
//! `AP203` subtype of `PRODUCT_CONTEXT` with a
//! `discipline_type='mechanical'` `WHERE` constraint; identical
//! field layout. Lands in the `product_contexts` arena with
//! `kind = Mechanical`.

use crate::entities::SimpleEntityHandler;
use crate::entities::assembly_product::product_context::read_product_context;
use crate::ir::assembly::{ProductContext, ProductContextKind};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MechanicalContextHandler;

#[step_entity(name = "MECHANICAL_CONTEXT")]
impl SimpleEntityHandler for MechanicalContextHandler {
    type WriteInput = ProductContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_context(
            ctx,
            entity_id,
            attrs,
            "MECHANICAL_CONTEXT",
            ProductContextKind::Mechanical,
        )
    }

    fn write(_buf: &mut WriteBuffer, _pc: ProductContext) -> Result<u64, WriteError> {
        unreachable!("MECHANICAL_CONTEXT is emitted inline by emit_application_context")
    }
}
