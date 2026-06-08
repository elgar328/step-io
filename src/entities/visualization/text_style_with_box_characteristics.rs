//! `TEXT_STYLE_WITH_BOX_CHARACTERISTICS` handler — phase text-style-box.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    BoxCharacteristic, CharacterStyle, TextStyle, TextStyleData, TextStyleWithBoxCharacteristics,
    VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TEXT_STYLE_WITH_BOX_CHARACTERISTICS")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let appearance_ref = read_entity_ref(attrs, 1, entity_id, "character_appearance")?;
        let Some(&t4df_id) = ctx.text_style_for_defined_font_id_map.get(&appearance_ref) else {
            // SELECT may target an unmodelled `character_glyph_style_*`
            // member; silently drop the occurrence.
            return Ok(());
        };
        let character_appearance = CharacterStyle::TextStyleForDefinedFont(t4df_id);
        let Attribute::List(items) = &attrs[2] else {
            return Ok(());
        };
        let mut characteristics = Vec::with_capacity(items.len());
        for item in items {
            if let Some(bc) = parse_box_characteristic(item) {
                characteristics.push(bc);
            }
        }
        if characteristics.is_empty() {
            return Ok(());
        }
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz.text_styles.push(TextStyle::WithBoxCharacteristics(
            TextStyleWithBoxCharacteristics {
                inherited: TextStyleData {
                    name,
                    character_appearance,
                },
                characteristics,
            },
        ));
        ctx.text_style_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: TextStyleWithBoxCharacteristics) -> Result<u64, WriteError> {
        let CharacterStyle::TextStyleForDefinedFont(t4df_id) = t.inherited.character_appearance;
        let appearance_step = buf.step_id(t4df_id);
        let chars = t
            .characteristics
            .into_iter()
            .map(|bc| {
                let (name, v) = match bc {
                    BoxCharacteristic::Height(v) => ("BOX_HEIGHT", v),
                    BoxCharacteristic::Width(v) => ("BOX_WIDTH", v),
                    BoxCharacteristic::SlantAngle(v) => ("BOX_SLANT_ANGLE", v),
                    BoxCharacteristic::RotateAngle(v) => ("BOX_ROTATE_ANGLE", v),
                };
                Attribute::Typed {
                    type_name: name.to_string(),
                    value: Box::new(Attribute::Real(v)),
                }
            })
            .collect();
        Ok(buf.push_simple(
            "TEXT_STYLE_WITH_BOX_CHARACTERISTICS",
            vec![
                Attribute::String(t.inherited.name),
                Attribute::EntityRef(appearance_step),
                Attribute::List(chars),
            ],
        ))
    }
}

fn parse_box_characteristic(item: &Attribute) -> Option<BoxCharacteristic> {
    let Attribute::Typed { type_name, value } = item else {
        return None;
    };
    let v = match value.as_ref() {
        Attribute::Real(v) => *v,
        #[allow(clippy::cast_precision_loss)]
        Attribute::Integer(v) => *v as f64,
        _ => return None,
    };
    Some(match type_name.as_str() {
        "BOX_HEIGHT" => BoxCharacteristic::Height(v),
        "BOX_WIDTH" => BoxCharacteristic::Width(v),
        "BOX_SLANT_ANGLE" => BoxCharacteristic::SlantAngle(v),
        "BOX_ROTATE_ANGLE" => BoxCharacteristic::RotateAngle(v),
        _ => return None,
    })
}
