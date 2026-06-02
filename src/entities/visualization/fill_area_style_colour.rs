//! `FILL_AREA_STYLE_COLOUR` handler. Wraps a colour with a name.
//! Reader resolves the colour ref through `viz_colour_id_map` and stores
//! the `ColourId`; writer looks up the cached STEP id for that arena entry
//! through `WriteBuffer::colour_step_ids`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyleColour;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleColourHandler;

#[step_entity(name = "FILL_AREA_STYLE_COLOUR")]
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
        let Some(&colour) = ctx.viz_colour_id_map.get(&colour_ref) else {
            return Ok(()); // symmetric ignorance — unknown ref skipped
        };
        ctx.viz_fasc_map
            .insert(entity_id, FillAreaStyleColour { name, colour });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, fasc: FillAreaStyleColour) -> Result<u64, WriteError> {
        let colour_step_id = buf.colour_step_ids[fasc.colour.0 as usize];
        Ok(buf.push_simple(
            "FILL_AREA_STYLE_COLOUR",
            vec![
                Attribute::String(fasc.name),
                Attribute::EntityRef(colour_step_id),
            ],
        ))
    }
}
