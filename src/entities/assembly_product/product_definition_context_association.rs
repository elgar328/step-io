//! `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION` handler `assembly_product` —
//! 2-layer path. STEP positional `(definition, frame_of_reference, role)` per
//! `AP214e3`. `definition` refs `PRODUCT_DEFINITION` → resolved via
//! `product_of_pdef` to a `ProductId` (PDEF data lives on Product). Emitted by
//! `emit_pdca_cluster` (lift + generated serialize), so `write` is unreachable.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContextAssociation;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionContextAssociationHandler;

#[step_entity(name = "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION")]
impl SimpleEntityHandler for ProductDefinitionContextAssociationHandler {
    type WriteInput = ProductDefinitionContextAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_context_association(entity_id, attrs)?;
        lower::lower_product_definition_context_association(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        _buf: &mut WriteBuffer,
        _a: ProductDefinitionContextAssociation,
    ) -> Result<u64, WriteError> {
        unreachable!(
            "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION is emitted inline by emit_pdca_cluster"
        )
    }
}
