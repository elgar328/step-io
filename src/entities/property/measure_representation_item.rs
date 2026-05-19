//! `MEASURE_REPRESENTATION_ITEM` handler — Pass 8-1.
//!
//! Reader stores `(name, kind, value)` in `measure_item_map` keyed by
//! STEP entity id; the PDR pass collects these into the parent `Property`.
//! Writer emits the bare MRI line with a typed measure (LENGTH / PLANE /
//! SOLID) and the unit ref resolved through the PD's bound context.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::UnitContextId;
use crate::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MeasureRepresentationItemHandler;

#[step_entity(name = "MEASURE_REPRESENTATION_ITEM", pass = Pass8Measure)]
impl SimpleEntityHandler for MeasureRepresentationItemHandler {
    /// `(measure, ctx)` — the writer uses both the measure value/kind and
    /// the parent property's unit context to resolve the unit ref.
    type WriteInput = (PropertyMeasure, Option<UnitContextId>);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MEASURE_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        // attrs[1] is a typed value (LENGTH_MEASURE / PLANE_ANGLE_MEASURE / ...).
        // Skip silently if the kind is outside the supported set —
        // symmetric ignorance keeps round-trip equality intact.
        let Some(Attribute::Typed { type_name, value }) = attrs.get(1) else {
            return Ok(());
        };
        let Some(kind) = match_measure_kind(type_name) else {
            return Ok(());
        };
        let Attribute::Real(measure_value) = value.as_ref() else {
            return Ok(());
        };
        // attrs[2] = unit_component. Try to resolve it to an explicit
        // NamedUnit / DerivedUnit arena ref so the writer can reproduce
        // the original unit (especially DERIVED_UNIT composite refs that
        // the per-kind context lookup can't recover). Non-resolving refs
        // fall through to `unit_ref = None` and the writer's legacy
        // context-based fallback.
        let unit_ref = match attrs.get(2) {
            Some(Attribute::EntityRef(uref)) => ctx
                .named_unit_id_map
                .get(uref)
                .copied()
                .map(PropertyMeasureUnit::Named)
                .or_else(|| {
                    ctx.derived_unit_id_map
                        .get(uref)
                        .copied()
                        .map(PropertyMeasureUnit::Derived)
                }),
            _ => None,
        };
        ctx.measure_item_map.insert(
            entity_id,
            PropertyMeasure {
                name,
                kind,
                value: *measure_value,
                unit_ref,
            },
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (m, ctx): (PropertyMeasure, Option<UnitContextId>),
    ) -> Result<u64, WriteError> {
        Ok(buf.emit_property_measure(&m, ctx))
    }
}

fn match_measure_kind(type_name: &str) -> Option<MeasureKind> {
    match type_name {
        "LENGTH_MEASURE" => Some(MeasureKind::Length),
        "PLANE_ANGLE_MEASURE" => Some(MeasureKind::PlaneAngle),
        "SOLID_ANGLE_MEASURE" => Some(MeasureKind::SolidAngle),
        "POSITIVE_RATIO_MEASURE" => Some(MeasureKind::PositiveRatio),
        "MASS_MEASURE" => Some(MeasureKind::Mass),
        "AREA_MEASURE" => Some(MeasureKind::Area),
        "VOLUME_MEASURE" => Some(MeasureKind::Volume),
        _ => None,
    }
}
