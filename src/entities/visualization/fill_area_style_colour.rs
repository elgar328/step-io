//! `FILL_AREA_STYLE_COLOUR` handler — Pass 7-2. Wraps a `COLOUR_RGB` with
//! a name. Reader resolves the colour ref through `viz_colour_rgb_map`;
//! writer dispatches the inner colour through `ColourRgbHandler`.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyleColour;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::colour_rgb::ColourRgbHandler;

pub(crate) struct FillAreaStyleColourHandler;

impl SimpleEntityHandler for FillAreaStyleColourHandler {
    const NAME: &'static str = "FILL_AREA_STYLE_COLOUR";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7FillColour;
    type WriteInput = FillAreaStyleColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "FILL_AREA_STYLE_COLOUR")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let colour_ref = read_entity_ref(attrs, 1, entity_id, "fill_colour")?;
        let Some(colour) = ctx.viz_colour_rgb_map.get(&colour_ref).cloned() else {
            return Ok(()); // symmetric ignorance — unknown ref skipped
        };
        ctx.viz_fasc_map
            .insert(entity_id, FillAreaStyleColour { name, colour });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, fasc: FillAreaStyleColour) -> Result<u64, WriteError> {
        let colour_id = ColourRgbHandler::write(buf, fasc.colour)?;
        Ok(buf.push_simple(
            "FILL_AREA_STYLE_COLOUR",
            vec![
                Attribute::String(fasc.name),
                Attribute::EntityRef(colour_id),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static FASC_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: FillAreaStyleColourHandler::NAME,
    pass_level: FillAreaStyleColourHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: FillAreaStyleColourHandler::read,
    },
};
