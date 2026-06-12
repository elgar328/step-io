//! `SYMBOL_STYLE` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::SymbolStyle;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolStyleHandler;

#[step_entity(name = "SYMBOL_STYLE")]
impl SimpleEntityHandler for SymbolStyleHandler {
    type WriteInput = SymbolStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_symbol_style(entity_id, attrs)?;
        let _ = entity_id;
        lower::lower_symbol_style(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ss: SymbolStyle) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(ss.style_of_symbol);
        let early = lift::lift_symbol_style(ss.name, colour_step);
        Ok(serialize::serialize_symbol_style(buf, &early))
    }
}
