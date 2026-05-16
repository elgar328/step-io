//! `COMPOSITE_CURVE_SEGMENT` handler — Pass 4-3c.
//!
//! Mirrors `ReaderContext::convert_composite_curve_segment` and
//! `WriteBuffer::emit_composite_curve_segment`. The reader records
//! segment metadata into `composite_segment_map` keyed by entity id;
//! the owning `COMPOSITE_CURVE` later assembles `CompositeSegment`
//! values from that map.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CompositeSegment, TransitionCode};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct CompositeCurveSegmentHandler;

impl SimpleEntityHandler for CompositeCurveSegmentHandler {
    const NAME: &'static str = "COMPOSITE_CURVE_SEGMENT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_3cTrimSeg;
    type WriteInput = CompositeSegment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COMPOSITE_CURVE_SEGMENT")?;
        let transition_str = read_enum(attrs, 0, entity_id, "transition")?;
        let same_sense = read_bool(attrs, 1, entity_id, "same_sense")?;
        let parent_step_id = read_entity_ref(attrs, 2, entity_id, "parent_curve")?;

        let transition = match transition_str {
            "CONTINUOUS" => TransitionCode::Continuous,
            "DISCONTINUOUS" => TransitionCode::Discontinuous,
            "CONT_SAME_GRADIENT" => TransitionCode::ContSameGradient,
            "CONT_SAME_GRADIENT_SAME_CURVATURE" => TransitionCode::ContSameGradientSameCurvature,
            _ => TransitionCode::Unspecified,
        };
        ctx.composite_segment_map
            .insert(entity_id, (transition, same_sense, parent_step_id));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, segment: CompositeSegment) -> Result<u64, WriteError> {
        let parent = buf.emit_curve(segment.parent_curve)?;
        let transition = match segment.transition {
            TransitionCode::Continuous => "CONTINUOUS",
            TransitionCode::Discontinuous => "DISCONTINUOUS",
            TransitionCode::ContSameGradient => "CONT_SAME_GRADIENT",
            TransitionCode::ContSameGradientSameCurvature => "CONT_SAME_GRADIENT_SAME_CURVATURE",
            TransitionCode::Unspecified => "UNSPECIFIED",
        };
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COMPOSITE_CURVE_SEGMENT".into(),
                attrs: vec![
                    Attribute::Enum(transition.into()),
                    Attribute::Enum(if segment.same_sense { "T" } else { "F" }.into()),
                    Attribute::EntityRef(parent),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static COMPOSITE_CURVE_SEGMENT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: CompositeCurveSegmentHandler::NAME,
    pass_level: CompositeCurveSegmentHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: CompositeCurveSegmentHandler::read,
    },
};
