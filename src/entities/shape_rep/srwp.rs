//! `SHAPE_REPRESENTATION_WITH_PARAMETERS` handler — phase srwp.
//!
//! `shape_representation` SUBTYPE that narrows `items` to a SELECT of
//! `direction` / `placement` / `descriptive_representation_item` /
//! `measure_representation_item` (the last resolved through the
//! `representation_item` arena via `repr_item_id_map`). Emit delayed
//! (Mdgpr / DM / TSR / CGR pattern) — the pre-pass skips this variant
//! and `emit_shape_representation_with_parameters` writes into the
//! `representation_step_ids` slot by `RepresentationId`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ShapeRepresentationWithParameters;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeRepresentationWithParametersHandler;

#[step_entity(name = "SHAPE_REPRESENTATION_WITH_PARAMETERS")]
impl SimpleEntityHandler for ShapeRepresentationWithParametersHandler {
    type WriteInput = ShapeRepresentationWithParameters;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_representation_with_parameters(entity_id, attrs)?;
        lower::lower_shape_representation_with_parameters(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        srwp: ShapeRepresentationWithParameters,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_shape_representation_with_parameters(buf, srwp)?;
        Ok(serialize::serialize_shape_representation_with_parameters(
            buf, &early,
        ))
    }
}
