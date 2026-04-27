//! `MEASURE_REPRESENTATION_ITEM` handler — Pass 8-1.
//!
//! Reader stores `(name, kind, value)` in `measure_item_map` keyed by
//! STEP entity id; the PDR pass collects these into the parent `Property`.
//! Writer emits the bare MRI line with a typed measure (LENGTH / PLANE /
//! SOLID) and the unit ref resolved through the PD's bound context.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::UnitContextId;
use crate::ir::property::{MeasureKind, PropertyMeasure};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct MeasureRepresentationItemHandler;

impl SimpleEntityHandler for MeasureRepresentationItemHandler {
    const NAME: &'static str = "MEASURE_REPRESENTATION_ITEM";
    const PASS_LEVEL: PassLevel = PassLevel::Pass8Measure;
    /// `(measure, ctx)` — the writer uses both the measure value/kind and
    /// the parent property's unit context to resolve the unit ref.
    type WriteInput = (PropertyMeasure, Option<UnitContextId>);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
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
        // attrs[2] = unit_component — ignored. The bound REPRESENTATION's
        // `context_of_items` field (or the parent Property's `context`) is
        // the authoritative unit reference; the writer reproduces it from
        // there.
        ctx.measure_item_map.insert(
            entity_id,
            PropertyMeasure {
                name,
                kind,
                value: *measure_value,
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
        _ => None,
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static MRI_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: MeasureRepresentationItemHandler::NAME,
    pass_level: MeasureRepresentationItemHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: MeasureRepresentationItemHandler::read,
    },
};
