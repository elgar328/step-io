//! `FILL_AREA_STYLE_COLOUR` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyleColour;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleColourHandler;

#[step_entity(name = "FILL_AREA_STYLE_COLOUR")]
impl SimpleEntityHandler for FillAreaStyleColourHandler {
    type WriteInput = FillAreaStyleColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_fill_area_style_colour(entity_id, attrs)?;
        lower::lower_fill_area_style_colour(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, fasc: FillAreaStyleColour) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(fasc.colour);
        let early = lift::lift_fill_area_style_colour(fasc.name, colour_step);
        Ok(serialize::serialize_fill_area_style_colour(buf, &early))
    }
}
