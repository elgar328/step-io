//! Bare `MEASURE_WITH_UNIT` supertype handler (carrier `Itself` variant).
//!
//! Per the ir.toml blueprint `measure_with_unit` is a `concrete_supertype`
//! (`shape = "carrier"`). The typed subtypes (`LENGTH_MEASURE_WITH_UNIT` etc.)
//! carry the measure kind in the entity name; the bare supertype carries it in
//! the typed `value_component` (`MEASURE_WITH_UNIT(LENGTH_MEASURE(-0.3), unit)`),
//! so it can hold *any* measure kind. It is read into the shared `measure_with_unit`
//! arena as [`MeasureWithUnit::Itself`], preserving the `measure_type` name so
//! the writer re-emits the generic form (not a typed subtype). Tolerance values
//! / measure qualifications resolve it through `mwu_id_map` regardless of variant.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MeasureWithUnitHandler;

#[step_entity(name = "MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for MeasureWithUnitHandler {
    /// `(measure_type, value, unit_step_id)`.
    type WriteInput = (String, f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(early) = bind::bind_measure_with_unit(entity_id, attrs)? else {
            return Ok(()); // value_component (measure_value) did not bind — drop
        };
        lower::lower_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (measure_type, value, unit_step): (String, f64, u64),
    ) -> Result<u64, WriteError> {
        let early = lift::lift_measure_with_unit(measure_type, value, unit_step);
        Ok(serialize::serialize_measure_with_unit(buf, &early))
    }
}
