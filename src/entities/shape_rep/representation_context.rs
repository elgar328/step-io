//! `REPRESENTATION_CONTEXT` handler — shape-rep domain (2-layer path).
//! Bare unitless context (no dimension count).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::UnitlessContext;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RepresentationContextHandler;

#[step_entity(name = "REPRESENTATION_CONTEXT")]
impl SimpleEntityHandler for RepresentationContextHandler {
    type WriteInput = UnitlessContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_representation_context(entity_id, attrs)?;
        lower::lower_representation_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, uc: UnitlessContext) -> Result<u64, WriteError> {
        let early = lift::lift_representation_context(uc.identifier, uc.context_type);
        Ok(serialize::serialize_representation_context(buf, &early))
    }
}
