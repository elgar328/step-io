//! `POINT_STYLE` handler — phase point-style.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Marker, MarkerSize, MarkerType, PointStyle};
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
        // 2-layer path: bind → L1 (Ok(None) = unrecognized SELECT form → drop),
        // then lower → L2. `lower` resolves marker/size/colour refs via the
        // existing id_cache and registers the typed `EarlyPointStyleId` key
        // (replaces viz_point_style_id_map).
        if let Some(early) = bind::bind_point_style(entity_id, attrs)? {
            lower::lower_point_style(ctx, entity_id, early);
        }
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
