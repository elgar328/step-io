//! `TEXT_LITERAL` handler — phase text-literal.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    Axis2Placement, FontSelect, TextLiteral, TextPath, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "TEXT_LITERAL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let literal = read_string_or_unset(attrs, 1, entity_id, "literal")?.to_owned();
        let placement_ref = read_entity_ref(attrs, 2, entity_id, "placement")?;
        let placement = if let Some(&id) = ctx.placement_2d_map.get(&placement_ref) {
            Axis2Placement::D2(id)
        } else if let Some(&id) = ctx.placement_map.get(&placement_ref) {
            Axis2Placement::D3(id)
        } else {
            return Ok(());
        };
        let alignment = read_string_or_unset(attrs, 3, entity_id, "alignment")?.to_owned();
        let path = match &attrs[4] {
            Attribute::Enum(token) => text_path_from_token(token),
            _ => return Ok(()),
        };
        let font_ref = read_entity_ref(attrs, 5, entity_id, "font")?;
        let Some(&font_id) = ctx.dptf_id_map.get(&font_ref) else {
            // SELECT member may target `externally_defined_text_font` —
            // not modelled in step-io; drop the carrier.
            return Ok(());
        };
        let font = FontSelect::DraughtingPreDefined(font_id);
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz.text_literals.push(TextLiteral {
            name,
            literal,
            placement,
            alignment,
            path,
            font,
        });
        ctx.text_literal_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextLiteral) -> Result<u64, WriteError> {
        let placement_step = match t.placement {
            Axis2Placement::D2(id) => buf.emit_axis2_placement_2d(id)?,
            Axis2Placement::D3(id) => buf.emit_axis2_placement_3d(id)?,
        };
        let FontSelect::DraughtingPreDefined(font_id) = t.font;
        let font_step = buf.step_id(font_id);
        Ok(buf.push_simple(
            "TEXT_LITERAL",
            vec![
                Attribute::String(t.name),
                Attribute::String(t.literal),
                Attribute::EntityRef(placement_step),
                Attribute::String(t.alignment),
                Attribute::Enum(text_path_to_token(&t.path)),
                Attribute::EntityRef(font_step),
            ],
        ))
    }
}

fn text_path_from_token(token: &str) -> TextPath {
    match token {
        "LEFT" => TextPath::Left,
        "RIGHT" => TextPath::Right,
        "UP" => TextPath::Up,
        "DOWN" => TextPath::Down,
        other => TextPath::Other(other.to_owned()),
    }
}

fn text_path_to_token(path: &TextPath) -> String {
    match path {
        TextPath::Left => "LEFT".into(),
        TextPath::Right => "RIGHT".into(),
        TextPath::Up => "UP".into(),
        TextPath::Down => "DOWN".into(),
        TextPath::Other(s) => s.clone(),
    }
}
