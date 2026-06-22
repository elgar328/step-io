//! `DRAUGHTING_PRE_DEFINED_COLOUR` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::DraughtingPreDefinedColour;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DraughtingPreDefinedColourHandler;

#[step_entity(name = "DRAUGHTING_PRE_DEFINED_COLOUR")]
impl SimpleEntityHandler for DraughtingPreDefinedColourHandler {
    type WriteInput = DraughtingPreDefinedColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_draughting_pre_defined_colour(entity_id, attrs)?;
        lower::lower_draughting_pre_defined_colour(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: DraughtingPreDefinedColour) -> Result<u64, WriteError> {
        let early = lift::lift_draughting_pre_defined_colour(v.name);
        Ok(serialize::serialize_draughting_pre_defined_colour(
            buf, &early,
        ))
    }
}
