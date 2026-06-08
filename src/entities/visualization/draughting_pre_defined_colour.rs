//! `DRAUGHTING_PRE_DEFINED_COLOUR` handler (shares the colour
//! leaf pass with `COLOUR_RGB`). Pre-defined colours name a stock colour
//! (`"white"`, `"black"`, etc.) instead of carrying RGB triples. Reader
//! pushes a `Colour::PreDefined(...)` variant into `VisualizationPool::colours`
//! and records the resulting `ColourId` in `viz_colour_id_map` so the
//! consumer path (`FILL_AREA_STYLE_COLOUR`,
//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) resolves either flavour
//! through the same map.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Colour, DraughtingPreDefinedColour, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DraughtingPreDefinedColourHandler;

#[step_entity(name = "DRAUGHTING_PRE_DEFINED_COLOUR")]
impl SimpleEntityHandler for DraughtingPreDefinedColourHandler {
    type WriteInput = DraughtingPreDefinedColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DRAUGHTING_PRE_DEFINED_COLOUR")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .colours
            .push(Colour::PreDefined(DraughtingPreDefinedColour { name }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: DraughtingPreDefinedColour) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DRAUGHTING_PRE_DEFINED_COLOUR".into(),
                attrs: vec![Attribute::String(c.name)],
            },
        });
        Ok(n)
    }
}
