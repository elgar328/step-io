//! `PRE_DEFINED_CURVE_FONT` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PreDefinedCurveFontData;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PreDefinedCurveFontHandler;

#[step_entity(name = "PRE_DEFINED_CURVE_FONT")]
impl SimpleEntityHandler for PreDefinedCurveFontHandler {
    type WriteInput = PreDefinedCurveFontData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_pre_defined_curve_font(entity_id, attrs)?;
        lower::lower_pre_defined_curve_font(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: PreDefinedCurveFontData) -> Result<u64, WriteError> {
        let early = lift::lift_pre_defined_curve_font(v.name);
        Ok(serialize::serialize_pre_defined_curve_font(buf, &early))
    }
}
