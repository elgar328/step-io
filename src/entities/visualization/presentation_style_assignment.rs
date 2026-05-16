//! `PRESENTATION_STYLE_ASSIGNMENT` handler — Pass 7-9. Aggregates one or
//! more `SURFACE_STYLE_USAGE` entries.
//!
//! Other style flavours (`CURVE_STYLE` / `POINT_STYLE` / direct
//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) are silently dropped at read
//! time so the writer's symmetric drop preserves round-trip equality on
//! the supported subset.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::PresentationStyleAssignment;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_style_usage::SurfaceStyleUsageHandler;

pub(crate) struct PresentationStyleAssignmentHandler;

impl SimpleEntityHandler for PresentationStyleAssignmentHandler {
    const NAME: &'static str = "PRESENTATION_STYLE_ASSIGNMENT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7Assignment;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PSA_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: PresentationStyleAssignmentHandler::NAME,
    pass_level: PresentationStyleAssignmentHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: PresentationStyleAssignmentHandler::read,
    },
};
