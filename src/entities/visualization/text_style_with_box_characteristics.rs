//! `TEXT_STYLE_WITH_BOX_CHARACTERISTICS` handler — 2-layer path. Reuses the
//! generated bind/serialize; `characteristics` is a `box_characteristic_select`
//! typed-aggregate (synth select + vec-select).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::TextStyleWithBoxCharacteristics;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TextStyleWithBoxCharacteristicsHandler;

#[step_entity(name = "TEXT_STYLE_WITH_BOX_CHARACTERISTICS")]
impl SimpleEntityHandler for TextStyleWithBoxCharacteristicsHandler {
    type WriteInput = TextStyleWithBoxCharacteristics;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_text_style_with_box_characteristics(entity_id, attrs)?;
        lower::lower_text_style_with_box_characteristics(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextStyleWithBoxCharacteristics) -> Result<u64, WriteError> {
        let appearance_step = t.inherited.character_appearance.emit_select(buf);
        let early = lift::lift_text_style_with_box_characteristics(
            t.inherited.name,
            appearance_step,
            t.characteristics,
        );
        Ok(serialize::serialize_text_style_with_box_characteristics(
            buf, &early,
        ))
    }
}
