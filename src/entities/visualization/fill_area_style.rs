//! `FILL_AREA_STYLE` handler — Pass 7-3. Aggregates one or more
//! `FILL_AREA_STYLE_COLOUR` entries into a named fill-area style.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyle;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::fill_area_style_colour::FillAreaStyleColourHandler;
use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleHandler;

#[step_entity(name = "FILL_AREA_STYLE", pass = Pass7FillArea)]
impl SimpleEntityHandler for FillAreaStyleHandler {
    type WriteInput = FillAreaStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "FILL_AREA_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "fill_styles")?;
        let mut fill_styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(fasc) = ctx.viz_fasc_map.get(&r).cloned() {
                fill_styles.push(fasc);
            }
        }
        ctx.viz_fas_map
            .insert(entity_id, FillAreaStyle { name, fill_styles });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, fas: FillAreaStyle) -> Result<u64, WriteError> {
        let mut style_refs = Vec::with_capacity(fas.fill_styles.len());
        for fasc in fas.fill_styles {
            style_refs.push(Attribute::EntityRef(FillAreaStyleColourHandler::write(
                buf, fasc,
            )?));
        }
        Ok(buf.push_simple(
            "FILL_AREA_STYLE",
            vec![Attribute::String(fas.name), Attribute::List(style_refs)],
        ))
    }
}
