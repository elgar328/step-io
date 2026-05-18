//! `SURFACE_SIDE_STYLE` handler — Pass 7-7. Aggregates one or more
//! `SURFACE_SIDE_STYLE` entries (each a fill-area or rendering style)
//! into a named composite.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SurfaceSideStyle, SurfaceSideStyleEntry};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_style_fill_area::SurfaceStyleFillAreaHandler;
use step_io_macros::step_entity;

pub(crate) struct SurfaceSideStyleHandler;

#[step_entity(name = "SURFACE_SIDE_STYLE", pass = Pass7SurfaceSide)]
impl SimpleEntityHandler for SurfaceSideStyleHandler {
    type WriteInput = SurfaceSideStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_SIDE_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(entry) = ctx.viz_sss_entry_map.get(&r).cloned() {
                styles.push(entry);
            } else if let Some(&ssr_id) = ctx.viz_ssr_id_map.get(&r) {
                styles.push(SurfaceSideStyleEntry::Rendering(ssr_id));
            }
        }
        ctx.viz_sss_map
            .insert(entity_id, SurfaceSideStyle { name, styles });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sss: SurfaceSideStyle) -> Result<u64, WriteError> {
        let mut style_refs = Vec::with_capacity(sss.styles.len());
        for entry in sss.styles {
            let entry_id = match entry {
                SurfaceSideStyleEntry::FillArea(ssfa) => {
                    SurfaceStyleFillAreaHandler::write(buf, ssfa)?
                }
                SurfaceSideStyleEntry::Rendering(ssr_id) => buf.ssr_step_ids[ssr_id.0 as usize],
            };
            style_refs.push(Attribute::EntityRef(entry_id));
        }
        Ok(buf.push_simple(
            "SURFACE_SIDE_STYLE",
            vec![Attribute::String(sss.name), Attribute::List(style_refs)],
        ))
    }
}
