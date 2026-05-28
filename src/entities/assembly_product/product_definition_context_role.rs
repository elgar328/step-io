//! `PRODUCT_DEFINITION_CONTEXT_ROLE` handler — Pass 9-28 `assembly_product`.
//! STEP positional `(name, description)` per `AP214e3`. Leaf entity.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::ProductDefinitionContextRole;
use crate::ir::attr::{check_count, read_optional_string, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionContextRoleHandler;

#[step_entity(name = "PRODUCT_DEFINITION_CONTEXT_ROLE", pass = Pass9PdcRole)]
impl SimpleEntityHandler for ProductDefinitionContextRoleHandler {
    type WriteInput = ProductDefinitionContextRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRODUCT_DEFINITION_CONTEXT_ROLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_optional_string(attrs, 1, entity_id, "description")?;
        let id = ctx
            .product_definition_context_roles
            .push(ProductDefinitionContextRole { name, description });
        ctx.pdc_role_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _r: ProductDefinitionContextRole) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_DEFINITION_CONTEXT_ROLE is emitted inline by emit_pdca_cluster")
    }
}
