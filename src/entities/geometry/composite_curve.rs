//! `COMPOSITE_CURVE` handler — Pass 4-3c.
//!
//! Mirrors `ReaderContext::convert_composite_curve` and
//! `WriteBuffer::emit_composite_curve`. Resolves each segment ref via
//! the `composite_segment_map` populated by the
//! `COMPOSITE_CURVE_SEGMENT` handler.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::composite_curve_segment::CompositeCurveSegmentHandler;
use crate::ir::attr::{
    check_count, logical_to_step, read_entity_ref_list, read_logical, read_string,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CompositeCurve, CompositeSegment, Curve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct CompositeCurveHandler;

#[step_entity(name = "COMPOSITE_CURVE", pass = Pass4_3cComp)]
impl SimpleEntityHandler for CompositeCurveHandler {
    type WriteInput = CompositeCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COMPOSITE_CURVE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let segment_refs = read_entity_ref_list(attrs, 1, entity_id, "segments")?;
        let self_intersect = read_logical(attrs, 2, entity_id, "self_intersect")?;

        let mut segments = Vec::with_capacity(segment_refs.len());
        for seg_ref in segment_refs {
            let Some(&(transition, same_sense, parent_step_id)) =
                ctx.composite_segment_map.get(&seg_ref)
            else {
                return Err(ConvertError::MissingReference {
                    from: entity_id,
                    to: seg_ref,
                    field_name: "segments",
                });
            };
            let parent_curve = ctx.resolve_curve(entity_id, parent_step_id, "parent_curve")?;
            segments.push(CompositeSegment {
                transition,
                same_sense,
                parent_curve,
            });
        }

        let composite = CompositeCurve {
            segments,
            self_intersect,
        };
        let id = ctx.geometry.curves.push(Curve::Composite(composite));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, composite: CompositeCurve) -> Result<u64, WriteError> {
        let mut segment_refs = Vec::with_capacity(composite.segments.len());
        for seg in &composite.segments {
            segment_refs.push(Attribute::EntityRef(CompositeCurveSegmentHandler::write(
                buf, *seg,
            )?));
        }
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COMPOSITE_CURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(segment_refs),
                    Attribute::Enum(logical_to_step(composite.self_intersect).into()),
                ],
            },
        });
        Ok(n)
    }
}
