//! `MEASURE_REPRESENTATION_ITEM` handlers.
//!
//! Two handlers register for the same entity name: a simple handler for the
//! single-line form and a `#[step_entity_complex]` handler for the
//! multi-part form (`(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
//! MEASURE_WITH_UNIT(...) [QUALIFIED_REPRESENTATION_ITEM(...)]
//! REPRESENTATION_ITEM(...)))` — the shape GD&T tolerance magnitudes take in
//! AP242). Both capture a `MeasureRepresentationItem` into the
//! `representation_item` arena (with the verbatim `measure_value` type-name)
//! and register it in `repr_item_id_map`; the property / GD&T consumers
//! resolve it there, and the writer emits it via `emit_representation_items`.

use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::PropertyMeasureUnit;
use crate::ir::representation_item::{
    MeasureForm, MeasureRepresentationItem, MeasureValue, QualifierRef, RepresentationItem,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct MeasureRepresentationItemHandler;

#[step_entity(name = "MEASURE_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for MeasureRepresentationItemHandler {
    /// Never dispatched — the writer emits the MRI via the
    /// `representation_item` arena (`emit_representation_items`).
    type WriteInput = ();

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MEASURE_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        // attrs[1] = typed value, attrs[2] = unit_component. Capture into the
        // representation_item arena (phase measure-arena-4) with the verbatim
        // measure_value (NUMERIC_MEASURE / RATIO_MEASURE / POSITIVE_LENGTH_MEASURE
        // are preserved, not dropped / downgraded). Consumers resolve it
        // through repr_item_id_map.
        let Some(value) = read_measure_value(attrs.get(1)) else {
            return Ok(());
        };
        let unit_ref = resolve_unit_ref(ctx, attrs.get(2));
        let id = ctx
            .representation_items
            .push(RepresentationItem::MeasureRepresentationItem(
                MeasureRepresentationItem {
                    form: MeasureForm::Simple,
                    name,
                    value,
                    unit_ref,
                    qualifiers: Vec::new(),
                    measure_supertype: None,
                },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: ()) -> Result<u64, WriteError> {
        unreachable!("MEASURE_REPRESENTATION_ITEM is emitted via the representation_item arena")
    }
}

pub(crate) struct MeasureRepresentationItemComplexHandler;

/// Complex (multi-part) `MEASURE_REPRESENTATION_ITEM`. The macro lists every
/// exact part-set this handler owns (the length / plane-angle / ratio measure
/// forms, with and without `QUALIFIED_REPRESENTATION_ITEM`). `name` comes from
/// the `REPRESENTATION_ITEM` part, the typed value + unit ref from the
/// `MEASURE_WITH_UNIT` part; the result is captured into the
/// `representation_item` arena identically to the simple handler.
#[step_entity_complex(
    name = "MEASURE_REPRESENTATION_ITEM",
    cases = [
        ["LENGTH_MEASURE_WITH_UNIT", "MEASURE_REPRESENTATION_ITEM", "MEASURE_WITH_UNIT", "QUALIFIED_REPRESENTATION_ITEM", "REPRESENTATION_ITEM"],
        ["LENGTH_MEASURE_WITH_UNIT", "MEASURE_REPRESENTATION_ITEM", "MEASURE_WITH_UNIT", "REPRESENTATION_ITEM"],
        ["MEASURE_REPRESENTATION_ITEM", "MEASURE_WITH_UNIT", "PLANE_ANGLE_MEASURE_WITH_UNIT", "REPRESENTATION_ITEM"],
        ["MEASURE_REPRESENTATION_ITEM", "MEASURE_WITH_UNIT", "RATIO_MEASURE_WITH_UNIT", "REPRESENTATION_ITEM"],
    ]
)]
impl ComplexEntityHandler for MeasureRepresentationItemComplexHandler {
    type WriteInput = ();

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();
        let mwu_attrs = require_part_attrs(parts, "MEASURE_WITH_UNIT", entity_id)?;
        // Capture the complex form into the `representation_items` arena so
        // SHAPE_DIMENSION_REPRESENTATION / tolerance / property resolve it.
        insert_measure_repr_item(ctx, entity_id, name, parts, mwu_attrs);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: ()) -> Result<u64, WriteError> {
        unreachable!("MEASURE_REPRESENTATION_ITEM is emitted via the representation_item arena")
    }
}

/// Resolve the `unit_component` ref to an explicit `NamedUnit` / `DerivedUnit`
/// arena ref; non-resolving refs fall through to `None`.
fn resolve_unit_ref(
    ctx: &ReaderContext,
    unit_attr: Option<&Attribute>,
) -> Option<PropertyMeasureUnit> {
    match unit_attr {
        Some(Attribute::EntityRef(uref)) => ctx
            .id_cache
            .get::<crate::ir::id::NamedUnitId>(*uref)
            .map(PropertyMeasureUnit::Named)
            .or_else(|| {
                ctx.id_cache
                    .get::<crate::ir::id::DerivedUnitId>(*uref)
                    .map(PropertyMeasureUnit::Derived)
            }),
        _ => None,
    }
}

/// Read a `measure_value` typed literal into [`MeasureValue`], preserving the
/// SELECT member type-name verbatim (e.g. `"POSITIVE_LENGTH_MEASURE"`,
/// `"NUMERIC_MEASURE"`). `None` when the attribute is not a primitive typed
/// measure value.
fn read_measure_value(value_attr: Option<&Attribute>) -> Option<MeasureValue> {
    match value_attr {
        Some(Attribute::Typed { type_name, value }) => match value.as_ref() {
            Attribute::Real(v) => Some(MeasureValue::Real {
                type_name: type_name.clone(),
                value: *v,
            }),
            Attribute::Integer(v) => Some(MeasureValue::Integer {
                type_name: type_name.clone(),
                value: *v,
            }),
            Attribute::String(s) => Some(MeasureValue::Text {
                type_name: type_name.clone(),
                value: s.clone(),
            }),
            _ => None,
        },
        _ => None,
    }
}

/// Capture a complex `MEASURE_REPRESENTATION_ITEM` into the
/// `representation_items` arena, preserving the typed `measure_value` (verbatim
/// type-name), unit ref, value qualifiers, and the typed `<X>_MEASURE_WITH_UNIT`
/// supertype part. Registered in `repr_item_id_map` so
/// `resolve_representation_item_ref` reaches it. Skips (no push) when the value
/// is not a primitive `measure_value`.
fn insert_measure_repr_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    parts: &[RawEntityPart],
    mwu_attrs: &[Attribute],
) {
    let Some(value) = read_measure_value(mwu_attrs.first()) else {
        return;
    };
    let unit_ref = resolve_unit_ref(ctx, mwu_attrs.get(1));
    // QUALIFIED_REPRESENTATION_ITEM part: attr[0] = SET of value_qualifier refs.
    let qualifiers = match find_part_attrs(parts, "QUALIFIED_REPRESENTATION_ITEM") {
        Some(q_attrs) => read_entity_ref_list(q_attrs, 0, entity_id, "qualifiers")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                if let Some(id) = ctx.id_cache.get::<crate::ir::id::TypeQualifierId>(r) {
                    Some(QualifierRef::TypeQualifier(id))
                } else {
                    ctx.id_cache
                        .get::<crate::ir::id::ValueFormatTypeQualifierId>(r)
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
                form: MeasureForm::Complex,
                name,
                value,
                unit_ref,
                qualifiers,
                measure_supertype,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}
