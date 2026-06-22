//! `TEXT_LITERAL` handler — 2-layer path. Reuses the generated bind/serialize;
//! `path` is a strict `text_path` enum (non-standard token → drop + NORM).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Axis2Placement, TextLiteral};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TextLiteralHandler;

#[step_entity(name = "TEXT_LITERAL")]
impl SimpleEntityHandler for TextLiteralHandler {
    type WriteInput = TextLiteral;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_text_literal(entity_id, attrs)?;
        lower::lower_text_literal(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextLiteral) -> Result<u64, WriteError> {
        let placement_step = match t.placement {
            Axis2Placement::D2(id) => buf.emit_axis2_placement_2d(id)?,
            Axis2Placement::D3(id) => buf.emit_axis2_placement_3d(id)?,
        };
        let font_step = t.font.emit_select(buf);
        let early = lift::lift_text_literal(
            t.name,
            t.literal,
            placement_step,
            t.alignment,
            t.path,
            font_step,
        );
        Ok(serialize::serialize_text_literal(buf, &early))
    }
}
