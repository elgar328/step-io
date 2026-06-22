//! Bare `NAMED_UNIT(#dimensions)` handler — a dimensionless/count unit.
//!
//! `NAMED_UNIT` is the abstract supertype of the typed units, but it is also
//! directly instantiable as a simple entity carrying just the inherited
//! `dimensions` attribute (e.g. `NAMED_UNIT(#dims)` with all-zero exponents =
//! a count unit). This is the named-unit analogue of bare `MEASURE_WITH_UNIT`.
//! Shares the `named_units` arena via [`NamedUnit::Itself`]; coexists with the
//! complex `(MASS_UNIT()NAMED_UNIT()SI_UNIT())` handlers (simple vs complex
//! dispatch, mirroring `RATIO_UNIT`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::NamedUnitData;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct NamedUnitSimpleHandler;

#[step_entity(name = "NAMED_UNIT")]
impl SimpleEntityHandler for NamedUnitSimpleHandler {
    /// The bare named-unit arena data. The units pool emitter pre-reserves each
    /// `NamedUnit` step id and calls `serialize_named_unit_with_id` directly
    /// (see `emit_named_unit_plain`); this fresh-id `write` exists for the
    /// generic handler contract.
    type WriteInput = NamedUnitData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_named_unit(entity_id, attrs)?;
        lower::lower_named_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: NamedUnitData) -> Result<u64, WriteError> {
        let dim_step = data.dimensions.map_or(0, |id| buf.step_id(id));
        Ok(serialize::serialize_named_unit(
            buf,
            &lift::lift_named_unit(dim_step),
        ))
    }
}
