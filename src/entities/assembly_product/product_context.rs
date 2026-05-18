//! `PRODUCT_CONTEXT` handler — Pass 9-27 `assembly_product`. STEP
//! positional `(name, frame_of_reference, discipline_type)` per
//! `AP214e3` (inherits `application_context_element`). Lands in the
//! `product_contexts` arena with `kind = Plain`.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{ProductContext, ProductContextKind};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductContextHandler;

#[step_entity(name = "PRODUCT_CONTEXT", pass = Pass9AssemblyContext)]
impl SimpleEntityHandler for ProductContextHandler {
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
            "PRODUCT_CONTEXT",
            ProductContextKind::Plain,
        )
    }

    fn write(_buf: &mut WriteBuffer, _pc: ProductContext) -> Result<u64, WriteError> {
        unreachable!("PRODUCT_CONTEXT is emitted inline by emit_application_context")
    }
}

pub(crate) fn read_product_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
    kind: ProductContextKind,
) -> Result<(), ConvertError> {
    check_count(attrs, 3, entity_id, entity_name)?;
    let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
    let frame_ref = read_entity_ref(attrs, 1, entity_id, "frame_of_reference")?;
    let discipline_type = read_string(attrs, 2, entity_id, "discipline_type")?.to_owned();
    let Some(&frame_of_reference) = ctx.plm_application_context_id_map.get(&frame_ref) else {
        return Ok(()); // AC missing — drop (Pass9PlmAppContext runs before us)
    };
    let id = ctx.product_contexts.push(ProductContext {
        name,
        frame_of_reference,
        discipline_type,
        kind,
    });
    ctx.product_context_id_map.insert(entity_id, id);
    Ok(())
}
