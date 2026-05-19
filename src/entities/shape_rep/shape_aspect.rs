//! `SHAPE_ASPECT` handler — Pass 8-pre.
//!
//! Resolves `SHAPE_ASPECT.of_shape` through the assembly pass's
//! `pdef_shape_to_pdef` and `pdef_to_product` maps to a `ProductId`.
//! Future PMI work (Tolerance / Datum / GD&T per ROADMAP Phase 2) hangs
//! additional handlers off the same group.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ShapeAspect;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeAspectWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

pub(crate) struct ShapeAspectHandler;

#[step_entity(name = "SHAPE_ASPECT", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for ShapeAspectHandler {
    type WriteInput = ShapeAspectWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
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

        let id = ctx.shape_aspects.push(ShapeAspect {
            name,
            description,
            target: product_id,
            product_definitional,
        });
        ctx.shape_aspect_id_map.insert(entity_id, id);
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
