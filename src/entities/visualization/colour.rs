//! `COLOUR` handler — the attributeless base colour entity (`ENTITY colour;
//! END_ENTITY;`, ISO 10303-46). Schema-valid to instantiate directly as
//! `COLOUR()`; NIST fixtures use it as an unspecified-colour placeholder.
//! Reader pushes a `Colour::Itself` variant into `VisualizationPool::colours`
//! and records the `ColourId` in `viz_colour_id_map` so the consumer path
//! (`FILL_AREA_STYLE_COLOUR`, `SURFACE_STYLE_RENDERING`) resolves it through
//! the same map as the RGB / pre-defined flavours.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Colour, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct ColourHandler;

#[step_entity(name = "COLOUR")]
impl SimpleEntityHandler for ColourHandler {
    type WriteInput = ();

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 0, entity_id, "COLOUR")?;
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.colours.push(Colour::Itself);
        ctx.viz_colour_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (): ()) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COLOUR".into(),
                attrs: vec![],
            },
        });
        Ok(n)
    }
}
