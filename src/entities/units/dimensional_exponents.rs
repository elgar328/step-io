//! `DIMENSIONAL_EXPONENTS` handler — units (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::DimensionalExponents;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DimensionalExponentsHandler;

#[step_entity(name = "DIMENSIONAL_EXPONENTS")]
impl SimpleEntityHandler for DimensionalExponentsHandler {
    type WriteInput = DimensionalExponents;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_dimensional_exponents(entity_id, attrs)?;
        lower::lower_dimensional_exponents(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, de: DimensionalExponents) -> Result<u64, WriteError> {
        let early = lift::lift_dimensional_exponents(de);
        Ok(serialize::serialize_dimensional_exponents(buf, &early))
    }
}
