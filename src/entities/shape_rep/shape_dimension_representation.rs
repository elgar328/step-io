//! `SHAPE_DIMENSION_REPRESENTATION` handler — phase sdr-dcr.
//!
//! SDR is a SUBTYPE OF `SHAPE_REPRESENTATION`; unlike the [`PlainRepr`]
//! narrowing (which extracts a single placement frame), SDR preserves
//! the full `items` SET because blueprint members are
//! `dimension_representation_item` SELECT entries — generic
//! [`RepresentationItemRef`] resolution covers the variants step-io
//! models. Unresolved items skip silently.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ShapeDimensionRepresentation;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeDimensionRepresentationHandler;

#[step_entity(name = "SHAPE_DIMENSION_REPRESENTATION")]
impl SimpleEntityHandler for ShapeDimensionRepresentationHandler {
    type WriteInput = ShapeDimensionRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_dimension_representation(entity_id, attrs)?;
        lower::lower_shape_dimension_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sdr: ShapeDimensionRepresentation) -> Result<u64, WriteError> {
        let early = lift::lift_shape_dimension_representation(buf, sdr)?;
        Ok(serialize::serialize_shape_dimension_representation(
            buf, &early,
        ))
    }
}
