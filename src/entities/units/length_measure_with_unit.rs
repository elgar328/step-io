//! `LENGTH_MEASURE_WITH_UNIT` handler — Pass 0-3 (units-1).
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

use crate::entities::SimpleEntityHandler;
use crate::entities::units::shared::read_mwu_attrs;
use crate::ir::error::ConvertError;
use crate::ir::units::MeasureWithUnit;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct LengthMeasureWithUnitHandler;

#[step_entity(name = "LENGTH_MEASURE_WITH_UNIT", pass = Pass0MwuDue)]
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
        let Some((value, unit_step)) =
            read_mwu_attrs(attrs, entity_id, "LENGTH_MEASURE_WITH_UNIT")?
        else {
            return Ok(());
        };
        let Some(&unit_id) = ctx.named_unit_id_map.get(&unit_step) else {
            return Ok(());
        };
        let id = ctx.mwu_arena.push(MeasureWithUnit::Length {
            value,
            unit: unit_id,
        });
        ctx.mwu_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        Ok(emit_mwu(
            buf,
            "LENGTH_MEASURE_WITH_UNIT",
            "LENGTH_MEASURE",
            value,
            unit_step,
        ))
    }
}

pub(super) fn emit_mwu(
    buf: &mut WriteBuffer,
    entity_name: &str,
    measure_type: &str,
    value: f64,
    unit_step: u64,
) -> u64 {
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: entity_name.into(),
            attrs: vec![
                Attribute::Typed {
                    type_name: measure_type.into(),
                    value: Box::new(Attribute::Real(value)),
                },
                Attribute::EntityRef(unit_step),
            ],
        },
    });
    n
}
