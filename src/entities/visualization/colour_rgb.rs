//! `COLOUR_RGB` handler. Leaf colour record. Reader pushes the
//! resolved [`ColourRgb`] into `VisualizationPool::colours` as
//! `Colour::Rgb(...)` and records the resulting `ColourId` in
//! `viz_colour_id_map` so downstream consumers can resolve a colour ref
//! to an arena index. Writer takes a `ColourRgb` (extracted from the
//! `Colour::Rgb` variant by the visualization emit pass) and emits a
//! fresh `COLOUR_RGB` entity each call — no semantic dedup.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Colour, ColourRgb, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct ColourRgbHandler;

#[step_entity(name = "COLOUR_RGB")]
impl SimpleEntityHandler for ColourRgbHandler {
    type WriteInput = ColourRgb;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "COLOUR_RGB")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let red = read_real(attrs, 1, entity_id, "red")?;
        let green = read_real(attrs, 2, entity_id, "green")?;
        let blue = read_real(attrs, 3, entity_id, "blue")?;
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.colours.push(Colour::Rgb(ColourRgb {
            name,
            red,
            green,
            blue,
        }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: ColourRgb) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COLOUR_RGB".into(),
                attrs: vec![
                    Attribute::String(c.name),
                    Attribute::Real(c.red),
                    Attribute::Real(c.green),
                    Attribute::Real(c.blue),
                ],
            },
        });
        Ok(n)
    }
}
