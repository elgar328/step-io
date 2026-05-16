//! `PRESENTATION_STYLE_ASSIGNMENT` handler — Pass 7-9. Aggregates one or
//! more `SURFACE_STYLE_USAGE` entries.
//!
//! Other style flavours (`CURVE_STYLE` / `POINT_STYLE` / direct
//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) are silently dropped at read
//! time so the writer's symmetric drop preserves round-trip equality on
//! the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::PresentationStyleAssignment;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_style_usage::SurfaceStyleUsageHandler;
use step_io_macros::step_entity;

pub(crate) struct PresentationStyleAssignmentHandler;

#[step_entity(name = "PRESENTATION_STYLE_ASSIGNMENT", pass = Pass7Assignment)]
impl SimpleEntityHandler for PresentationStyleAssignmentHandler {
    type WriteInput = PresentationStyleAssignment;

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
            if let Some(ssu) = ctx.viz_ssu_map.get(&r).cloned() {
                styles.push(ssu);
            }
        }
        ctx.viz_psa_map
            .insert(entity_id, PresentationStyleAssignment { styles });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, psa: PresentationStyleAssignment) -> Result<u64, WriteError> {
        let mut style_refs = Vec::with_capacity(psa.styles.len());
        for ssu in psa.styles {
            style_refs.push(Attribute::EntityRef(SurfaceStyleUsageHandler::write(
                buf, ssu,
            )?));
        }
        Ok(buf.push_simple(
            "PRESENTATION_STYLE_ASSIGNMENT",
            vec![Attribute::List(style_refs)],
        ))
    }
}
