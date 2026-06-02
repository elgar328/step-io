//! `PRODUCT_DEFINITION_CONTEXT` handler — Pass 9-27 `assembly_product`.
//! STEP positional `(name, frame_of_reference, life_cycle_stage)` per
//! `AP214e3` (inherits `application_context_element`). Lands in the
//! `product_definition_contexts` arena with `kind = Plain`.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{ProductDefinitionContext, ProductDefinitionContextKind};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
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
        read_product_definition_context(
            ctx,
            entity_id,
            attrs,
            "PRODUCT_DEFINITION_CONTEXT",
            ProductDefinitionContextKind::Plain,
        )
    }

    fn write(_buf: &mut WriteBuffer, _pdc: ProductDefinitionContext) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_DEFINITION_CONTEXT is emitted inline by emit_application_context")
    }
}

pub(crate) fn read_product_definition_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
    kind: ProductDefinitionContextKind,
) -> Result<(), ConvertError> {
    check_count(attrs, 3, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let frame_ref = read_entity_ref(attrs, 1, entity_id, "frame_of_reference")?;
    let life_cycle_stage =
        read_string_or_unset(attrs, 2, entity_id, "life_cycle_stage")?.to_owned();
    let Some(&frame_of_reference) = ctx.plm_application_context_id_map.get(&frame_ref) else {
        return Ok(());
    };
    let id = ctx
        .product_definition_contexts
        .push(ProductDefinitionContext {
            name,
            frame_of_reference,
            life_cycle_stage,
            kind,
        });
    ctx.product_definition_context_id_map.insert(entity_id, id);
    Ok(())
}
