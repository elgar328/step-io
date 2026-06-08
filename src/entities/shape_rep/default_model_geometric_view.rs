//! `DEFAULT_MODEL_GEOMETRIC_VIEW` handler.
//!
//! `SUBTYPE OF (model_geometric_view, shape_aspect)` — a flat 8-attribute
//! simple entity combining the two supertypes:
//! `(co.name, co.description, item, rep, sa.name, sa.description, of_shape,
//! product_definitional*)`. `item` is a `CAMERA_MODEL`, `rep` a
//! `DRAUGHTING_MODEL`, `of_shape` a `PRODUCT_DEFINITION_SHAPE` (resolved to a
//! product like `SHAPE_ASPECT`). `product_definitional` is DERIVE `false` (`*`).
//! A leaf (nothing references it), so it just round-trips through its own arena.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DefaultModelGeometricView;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DefaultModelGeometricViewWriteInput {
    pub(crate) view: DefaultModelGeometricView,
    pub(crate) of_shape_step: u64,
}

pub(crate) struct DefaultModelGeometricViewHandler;

#[step_entity(name = "DEFAULT_MODEL_GEOMETRIC_VIEW")]
impl SimpleEntityHandler for DefaultModelGeometricViewHandler {
    type WriteInput = DefaultModelGeometricViewWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 8, entity_id, "DEFAULT_MODEL_GEOMETRIC_VIEW")?;
        let co_name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let co_description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let rep_ref = read_entity_ref(attrs, 3, entity_id, "rep")?;
        let sa_name = read_string_or_unset(attrs, 4, entity_id, "name")?.to_owned();
        let sa_description = read_string_or_unset(attrs, 5, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 6, entity_id, "of_shape")?;
        // attr[7] product_definitional is DERIVE `*` — not read.

        let Some(&item) = ctx.viz_camera_model_id_map.get(&item_ref) else {
            return Ok(());
        };
        let Some(&rep) = ctx.repr_id_map.get(&rep_ref) else {
            return Ok(());
        };
        // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId
        // (the SHAPE_ASPECT chain).
        let Some(&pdef_step) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(());
        };
        let Some(&product_step) = ctx.pdef_to_product.get(&pdef_step) else {
            return Ok(());
        };
        let Some(&target) = ctx.product_arena_map.get(&product_step) else {
            return Ok(());
        };

        ctx.default_model_geometric_views
            .push(DefaultModelGeometricView {
                co_name,
                co_description,
                sa_name,
                sa_description,
                item,
                rep,
                target,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        DefaultModelGeometricViewWriteInput {
            view,
            of_shape_step,
        }: DefaultModelGeometricViewWriteInput,
    ) -> Result<u64, WriteError> {
        let item_step = buf.step_id(view.item);
        let rep_step = buf.step_id(view.rep);
        Ok(buf.push_simple(
            "DEFAULT_MODEL_GEOMETRIC_VIEW",
            vec![
                Attribute::String(view.co_name),
                Attribute::String(view.co_description),
                Attribute::EntityRef(item_step),
                Attribute::EntityRef(rep_step),
                Attribute::String(view.sa_name),
                Attribute::String(view.sa_description),
                Attribute::EntityRef(of_shape_step),
                Attribute::Derived,
            ],
        ))
    }
}
