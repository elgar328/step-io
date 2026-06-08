//! `POINT_STYLE` handler — phase point-style.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    FoundedItem, Marker, MarkerSize, MarkerType, PointStyle, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PointStyleHandler;

#[step_entity(name = "POINT_STYLE")]
impl SimpleEntityHandler for PointStyleHandler {
    type WriteInput = PointStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "POINT_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let marker = match &attrs[1] {
            Attribute::EntityRef(n) => {
                let Some(id) = ctx.id_cache.get::<crate::ir::id::PreDefinedMarkerId>(*n) else {
                    return Ok(());
                };
                Marker::Predefined(id)
            }
            // `marker_select` is a SELECT whose `marker_type` member is an
            // enumeration; P21 type-tags it as `MARKER_TYPE(.PLUS.)`, which
            // the parser yields as a `Typed` wrapper. NIST fixtures use this
            // form. A bare `Enum` is also accepted for tolerant input.
            Attribute::Typed { type_name, value } if type_name == "MARKER_TYPE" => {
                let Attribute::Enum(token) = value.as_ref() else {
                    return Ok(());
                };
                Marker::Type(marker_type_from_token(token))
            }
            Attribute::Enum(token) => Marker::Type(marker_type_from_token(token)),
            _ => return Ok(()),
        };
        let marker_size = match &attrs[2] {
            Attribute::Typed { type_name, value } => match (type_name.as_str(), value.as_ref()) {
                ("POSITIVE_LENGTH_MEASURE", Attribute::Real(v)) => MarkerSize::PositiveLength(*v),
                ("POSITIVE_LENGTH_MEASURE", Attribute::Integer(v)) => {
                    #[allow(clippy::cast_precision_loss)]
                    let f = *v as f64;
                    MarkerSize::PositiveLength(f)
                }
                ("DESCRIPTIVE_MEASURE", Attribute::String(s)) => MarkerSize::Descriptive(s.clone()),
                _ => return Ok(()),
            },
            Attribute::EntityRef(n) => {
                let Some(id) = ctx.id_cache.get::<crate::ir::id::MeasureWithUnitId>(*n) else {
                    return Ok(());
                };
                MarkerSize::MeasureWithUnit(id)
            }
            _ => return Ok(()),
        };
        let colour_ref = read_entity_ref(attrs, 3, entity_id, "marker_colour")?;
        let Some(marker_colour) = ctx.id_cache.get::<crate::ir::id::ColourId>(colour_ref) else {
            return Ok(());
        };
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz.founded_items.push(FoundedItem::PointStyle(PointStyle {
            name,
            marker,
            marker_size,
            marker_colour,
        }));
        ctx.viz_point_style_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ps: PointStyle) -> Result<u64, WriteError> {
        let marker_attr = match ps.marker {
            Marker::Predefined(id) => Attribute::EntityRef(buf.step_id(id)),
            // Type-tag the enum SELECT member: `MARKER_TYPE(.PLUS.)`.
            Marker::Type(t) => Attribute::Typed {
                type_name: "MARKER_TYPE".into(),
                value: Box::new(Attribute::Enum(marker_type_to_token(&t))),
            },
        };
        let size_attr = match ps.marker_size {
            MarkerSize::PositiveLength(v) => Attribute::Typed {
                type_name: "POSITIVE_LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(v)),
            },
            MarkerSize::MeasureWithUnit(id) => Attribute::EntityRef(buf.step_id(id)),
            MarkerSize::Descriptive(s) => Attribute::Typed {
                type_name: "DESCRIPTIVE_MEASURE".into(),
                value: Box::new(Attribute::String(s)),
            },
        };
        let colour_step = buf.step_id(ps.marker_colour);
        Ok(buf.push_simple(
            "POINT_STYLE",
            vec![
                Attribute::String(ps.name),
                marker_attr,
                size_attr,
                Attribute::EntityRef(colour_step),
            ],
        ))
    }
}

fn marker_type_from_token(token: &str) -> MarkerType {
    match token {
        "DOT" => MarkerType::Dot,
        "X" => MarkerType::X,
        "PLUS" => MarkerType::Plus,
        "ASTERISK" => MarkerType::Asterisk,
        "RING" => MarkerType::Ring,
        "SQUARE" => MarkerType::Square,
        "TRIANGLE" => MarkerType::Triangle,
        other => MarkerType::Other(other.to_owned()),
    }
}

fn marker_type_to_token(t: &MarkerType) -> String {
    match t {
        MarkerType::Dot => "DOT".into(),
        MarkerType::X => "X".into(),
        MarkerType::Plus => "PLUS".into(),
        MarkerType::Asterisk => "ASTERISK".into(),
        MarkerType::Ring => "RING".into(),
        MarkerType::Square => "SQUARE".into(),
        MarkerType::Triangle => "TRIANGLE".into(),
        MarkerType::Other(s) => s.clone(),
    }
}
