//! `PRODUCT_DEFINITION` handler — Pass 6-3.
//!
//! Reader follows the formation ref out of `formation_to_product` to learn
//! which product the PDEF describes, and stores the result in
//! `pdef_to_product`. Writer emits the bare PDEF line linking a formation
//! and the per-file `PRODUCT_DEFINITION_CONTEXT`. The
//! `_WITH_ASSOCIATED_DOCUMENTS` subtype reuses the same reader body via
//! [`read_product_definition_body`].

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Shared reader body for `PRODUCT_DEFINITION` and its
/// `_WITH_ASSOCIATED_DOCUMENTS` subtype. Both share the first four attrs;
/// the subtype's extra `documentation_ids` (attr[4]) is ignored at read.
/// Caller's only responsibility is dispatching the right entity name —
/// the body validates attribute count as a *minimum* (`>= 4`).
pub(crate) fn read_product_definition_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<(), ConvertError> {
    if attrs.len() < 4 {
        return Err(ConvertError::AttributeCount {
            entity_id,
            entity_name: "PRODUCT_DEFINITION".into(),
            expected: 4,
            actual: attrs.len(),
        });
    }
    // PRODUCT_DEFINITION.id is a fixed-value enum-like string ("design" /
    // "implementation"); keep strict. description is informal — accept `$`.
    let _id = read_string_or_unset(attrs, 0, entity_id, "id")?;
    let _description = read_string_or_unset(attrs, 1, entity_id, "description")?;
    let formation_ref = read_entity_ref(attrs, 2, entity_id, "formation")?;
    // attrs[3] = frame_of_reference (PRODUCT_DEFINITION_CONTEXT).
    let pdc_step_ref = read_entity_ref(attrs, 3, entity_id, "frame_of_reference").ok();

    let Some(&product_ref) = ctx.formation_to_product.get(&formation_ref) else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: formation_ref,
            field_name: "formation",
        });
    };
    ctx.pdef_to_product.insert(entity_id, product_ref);
    if let Some(pdc_ref) = pdc_step_ref {
        if let Some(&pid) = ctx.product_arena_map.get(&product_ref) {
            ctx.product_pdc_step_refs.insert(pid, pdc_ref);
        }
    }
    Ok(())
}

pub(crate) struct ProductDefinitionWriteInput {
    pub(crate) formation: u64,
    pub(crate) pdef_ctx: u64,
}

pub(crate) struct ProductDefinitionHandler;

#[step_entity(name = "PRODUCT_DEFINITION")]
impl SimpleEntityHandler for ProductDefinitionHandler {
    type WriteInput = ProductDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_body(ctx, entity_id, attrs)
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionWriteInput {
            formation,
            pdef_ctx,
        }: ProductDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION",
            vec![
                Attribute::String("design".into()),
                Attribute::String(String::new()),
                Attribute::EntityRef(formation),
                Attribute::EntityRef(pdef_ctx),
            ],
        ))
    }
}
