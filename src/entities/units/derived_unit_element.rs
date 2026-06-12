//! `DERIVED_UNIT_ELEMENT` handler — units (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DerivedUnitElementHandler;

#[step_entity(name = "DERIVED_UNIT_ELEMENT")]
impl SimpleEntityHandler for DerivedUnitElementHandler {
    /// `(unit_step_id, exponent)` — STEP positional `(unit, exponent)`.
    type WriteInput = (u64, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_derived_unit_element(entity_id, attrs)?;
        lower::lower_derived_unit_element(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (unit_step, exponent): (u64, f64)) -> Result<u64, WriteError> {
        let early = lift::lift_derived_unit_element(unit_step, exponent);
        Ok(serialize::serialize_derived_unit_element(buf, &early))
    }
}
