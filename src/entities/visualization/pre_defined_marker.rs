//! `PRE_DEFINED_MARKER` / `PRE_DEFINED_POINT_MARKER_SYMBOL` handlers —
//! visualization (2-layer path). Both variants share the
//! `pre_defined_markers` arena.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PreDefinedMarkerData, PreDefinedPointMarkerSymbol};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PreDefinedMarkerHandler;

#[step_entity(name = "PRE_DEFINED_MARKER")]
impl SimpleEntityHandler for PreDefinedMarkerHandler {
    type WriteInput = PreDefinedMarkerData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_pre_defined_marker(entity_id, attrs)?;
        lower::lower_pre_defined_marker(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: PreDefinedMarkerData) -> Result<u64, WriteError> {
        let early = lift::lift_pre_defined_marker(d.name);
        Ok(serialize::serialize_pre_defined_marker(buf, &early))
    }
}

pub(crate) struct PreDefinedPointMarkerSymbolHandler;

#[step_entity(name = "PRE_DEFINED_POINT_MARKER_SYMBOL")]
impl SimpleEntityHandler for PreDefinedPointMarkerSymbolHandler {
    type WriteInput = PreDefinedPointMarkerSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_pre_defined_point_marker_symbol(entity_id, attrs)?;
        lower::lower_pre_defined_point_marker_symbol(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: PreDefinedPointMarkerSymbol) -> Result<u64, WriteError> {
        let early = lift::lift_pre_defined_point_marker_symbol(p.name);
        Ok(serialize::serialize_pre_defined_point_marker_symbol(
            buf, &early,
        ))
    }
}
