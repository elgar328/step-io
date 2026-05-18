//! `RATIO_MEASURE_WITH_UNIT` handler — Pass 0-3 (units-1).

use crate::entities::SimpleEntityHandler;
use crate::entities::units::length_measure_with_unit::emit_mwu;
use crate::entities::units::shared::read_mwu_attrs;
use crate::ir::error::ConvertError;
use crate::ir::units::MeasureWithUnit;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RatioMeasureWithUnitHandler;

#[step_entity(name = "RATIO_MEASURE_WITH_UNIT", pass = Pass0MwuDue)]
impl SimpleEntityHandler for RatioMeasureWithUnitHandler {
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
        let Some((value, unit_step)) = read_mwu_attrs(attrs, entity_id, "RATIO_MEASURE_WITH_UNIT")?
        else {
            return Ok(());
        };
        let Some(&unit_id) = ctx.named_unit_id_map.get(&unit_step) else {
            return Ok(());
        };
        let id = ctx.mwu_arena.push(MeasureWithUnit::Ratio {
            value,
            unit: unit_id,
        });
        ctx.mwu_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, unit_step): (f64, u64)) -> Result<u64, WriteError> {
        Ok(emit_mwu(
            buf,
            "RATIO_MEASURE_WITH_UNIT",
            "POSITIVE_RATIO_MEASURE",
            value,
            unit_step,
        ))
    }
}
