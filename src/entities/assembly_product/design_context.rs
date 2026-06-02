//! `DESIGN_CONTEXT` handler `assembly_product`. `AP203`
//! subtype of `PRODUCT_DEFINITION_CONTEXT` with a
//! `life_cycle_stage='design'` `WHERE` constraint; identical field
//! layout. Lands in the `product_definition_contexts` arena with
//! `kind = Design`.

use crate::entities::SimpleEntityHandler;
use crate::entities::assembly_product::product_definition_context::read_product_definition_context;
use crate::ir::assembly::{ProductDefinitionContext, ProductDefinitionContextKind};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DesignContextHandler;

#[step_entity(name = "DESIGN_CONTEXT")]
impl SimpleEntityHandler for DesignContextHandler {
    type WriteInput = ProductDefinitionContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_context(
            ctx,
            entity_id,
            attrs,
            "DESIGN_CONTEXT",
            ProductDefinitionContextKind::Design,
        )
    }

    fn write(_buf: &mut WriteBuffer, _pdc: ProductDefinitionContext) -> Result<u64, WriteError> {
        unreachable!("DESIGN_CONTEXT is emitted inline by emit_application_context")
    }
}
