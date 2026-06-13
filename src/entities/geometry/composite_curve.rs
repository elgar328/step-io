//! `COMPOSITE_CURVE` handler.
//!
//! Mirrors `ReaderContext::convert_composite_curve` and
//! `WriteBuffer::emit_composite_curve`. Resolves each segment ref via
//! the `composite_segment_map` populated by the
//! `COMPOSITE_CURVE_SEGMENT` handler.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::composite_curve_segment::CompositeCurveSegmentHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::CompositeCurve;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CompositeCurveHandler;

#[step_entity(name = "COMPOSITE_CURVE")]
impl SimpleEntityHandler for CompositeCurveHandler {
    type WriteInput = CompositeCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_composite_curve(entity_id, attrs)?;
        lower::lower_composite_curve(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, composite: CompositeCurve) -> Result<u64, WriteError> {
        let mut segment_refs = Vec::with_capacity(composite.segments.len());
        for seg in &composite.segments {
            segment_refs.push(CompositeCurveSegmentHandler::write(buf, *seg)?);
        }
        let early = lift::lift_composite_curve(segment_refs, composite.self_intersect);
        Ok(serialize::serialize_composite_curve(buf, &early))
    }
}
