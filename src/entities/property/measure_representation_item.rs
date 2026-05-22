//! `MEASURE_REPRESENTATION_ITEM` handlers — Pass 8-1.
//!
//! Two handlers register for the same entity name: a simple handler for the
//! single-line form and a `#[step_entity_complex]` handler for the
//! multi-part form (`(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
//! MEASURE_WITH_UNIT(...) [QUALIFIED_REPRESENTATION_ITEM(...)]
//! REPRESENTATION_ITEM(...)))` — the shape GD&T tolerance magnitudes take in
//! AP242). Both store `(name, kind, value)` in `measure_item_map` keyed by
//! STEP entity id; the PDR pass and the GD&T pass collect these. Writer emits
//! the bare simple MRI line with a typed measure and a resolved unit ref.

use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::UnitContextId;
use crate::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

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
        // attrs[1] = typed value, attrs[2] = unit_component.
        insert_measure_item(ctx, entity_id, name, attrs.get(1), attrs.get(2));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (m, ctx): (PropertyMeasure, Option<UnitContextId>),
    ) -> Result<u64, WriteError> {
        Ok(buf.emit_property_measure(&m, ctx))
    }
}

pub(crate) struct MeasureRepresentationItemComplexHandler;

/// Complex (multi-part) `MEASURE_REPRESENTATION_ITEM`. `required` keeps only
/// the three parts every instance carries, so both the 4-part and the
/// 5-part (`QUALIFIED_REPRESENTATION_ITEM`-bearing) forms match —
/// `has_all_parts` admits supersets. `name` comes from the
/// `REPRESENTATION_ITEM` part, the typed value + unit ref from the
/// `MEASURE_WITH_UNIT` part; the result lands in `measure_item_map`
/// identically to the simple handler. The `write` method satisfies the
/// trait but is never dispatched — the writer always re-emits the simple
/// MRI form through `emit_property_measure`.
#[step_entity_complex(
    name = "MEASURE_REPRESENTATION_ITEM",
    pass = Pass8Measure,
    required = ["MEASURE_REPRESENTATION_ITEM", "MEASURE_WITH_UNIT", "REPRESENTATION_ITEM"]
)]
impl ComplexEntityHandler for MeasureRepresentationItemComplexHandler {
    type WriteInput = (PropertyMeasure, Option<UnitContextId>);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();
        let mwu_attrs = require_part_attrs(parts, "MEASURE_WITH_UNIT", entity_id)?;
        // MEASURE_WITH_UNIT part: attr[0] = typed value, attr[1] = unit_component.
        insert_measure_item(ctx, entity_id, name, mwu_attrs.first(), mwu_attrs.get(1));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (m, ctx): (PropertyMeasure, Option<UnitContextId>),
    ) -> Result<u64, WriteError> {
        Ok(buf.emit_property_measure(&m, ctx))
    }
}

/// Build a [`PropertyMeasure`] from the MRI value pair and insert it into
/// `measure_item_map`. `value_attr` is the typed measure (`Attribute::Typed`),
/// `unit_attr` the `unit_component` ref. Silently skips (no insert) when the
/// measure kind or value shape is outside the supported set — identical
/// ignorance for the simple and complex MRI forms keeps round-trip intact.
fn insert_measure_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    value_attr: Option<&Attribute>,
    unit_attr: Option<&Attribute>,
) {
    let Some(Attribute::Typed { type_name, value }) = value_attr else {
        return;
    };
    let Some(kind) = match_measure_kind(type_name) else {
        return;
    };
    let Attribute::Real(measure_value) = value.as_ref() else {
        return;
    };
    // Resolve the unit_component ref to an explicit NamedUnit / DerivedUnit
    // arena ref; non-resolving refs fall through to `unit_ref = None` and the
    // writer's context-based fallback.
    let unit_ref = match unit_attr {
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
}

fn match_measure_kind(type_name: &str) -> Option<MeasureKind> {
    match type_name {
        "LENGTH_MEASURE" | "POSITIVE_LENGTH_MEASURE" => Some(MeasureKind::Length),
        "PLANE_ANGLE_MEASURE" => Some(MeasureKind::PlaneAngle),
        "SOLID_ANGLE_MEASURE" => Some(MeasureKind::SolidAngle),
        "POSITIVE_RATIO_MEASURE" => Some(MeasureKind::PositiveRatio),
        "MASS_MEASURE" => Some(MeasureKind::Mass),
        "AREA_MEASURE" => Some(MeasureKind::Area),
        "VOLUME_MEASURE" => Some(MeasureKind::Volume),
        _ => None,
    }
}
