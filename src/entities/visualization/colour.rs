//! `COLOUR` handler — the attributeless base colour entity (`ENTITY colour;
//! END_ENTITY;`, ISO 10303-46). Schema-valid to instantiate directly as
//! `COLOUR()`; NIST fixtures use it as an unspecified-colour placeholder.
//! Reader pushes a `Colour::Itself` variant into `VisualizationPool::colours`
//! (2-layer path; reuses the generated 0-attr bind/serialize).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ColourHandler;

#[step_entity(name = "COLOUR")]
impl SimpleEntityHandler for ColourHandler {
    type WriteInput = ();

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_colour(entity_id, attrs)?;
        lower::lower_colour(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (): ()) -> Result<u64, WriteError> {
        Ok(serialize::serialize_colour(buf, &lift::lift_colour()))
    }
}
