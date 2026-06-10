//! `FILL_AREA_STYLE` handler. Aggregates one or more
//! `FILL_AREA_STYLE_COLOUR` entries into a named fill-area style. Pushes
//! into the shared `founded_item` arena as the `FillAreaStyle` variant.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyle;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::fill_area_style_colour::FillAreaStyleColourHandler;
use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleHandler;

#[step_entity(name = "FILL_AREA_STYLE")]
impl SimpleEntityHandler for FillAreaStyleHandler {
    type WriteInput = FillAreaStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves the
        // fill_styles via viz_fasc_map and registers the typed
        // `EarlyFillAreaStyleId` key (replaces viz_fas_id_map).
        let early = bind::bind_fill_area_style(entity_id, attrs)?;
        lower::lower_fill_area_style(ctx, entity_id, early);
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
