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

use crate::entities::SimpleEntityHandler;
use crate::entities::units::length_measure_with_unit::emit_mwu;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::units::{MeasureWithUnit, MeasureWithUnitData};
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
        // A bare MWU used as a CONVERSION_BASED_UNIT conversion factor is
        // re-emitted inline by the CBU path — keep it out of the arena to avoid
        // double-emit (mirrors the typed MWU handlers).
        if ctx.cbu_internal_mwu_refs.contains(&entity_id) {
            return Ok(());
        }
        check_count(attrs, 2, entity_id, "MEASURE_WITH_UNIT")?;
        // value_component is a typed `measure_value`, e.g. LENGTH_MEASURE(-0.3).
        // Keep the type name (the measure kind) — unlike `read_mwu_attrs`, which
        // discards it. Negative reals are accepted (tolerance bounds).
        let (measure_type, value) = match attrs.first() {
            Some(Attribute::Typed { type_name, value }) => match value.as_ref() {
                Attribute::Real(v) => (type_name.clone(), *v),
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        let unit_step = read_entity_ref(attrs, 1, entity_id, "unit_component")?;
        let Some(&unit) = ctx.named_unit_id_map.get(&unit_step) else {
            return Ok(());
        };
        let id = ctx
            .mwu_arena
            .push(MeasureWithUnit::Itself(MeasureWithUnitData {
                measure_type,
                value,
                unit,
            }));
        ctx.mwu_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (measure_type, value, unit_step): (String, f64, u64),
    ) -> Result<u64, WriteError> {
        Ok(emit_mwu(
            buf,
            "MEASURE_WITH_UNIT",
            &measure_type,
            value,
            unit_step,
        ))
    }
}
