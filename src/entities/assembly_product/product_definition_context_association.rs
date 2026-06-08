//! `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION` handler
//! `assembly_product`. STEP positional
//! `(definition, frame_of_reference, role)` per `AP214e3`.
//! `definition` refs `PRODUCT_DEFINITION` → resolved via
//! `pdef_to_product` to a `ProductId` (PDEF data lives on Product).

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContextAssociation;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION",
        )?;
        let def_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let frame_ref = read_entity_ref(attrs, 1, entity_id, "frame_of_reference")?;
        let role_ref = read_entity_ref(attrs, 2, entity_id, "role")?;
        let Some(&pdef_product_eid) = ctx.pdef_to_product.get(&def_ref) else {
            return Ok(());
        };
        let Some(&pid) = ctx.product_arena_map.get(&pdef_product_eid) else {
            return Ok(());
        };
        let Some(&pdcid) = ctx.product_definition_context_id_map.get(&frame_ref) else {
            return Ok(());
        };
        let Some(&roleid) = ctx.pdc_role_id_map.get(&role_ref) else {
            return Ok(());
        };
        let id =
            ctx.product_definition_context_associations
                .push(ProductDefinitionContextAssociation {
                    definition: pid,
                    frame_of_reference: pdcid,
                    role: roleid,
                });
        ctx.pdca_id_map.insert(entity_id, id);
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
