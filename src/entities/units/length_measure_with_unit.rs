//! `LENGTH_MEASURE_WITH_UNIT` handler (units-1).
//!
//! AP214 schema: `LENGTH_MEASURE_WITH_UNIT(value_component: LENGTH_MEASURE,
//! unit_component: unit)` — STEP positional `(typed-real, ref)`. The
//! `value_component` is a typed `LENGTH_MEASURE(real)`. Subtype of
//! `MEASURE_WITH_UNIT` with the same positional layout.
//!
//! MWUs referenced as the `conversion_factor` of a
//! `CONVERSION_BASED_UNIT` complex are skipped — the writer re-emits them
//! inline through the existing CBU emit path, so adding them to
//! `mwu_arena` would double-emit on round-trip.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct LengthMeasureWithUnitHandler;

#[step_entity(name = "LENGTH_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for LengthMeasureWithUnitHandler {
    /// `(value, unit_step_id)` — value as f64, unit ref resolved to its
    /// emitted STEP entity id by the writer's named-unit cache.
    type WriteInput = (f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if ctx.cbu_internal_mwu_refs.contains(&entity_id) {
            return Ok(());
        }
        let Some(early) = bind::bind_length_measure_with_unit(entity_id, attrs)? else {
            return Ok(());
        };
        lower::lower_length_measure_with_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        let early = lift::lift_length_measure_with_unit(value, unit_step);
        Ok(serialize::serialize_length_measure_with_unit(buf, &early))
    }
}
