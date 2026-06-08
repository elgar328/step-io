//! `CURVE_STYLE` handler. Combines a curve-font reference, a
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

#[step_entity(name = "CURVE_STYLE")]
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
        // `curve_font` is OPTIONAL in AP242 (required in AP203/AP214). An
        // omitted `$` is preserved as `None`; a present ref resolves to a
        // modelled pre_defined_curve_font, else the unsupported SELECT variant
        // drops the whole CURVE_STYLE (as before).
        let curve_font = if matches!(attrs.get(1), Some(Attribute::Unset)) {
            None
        } else {
            let font_ref = read_entity_ref(attrs, 1, entity_id, "curve_font")?;
            let Some(&id) = ctx.viz_pre_defined_curve_font_id_map.get(&font_ref) else {
                return Ok(()); // unsupported curve_font SELECT variant — drop
            };
            Some(id)
        };
        let Some(curve_width) = read_curve_width(ctx, attrs, 2, entity_id)? else {
            return Ok(()); // unsupported curve_width SELECT variant — drop
        };
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
        let font_attr = match cs.curve_font {
            Some(id) => Attribute::EntityRef(buf.step_id(id)),
            None => Attribute::Unset,
        };
        let width_attr = match cs.curve_width {
            CurveWidth::PositiveLengthMeasure(v) => Attribute::Typed {
                type_name: "POSITIVE_LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(v)),
            },
            CurveWidth::MeasureWithUnit(id) => Attribute::EntityRef(buf.step_id(id)),
            CurveWidth::Descriptive(s) => Attribute::Typed {
                type_name: "DESCRIPTIVE_MEASURE".into(),
                value: Box::new(Attribute::String(s)),
            },
        };
        let colour_step_id = buf.step_id(cs.curve_colour);
        Ok(buf.push_simple(
            "CURVE_STYLE",
            vec![
                Attribute::String(cs.name),
                font_attr,
                width_attr,
                Attribute::EntityRef(colour_step_id),
            ],
        ))
    }
}

/// Read the `curve_width` `size_select` — mirrors [`MarkerSize`] reading in
/// `point_style`. Three members: inline `POSITIVE_LENGTH_MEASURE(real)` and
/// `DESCRIPTIVE_MEASURE(text)`, plus an entity-ref `measure_with_unit`
/// resolved through `mwu_id_map`. Returns `Ok(None)` when a `measure_with_unit`
/// ref does not resolve (drops the whole `CURVE_STYLE`, consistent with the
/// `curve_font` / colour SELECT-drop policy); other unmodelled forms still
/// yield a typed-mismatch error for visibility.
fn read_curve_width(
    ctx: &ReaderContext,
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
) -> Result<Option<CurveWidth>, ConvertError> {
    let attr = attrs.get(index).ok_or(ConvertError::AttributeIndex {
        entity_id,
        field_name: "curve_width",
        index,
        len: attrs.len(),
    })?;
    match attr {
        Attribute::Typed { type_name, value } => match (type_name.as_str(), value.as_ref()) {
            ("POSITIVE_LENGTH_MEASURE", Attribute::Real(v)) => {
                Ok(Some(CurveWidth::PositiveLengthMeasure(*v)))
            }
            #[allow(clippy::cast_precision_loss)]
            ("POSITIVE_LENGTH_MEASURE", Attribute::Integer(v)) => {
                Ok(Some(CurveWidth::PositiveLengthMeasure(*v as f64)))
            }
            ("DESCRIPTIVE_MEASURE", Attribute::String(s)) => {
                Ok(Some(CurveWidth::Descriptive(s.clone())))
            }
            (_, other) => Err(ConvertError::AttributeType {
                entity_id,
                field_name: "curve_width",
                expected: "POSITIVE_LENGTH_MEASURE(Real) or DESCRIPTIVE_MEASURE(String)",
                actual: AttributeKindTag::from_attribute(other),
            }),
        },
        Attribute::EntityRef(n) => Ok(ctx
            .mwu_id_map
            .get(n)
            .map(|&id| CurveWidth::MeasureWithUnit(id))),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name: "curve_width",
            expected: "POSITIVE_LENGTH_MEASURE(Real), DESCRIPTIVE_MEASURE(String), or measure_with_unit ref",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}
