//! `SHAPE_ASPECT` handler.
//!
//! Resolves `SHAPE_ASPECT.of_shape` to a `ProductId` through the typed
//! `product_of_pds` probe (recorded by the PDS lower).
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

#[step_entity(name = "SHAPE_ASPECT")]
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

        // Lookup: SHAPE_ASPECT.of_shape → PRODUCT_DEFINITION_SHAPE →
        // ProductId (typed one-probe).
        // [NS-shape-aspect-of-shape-pd] C3D: of_shape references a
        // PRODUCT_DEFINITION directly (must be a PRODUCT_DEFINITION_SHAPE) →
        // resolve via the product; writer re-emits the standard PDS form.
        // See reader::nonstandard.
        let product_id = if let Some(pid) = ctx.product_of_pds(of_shape_ref) {
            pid
        } else if ctx
            .id_cache
            .get::<crate::ir::ProductDefinitionId>(of_shape_ref)
            .is_some()
        {
            ctx.warnings.push(ConvertError::NonStandardInput {
                field: "SHAPE_ASPECT.of_shape".into(),
                count: 1,
                normalized_to: "PRODUCT_DEFINITION_SHAPE (of_shape was a PRODUCT_DEFINITION)"
                    .into(),
            });
            let Some(pid) = ctx.product_of_pdef(of_shape_ref) else {
                return Ok(());
            };
            pid
        } else {
            return Ok(()); // unresolved (genuinely non-product target)
        };

        let id = ctx.shape_aspects.push(ShapeAspect {
            name,
            description,
            target: product_id,
            product_definitional,
        });
        ctx.id_cache.insert(entity_id, id);
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
