//! `PRE_DEFINED_SYMBOL` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PreDefinedSymbolData;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PreDefinedSymbolHandler;

#[step_entity(name = "PRE_DEFINED_SYMBOL")]
impl SimpleEntityHandler for PreDefinedSymbolHandler {
    type WriteInput = PreDefinedSymbolData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_pre_defined_symbol(entity_id, attrs)?;
        lower::lower_pre_defined_symbol(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: PreDefinedSymbolData) -> Result<u64, WriteError> {
        let early = lift::lift_pre_defined_symbol(v.name);
        Ok(serialize::serialize_pre_defined_symbol(buf, &early))
    }
}
