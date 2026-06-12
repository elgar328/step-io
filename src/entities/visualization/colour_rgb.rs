//! `COLOUR_RGB` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::ColourRgb;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ColourRgbHandler;

#[step_entity(name = "COLOUR_RGB")]
impl SimpleEntityHandler for ColourRgbHandler {
    type WriteInput = ColourRgb;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_colour_rgb(entity_id, attrs)?;
        lower::lower_colour_rgb(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: ColourRgb) -> Result<u64, WriteError> {
        let early = lift::lift_colour_rgb(c);
        Ok(serialize::serialize_colour_rgb(buf, &early))
    }
}
