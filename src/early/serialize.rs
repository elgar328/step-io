//! `serialize`: `EarlyModel` (L1) → Part21 text. Purely mechanical — one
//! `push_simple` per L1 node, attributes in EXPRESS positional order. All ref
//! resolution already happened in [`lift`](super::lift), so this layer makes
//! no arena/cache lookups.

use crate::early::model::{EarlySurfaceSideStyle, EarlySurfaceStyleFillArea};
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
