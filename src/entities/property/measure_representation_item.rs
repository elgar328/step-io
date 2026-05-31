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
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::UnitContextId;
use crate::ir::property::{MeasureKind, PropertyMeasure, PropertyMeasureUnit};
use crate::ir::representation_item::{
    MeasureRepresentationItem, MeasureValue, QualifierRef, RepresentationItem,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
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
        // Keep the `measure_item_map` entry for the GD&T tolerance-magnitude
        // path (phase measure-arena-1 has not yet rerouted it).
        insert_measure_item(
            ctx,
            entity_id,
            name.clone(),
            mwu_attrs.first(),
            mwu_attrs.get(1),
        );
        // Also capture the complex form into the `representation_items` arena
        // so `SHAPE_DIMENSION_REPRESENTATION` resolves it (phase measure-arena-1).
        insert_measure_repr_item(ctx, entity_id, name, parts, mwu_attrs);
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
    let unit_ref = resolve_unit_ref(ctx, unit_attr);
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

/// Resolve the `unit_component` ref to an explicit `NamedUnit` / `DerivedUnit`
/// arena ref; non-resolving refs fall through to `None`.
fn resolve_unit_ref(
    ctx: &ReaderContext,
    unit_attr: Option<&Attribute>,
) -> Option<PropertyMeasureUnit> {
    match unit_attr {
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
    }
}

/// Capture a complex `MEASURE_REPRESENTATION_ITEM` into the
/// `representation_items` arena (phase measure-arena-1), preserving the typed
/// `measure_value` (verbatim type-name), unit ref, value qualifiers, and the
/// typed `<X>_MEASURE_WITH_UNIT` supertype part. Registered in
/// `repr_item_id_map` so `resolve_representation_item_ref` reaches it. Skips
/// (no push) when the value is not a primitive `measure_value`.
fn insert_measure_repr_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    parts: &[RawEntityPart],
    mwu_attrs: &[Attribute],
) {
    let value = match mwu_attrs.first() {
        Some(Attribute::Typed { type_name, value }) => match value.as_ref() {
            Attribute::Real(v) => MeasureValue::Real {
                type_name: type_name.clone(),
                value: *v,
            },
            Attribute::Integer(v) => MeasureValue::Integer {
                type_name: type_name.clone(),
                value: *v,
            },
            Attribute::String(s) => MeasureValue::Text {
                type_name: type_name.clone(),
                value: s.clone(),
            },
            _ => return,
        },
        _ => return,
    };
    let unit_ref = resolve_unit_ref(ctx, mwu_attrs.get(1));
    // QUALIFIED_REPRESENTATION_ITEM part: attr[0] = SET of value_qualifier refs.
    let qualifiers = match find_part_attrs(parts, "QUALIFIED_REPRESENTATION_ITEM") {
        Some(q_attrs) => read_entity_ref_list(q_attrs, 0, entity_id, "qualifiers")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                if let Some(&id) = ctx.type_qualifier_id_map.get(&r) {
                    Some(QualifierRef::TypeQualifier(id))
                } else {
                    ctx.value_format_type_qualifier_id_map
                        .get(&r)
                        .copied()
                        .map(QualifierRef::ValueFormatTypeQualifier)
                }
            })
            .collect(),
        None => Vec::new(),
    };
    // The typed `<X>_MEASURE_WITH_UNIT` supertype part (e.g.
    // `LENGTH_MEASURE_WITH_UNIT`), distinct from the base `MEASURE_WITH_UNIT`.
    let measure_supertype = parts
        .iter()
        .map(|p| p.name.as_str())
        .find(|n| n.ends_with("_MEASURE_WITH_UNIT") && *n != "MEASURE_WITH_UNIT")
        .map(str::to_owned);
    let id = ctx
        .representation_items
        .push(RepresentationItem::MeasureRepresentationItem(
            MeasureRepresentationItem {
                name,
                value,
                unit_ref,
                qualifiers,
                measure_supertype,
            },
        ));
    ctx.repr_item_id_map.insert(entity_id, id);
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
