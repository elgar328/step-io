//! `MEASURE_REPRESENTATION_ITEM` handlers — 2-layer.
//!
//! Two handlers register for the same entity name: a simple handler for the
//! single-line form and a `#[step_entity_complex]` handler for the multi-part
//! form (`(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
//! MEASURE_WITH_UNIT(...) [QUALIFIED_REPRESENTATION_ITEM(...)]
//! REPRESENTATION_ITEM(...)))` — the shape GD&T tolerance magnitudes take in
//! AP242). Both `read` = generated `bind` + hand `lower` (capturing a
//! `MeasureRepresentationItem` into the `representation_item` arena, verbatim
//! `measure_value` type-name); the writer emits via `emit_representation_items`
//! (lift + generated serialize), so both `write`s are `unreachable!`.

use crate::early::{bind, lower};
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
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
        if let Some(early) = bind::bind_measure_representation_item(entity_id, attrs)? {
            lower::lower_measure_representation_item(ctx, entity_id, &early);
        }
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: ()) -> Result<u64, WriteError> {
        unreachable!("MEASURE_REPRESENTATION_ITEM is emitted via the representation_item arena")
    }
}

pub(crate) struct MeasureRepresentationItemComplexHandler;

/// Complex (multi-part) `MEASURE_REPRESENTATION_ITEM`. The macro lists every
/// exact part-set this handler owns (the length / plane-angle / ratio measure
/// forms, with and without `QUALIFIED_REPRESENTATION_ITEM`). bind exact-matches
/// the part-set into the case variant; lower bridges to the arena.
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
        if let Some(early) = bind::bind_measure_representation_item_complex(entity_id, parts)? {
            lower::lower_measure_representation_item_complex(ctx, entity_id, early);
        }
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: ()) -> Result<u64, WriteError> {
        unreachable!("MEASURE_REPRESENTATION_ITEM is emitted via the representation_item arena")
    }
}
