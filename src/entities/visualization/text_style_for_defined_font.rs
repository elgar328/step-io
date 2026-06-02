//! `TEXT_STYLE_FOR_DEFINED_FONT` handler — phase text-style-font.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{TextStyleForDefinedFont, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TextStyleForDefinedFontHandler;

#[step_entity(name = "TEXT_STYLE_FOR_DEFINED_FONT")]
impl SimpleEntityHandler for TextStyleForDefinedFontHandler {
    type WriteInput = TextStyleForDefinedFont;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "TEXT_STYLE_FOR_DEFINED_FONT")?;
        let colour_ref = read_entity_ref(attrs, 0, entity_id, "text_colour")?;
        let Some(&text_colour) = ctx.viz_colour_id_map.get(&colour_ref) else {
            return Ok(());
        };
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz
            .text_styles_for_defined_font
            .push(TextStyleForDefinedFont { text_colour });
        ctx.text_style_for_defined_font_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextStyleForDefinedFont) -> Result<u64, WriteError> {
        let colour_step = buf.colour_step_ids[t.text_colour.0 as usize];
        Ok(buf.push_simple(
            "TEXT_STYLE_FOR_DEFINED_FONT",
            vec![Attribute::EntityRef(colour_step)],
        ))
    }
}
