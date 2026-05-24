//! `CURVE_STYLE` handler — Pass 7-8b. Combines a curve-font reference, a
//! width measure, and a colour reference. Reader resolves the font/colour
//! refs through `viz_pre_defined_curve_font_id_map` / `viz_colour_id_map`
//! and pushes the assembled `CurveStyle` into
//! `VisualizationPool::curve_styles`; writer looks up cached STEP ids for
//! the arena entries through `WriteBuffer::pre_defined_curve_font_step_ids`
//! / `colour_step_ids`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::visualization::{CurveStyle, CurveWidth, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CurveStyleHandler;

#[step_entity(name = "CURVE_STYLE", pass = Pass7CurveStyle)]
impl SimpleEntityHandler for CurveStyleHandler {
    type WriteInput = CurveStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "CURVE_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let font_ref = read_entity_ref(attrs, 1, entity_id, "curve_font")?;
        let Some(&curve_font) = ctx.viz_pre_defined_curve_font_id_map.get(&font_ref) else {
            return Ok(()); // unsupported curve_font SELECT variant — drop
        };
        let curve_width = read_positive_length_measure(attrs, 2, entity_id)?;
        let colour_ref = read_entity_ref(attrs, 3, entity_id, "curve_colour")?;
        let Some(&curve_colour) = ctx.viz_colour_id_map.get(&colour_ref) else {
            return Ok(()); // unsupported colour SELECT variant — drop
        };
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.curve_styles.push(CurveStyle {
            name,
            curve_font,
            curve_width,
            curve_colour,
        });
        ctx.viz_curve_style_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cs: CurveStyle) -> Result<u64, WriteError> {
        let font_step_id = buf.pre_defined_curve_font_step_ids[cs.curve_font.0 as usize];
        let width_attr = match cs.curve_width {
            CurveWidth::PositiveLengthMeasure(v) => Attribute::Typed {
                type_name: "POSITIVE_LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(v)),
            },
        };
        let colour_step_id = buf.colour_step_ids[cs.curve_colour.0 as usize];
        Ok(buf.push_simple(
            "CURVE_STYLE",
            vec![
                Attribute::String(cs.name),
                Attribute::EntityRef(font_step_id),
                width_attr,
                Attribute::EntityRef(colour_step_id),
            ],
        ))
    }
}

/// Extract the wrapped real from a `POSITIVE_LENGTH_MEASURE(x)` typed
/// attribute. Other SELECT members (e.g. `DESCRIPTIVE_MEASURE`) are not
/// modelled; encountering them yields a typed-mismatch error.
fn read_positive_length_measure(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
) -> Result<CurveWidth, ConvertError> {
    let attr = attrs.get(index).ok_or(ConvertError::AttributeIndex {
        entity_id,
        field_name: "curve_width",
        index,
        len: attrs.len(),
    })?;
    match attr {
        Attribute::Typed { type_name, value } if type_name == "POSITIVE_LENGTH_MEASURE" => {
            match value.as_ref() {
                Attribute::Real(v) => Ok(CurveWidth::PositiveLengthMeasure(*v)),
                #[allow(clippy::cast_precision_loss)]
                Attribute::Integer(v) => Ok(CurveWidth::PositiveLengthMeasure(*v as f64)),
                other => Err(ConvertError::AttributeType {
                    entity_id,
                    field_name: "curve_width",
                    expected: "Real inside POSITIVE_LENGTH_MEASURE",
                    actual: AttributeKindTag::from_attribute(other),
                }),
            }
        }
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name: "curve_width",
            expected: "POSITIVE_LENGTH_MEASURE(Real)",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}
