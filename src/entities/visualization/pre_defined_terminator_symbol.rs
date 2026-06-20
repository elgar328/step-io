//! `PRE_DEFINED_TERMINATOR_SYMBOL` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PreDefinedTerminatorSymbol;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PreDefinedTerminatorSymbolHandler;

#[step_entity(name = "PRE_DEFINED_TERMINATOR_SYMBOL")]
impl SimpleEntityHandler for PreDefinedTerminatorSymbolHandler {
    type WriteInput = PreDefinedTerminatorSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_pre_defined_terminator_symbol(entity_id, attrs)?;
        lower::lower_pre_defined_terminator_symbol(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: PreDefinedTerminatorSymbol) -> Result<u64, WriteError> {
        let early = lift::lift_pre_defined_terminator_symbol(v.name);
        Ok(serialize::serialize_pre_defined_terminator_symbol(
            buf, &early,
        ))
    }
}
