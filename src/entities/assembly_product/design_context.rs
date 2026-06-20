//! `DESIGN_CONTEXT` handler `assembly_product`. `AP203`
//! subtype of `PRODUCT_DEFINITION_CONTEXT` with a
//! `life_cycle_stage='design'` `WHERE` constraint; identical field
//! layout. Lands in the `product_definition_contexts` arena with
//! `kind = Design`.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContext;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_design_context(entity_id, attrs)?;
        lower::lower_design_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _pdc: ProductDefinitionContext) -> Result<u64, WriteError> {
        unreachable!("DESIGN_CONTEXT is emitted inline by emit_application_context")
    }
}
