//! `DEFAULT_MODEL_GEOMETRIC_VIEW` handler.
//!
//! `SUBTYPE OF (model_geometric_view, shape_aspect)` — a flat 8-attribute
//! simple entity combining the two supertypes:
//! `(co.name, co.description, item, rep, sa.name, sa.description, of_shape,
//! product_definitional*)`. `item` is a `CAMERA_MODEL`, `rep` a
//! `DRAUGHTING_MODEL`, `of_shape` a `PRODUCT_DEFINITION_SHAPE` (resolved to a
//! product like `SHAPE_ASPECT`). `product_definitional` is DERIVE `false` (`*`).
//! A leaf (nothing references it), so it just round-trips through its own arena.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
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
        let early = bind::bind_default_model_geometric_view(entity_id, attrs)?;
        lower::lower_default_model_geometric_view(ctx, &early);
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
        Ok(serialize::serialize_default_model_geometric_view(
            buf,
            &lift::lift_default_model_geometric_view(
                view.co_name,
                view.co_description,
                item_step,
                rep_step,
                view.sa_name,
                view.sa_description,
                of_shape_step,
            ),
        ))
    }
}
