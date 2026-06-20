//! `SYMBOL_COLOUR` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::SymbolColour;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolColourHandler;

#[step_entity(name = "SYMBOL_COLOUR")]
impl SimpleEntityHandler for SymbolColourHandler {
    type WriteInput = SymbolColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_symbol_colour(entity_id, attrs)?;
        lower::lower_symbol_colour(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: SymbolColour) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(s.colour_of_symbol);
        let early = lift::lift_symbol_colour(colour_step);
        Ok(serialize::serialize_symbol_colour(buf, &early))
    }
}
