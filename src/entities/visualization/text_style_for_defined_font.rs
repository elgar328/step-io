//! `TEXT_STYLE_FOR_DEFINED_FONT` handler — visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::TextStyleForDefinedFont;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_text_style_for_defined_font(entity_id, attrs)?;
        lower::lower_text_style_for_defined_font(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextStyleForDefinedFont) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(t.text_colour);
        let early = lift::lift_text_style_for_defined_font(colour_step);
        Ok(serialize::serialize_text_style_for_defined_font(
            buf, &early,
        ))
    }
}
