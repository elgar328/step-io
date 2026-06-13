//! `COMPOSITE_CURVE_SEGMENT` handler.
//!
//! Mirrors `ReaderContext::convert_composite_curve_segment` and
//! `WriteBuffer::emit_composite_curve_segment`. The reader records
//! segment metadata into `composite_segment_map` keyed by entity id;
//! the owning `COMPOSITE_CURVE` later assembles `CompositeSegment`
//! values from that map.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::CompositeSegment;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CompositeCurveSegmentHandler;

#[step_entity(name = "COMPOSITE_CURVE_SEGMENT")]
impl SimpleEntityHandler for CompositeCurveSegmentHandler {
    type WriteInput = CompositeSegment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_composite_curve_segment(entity_id, attrs)?;
        lower::lower_composite_curve_segment(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, segment: CompositeSegment) -> Result<u64, WriteError> {
        let parent = buf.emit_curve(segment.parent_curve)?;
        let early =
            lift::lift_composite_curve_segment(segment.transition, segment.same_sense, parent);
        Ok(serialize::serialize_composite_curve_segment(buf, &early))
    }
}
