//! `FILL_AREA_STYLE_COLOUR` handler — Pass 7-2. Wraps a `COLOUR_RGB` with
//! a name. Reader resolves the colour ref through `viz_colour_rgb_map`;
//! writer dispatches the inner colour through `ColourRgbHandler`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyleColour;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::colour_rgb::ColourRgbHandler;
use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleColourHandler;

#[step_entity(name = "FILL_AREA_STYLE_COLOUR", pass = Pass7FillColour)]
impl SimpleEntityHandler for FillAreaStyleColourHandler {
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
