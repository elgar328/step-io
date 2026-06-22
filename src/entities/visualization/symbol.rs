//! `SYMBOL_TARGET` + `DEFINED_SYMBOL` handlers — phase ds-st.
//!
//! Both are `geometric_representation_item` subtypes that step-io models
//! through the unified `GeometricRepresentationItem` enum arena. Pass
//! split: `SYMBOL_TARGET`  is read first so that `DEFINED_SYMBOL`
//!  can resolve its `target` ref through `symbol_target_id_map`.
//! `DEFINED_SYMBOL.definition` resolves through
//! `viz_pre_defined_symbol_id_map`; unresolved members drop the carrier
//! (symmetric on re-read).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{DefinedSymbol, SymbolTarget};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolTargetHandler;

#[step_entity(name = "SYMBOL_TARGET")]
impl SimpleEntityHandler for SymbolTargetHandler {
    type WriteInput = SymbolTarget;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_symbol_target(entity_id, attrs)?;
        lower::lower_symbol_target(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: SymbolTarget) -> Result<u64, WriteError> {
        let early = lift::lift_symbol_target(buf, t)?;
        Ok(serialize::serialize_symbol_target(buf, &early))
    }
}

pub(crate) struct DefinedSymbolHandler;

#[step_entity(name = "DEFINED_SYMBOL")]
impl SimpleEntityHandler for DefinedSymbolHandler {
    type WriteInput = DefinedSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_defined_symbol(entity_id, attrs)?;
        lower::lower_defined_symbol(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DefinedSymbol) -> Result<u64, WriteError> {
        let early = lift::lift_defined_symbol(buf, d);
        Ok(serialize::serialize_defined_symbol(buf, &early))
    }
}
