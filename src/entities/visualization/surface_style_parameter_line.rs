//! `SURFACE_STYLE_PARAMETER_LINE` handler — phase sspl.
//!
//! `founded_item` SUBTYPE with a `curve_or_render` SELECT and a
//! SET[1:2] OF `direction_count_select` (U / V typed integers).
//! Re-uses the `CurveOrRender` SELECT helpers from
//! [`super::surface_style_boundary`]. EXPRESS WHERE rule (HIINDEX = 1
//! XOR member types differ) is not enforced on read — lenient. Empty
//! SETs or fully unrecognised members drop the carrier, symmetric on
//! re-read.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::surface_style_boundary::{
    emit_curve_or_render, resolve_curve_or_render,
};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    DirectionCount, FoundedItem, SurfaceStyleParameterLine, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleParameterLineHandler;

#[step_entity(name = "SURFACE_STYLE_PARAMETER_LINE")]
impl SimpleEntityHandler for SurfaceStyleParameterLineHandler {
    type WriteInput = SurfaceStyleParameterLine;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_PARAMETER_LINE")?;
        let style_ref = read_entity_ref(attrs, 0, entity_id, "style_of_parameter_lines")?;
        let Some(style) = resolve_curve_or_render(ctx, style_ref) else {
            return Ok(());
        };
        let Attribute::List(items) = &attrs[1] else {
            return Ok(());
        };
        let mut direction_counts = Vec::with_capacity(items.len());
        for item in items {
            if let Some(dc) = parse_direction_count(item) {
                direction_counts.push(dc);
            }
        }
        if direction_counts.is_empty() {
            return Ok(());
        }
        ctx.visualization
            .get_or_insert_with(VisualizationPool::default)
            .founded_items
            .push(FoundedItem::SurfaceStyleParameterLine(
                SurfaceStyleParameterLine {
                    style_of_parameter_lines: style,
                    direction_counts,
                },
            ));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sspl: SurfaceStyleParameterLine) -> Result<u64, WriteError> {
        let style_step = emit_curve_or_render(buf, sspl.style_of_parameter_lines);
        let members: Vec<Attribute> = sspl
            .direction_counts
            .into_iter()
            .map(direction_count_to_attr)
            .collect();
        Ok(buf.push_simple(
            "SURFACE_STYLE_PARAMETER_LINE",
            vec![Attribute::EntityRef(style_step), Attribute::List(members)],
        ))
    }
}

fn parse_direction_count(attr: &Attribute) -> Option<DirectionCount> {
    let Attribute::Typed { type_name, value } = attr else {
        return None;
    };
    let Attribute::Integer(n) = value.as_ref() else {
        return None;
    };
    match type_name.as_str() {
        "U_DIRECTION_COUNT" => Some(DirectionCount::U(*n)),
        "V_DIRECTION_COUNT" => Some(DirectionCount::V(*n)),
        _ => None,
    }
}

fn direction_count_to_attr(dc: DirectionCount) -> Attribute {
    let (type_name, n) = match dc {
        DirectionCount::U(n) => ("U_DIRECTION_COUNT", n),
        DirectionCount::V(n) => ("V_DIRECTION_COUNT", n),
    };
    Attribute::Typed {
        type_name: type_name.into(),
        value: Box::new(Attribute::Integer(n)),
    }
}
