//! `serialize`: `EarlyModel` (L1) → Part21 text. Purely mechanical — one
//! `push_simple` per L1 node, attributes in EXPRESS positional order. All ref
//! resolution already happened in [`lift`](super::lift), so this layer makes
//! no arena/cache lookups.

use crate::early::model::{EarlySurfaceSideStyle, EarlySurfaceStyleFillArea, EarlyViewVolume};
use crate::ir::visualization::Projection;
use crate::parser::entity::Attribute;
use crate::writer::buffer::WriteBuffer;

/// `SURFACE_STYLE_FILL_AREA(fill_area)`.
pub(crate) fn serialize_surface_style_fill_area(
    buf: &mut WriteBuffer,
    l1: &EarlySurfaceStyleFillArea,
) -> u64 {
    buf.push_simple(
        "SURFACE_STYLE_FILL_AREA",
        vec![Attribute::EntityRef(l1.fill_area)],
    )
}

/// `SURFACE_SIDE_STYLE(name, styles)`.
pub(crate) fn serialize_surface_side_style(
    buf: &mut WriteBuffer,
    l1: &EarlySurfaceSideStyle,
) -> u64 {
    let style_refs = l1.styles.iter().map(|&s| Attribute::EntityRef(s)).collect();
    buf.push_simple(
        "SURFACE_SIDE_STYLE",
        vec![
            Attribute::String(l1.name.clone()),
            Attribute::List(style_refs),
        ],
    )
}

/// [`Projection`] → a STEP `central_or_parallel` enum `Attribute`.
fn projection_attr(p: Projection) -> Attribute {
    Attribute::Enum(
        match p {
            Projection::Central => "CENTRAL",
            Projection::Parallel => "PARALLEL",
        }
        .into(),
    )
}

/// `bool` → a STEP boolean enum `Attribute` (`.T.` / `.F.`).
fn bool_attr(b: bool) -> Attribute {
    Attribute::Enum(if b { "T" } else { "F" }.into())
}

/// `VIEW_VOLUME(...)` (9 attrs). Refs already resolved to step ids by `lift`.
pub(crate) fn serialize_view_volume(buf: &mut WriteBuffer, l1: &EarlyViewVolume) -> u64 {
    buf.push_simple(
        "VIEW_VOLUME",
        vec![
            projection_attr(l1.projection_type),
            Attribute::EntityRef(l1.projection_point),
            Attribute::Real(l1.view_plane_distance),
            Attribute::Real(l1.front_plane_distance),
            bool_attr(l1.front_plane_clipping),
            Attribute::Real(l1.back_plane_distance),
            bool_attr(l1.back_plane_clipping),
            bool_attr(l1.view_volume_sides_clipping),
            Attribute::EntityRef(l1.view_window),
        ],
    )
}
