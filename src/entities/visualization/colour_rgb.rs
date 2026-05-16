//! `COLOUR_RGB` handler — Pass 7-1. Leaf colour record. Reader stores in
//! `viz_colour_rgb_map`; the IR is tree-inline so each downstream consumer
//! clones the record. Writer re-emits a fresh `COLOUR_RGB` per emission —
//! 15 styled items sharing a colour in the source file emit 15 separate
//! entities, mirroring the read-side de-deduplication.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::ColorRgb;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct ColourRgbHandler;

#[step_entity(name = "COLOUR_RGB", pass = Pass7Colour)]
impl SimpleEntityHandler for ColourRgbHandler {
    type WriteInput = ColorRgb;

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
        ctx.viz_colour_rgb_map.insert(
            entity_id,
            ColorRgb {
                name,
                red,
                green,
                blue,
            },
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: ColorRgb) -> Result<u64, WriteError> {
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
