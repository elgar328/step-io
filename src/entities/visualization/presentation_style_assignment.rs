//! `PRESENTATION_STYLE_ASSIGNMENT` handler — Pass 7-9. Aggregates one or
//! more styling entries: currently `SURFACE_STYLE_USAGE` (stored inline)
//! and `CURVE_STYLE` (stored via `CurveStyleId` arena reference).
//!
//! Other style flavours (`POINT_STYLE`, etc.) are silently dropped at
//! read time so the writer's symmetric drop preserves round-trip
//! equality on the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationStyleAssignment, PresentationStyleAssignmentData, PsaStyle, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct PresentationStyleAssignmentHandler;

#[step_entity(name = "PRESENTATION_STYLE_ASSIGNMENT", pass = Pass7Assignment)]
impl SimpleEntityHandler for PresentationStyleAssignmentHandler {
    type WriteInput = PresentationStyleAssignmentData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRESENTATION_STYLE_ASSIGNMENT")?;
        let style_refs = read_entity_ref_list(attrs, 0, entity_id, "styles")?;
        let mut styles = Vec::new();
        for r in style_refs {
            if let Some(&ssu_id) = ctx.viz_ssu_id_map.get(&r) {
                styles.push(PsaStyle::Surface(ssu_id));
            } else if let Some(&cs_id) = ctx.viz_curve_style_id_map.get(&r) {
                styles.push(PsaStyle::Curve(cs_id));
            }
        }
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .presentation_style_assignments
            .push(PresentationStyleAssignment::Itself(
                PresentationStyleAssignmentData { styles },
            ));
        ctx.viz_psa_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        data: PresentationStyleAssignmentData,
    ) -> Result<u64, WriteError> {
        let mut style_refs = Vec::with_capacity(data.styles.len());
        for style in data.styles {
            let ref_id = match style {
                PsaStyle::Surface(ssu_id) => buf.founded_item_step_ids[ssu_id.0 as usize],
                PsaStyle::Curve(cs_id) => buf.curve_style_step_ids[cs_id.0 as usize],
            };
            style_refs.push(Attribute::EntityRef(ref_id));
        }
        Ok(buf.push_simple(
            "PRESENTATION_STYLE_ASSIGNMENT",
            vec![Attribute::List(style_refs)],
        ))
    }
}
