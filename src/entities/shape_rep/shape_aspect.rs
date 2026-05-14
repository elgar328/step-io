//! `SHAPE_ASPECT` handler — Pass 8-pre.
//!
//! Resolves `SHAPE_ASPECT.of_shape` through the assembly pass's
//! `pdef_shape_to_pdef` and `pdef_to_product` maps to a `ProductId`.
//! Future PMI work (Tolerance / Datum / GD&T per ROADMAP Phase 2) hangs
//! additional handlers off the same group.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ShapeAspect;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ShapeAspectWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

pub(crate) struct ShapeAspectHandler;

impl SimpleEntityHandler for ShapeAspectHandler {
    const NAME: &'static str = "SHAPE_ASPECT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass8ShapeAspect;
    type WriteInput = ShapeAspectWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_ASPECT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;

        // Lookup chain: SHAPE_ASPECT.of_shape → PRODUCT_DEFINITION_SHAPE
        //   → PRODUCT_DEFINITION → ProductId
        let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(()); // unresolved (rare — non-PDS targets)
        };
        let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
            return Ok(());
        };
        let Some(&product_id) = ctx.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };

        ctx.shape_aspects.push(ShapeAspect {
            name,
            description,
            target: product_id,
            product_definitional,
        });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeAspectWriteInput {
            name,
            description,
            pds_step_id,
            product_definitional,
        }: ShapeAspectWriteInput,
    ) -> Result<u64, WriteError> {
        let bool_attr = if product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "SHAPE_ASPECT",
            vec![
                Attribute::String(name),
                Attribute::String(description),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SHAPE_ASPECT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ShapeAspectHandler::NAME,
    pass_level: ShapeAspectHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ShapeAspectHandler::read,
    },
};
