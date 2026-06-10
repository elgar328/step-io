//! `lift`: `StepModel` (L2) → `EarlyModel` (L1), the write-side inverse of
//! [`lower`](super::lower). The kernel only ever produced L2, so `lift`
//! synthesizes a valid L1 from it (not a "reconstruction" of a parsed L1).
//!
//! For the pilot cluster `lift` is mechanical: the L2 already carries
//! everything the canonical output form needs. Cross-references are resolved
//! to their **output** step id (`#N`) via the writer's `StepIdCache`, so the
//! L1 node carries the same `#N`-as-`u64` form its read-side counterpart held
//! (read = input `#N`, write = output `#N`). [`serialize`](super::serialize)
//! then emits Part21 text mechanically.

use crate::early::model::{
    EarlyMarker, EarlyMarkerSize, EarlyPointStyle, EarlySurfaceSideStyle,
    EarlySurfaceStyleFillArea, EarlySurfaceStyleUsage, EarlyViewVolume,
};
use crate::ir::visualization::{
    Marker, MarkerSize, PointStyle, SurfaceSideStyle, SurfaceSideStyleEntry, SurfaceStyleFillArea,
    SurfaceStyleUsage, ViewVolume,
};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Lift L2 `SurfaceStyleFillArea` → L1. `fill_area` is resolved to the output
/// step id of the referenced `FILL_AREA_STYLE`.
pub(crate) fn lift_surface_style_fill_area(
    buf: &WriteBuffer,
    l2: &SurfaceStyleFillArea,
) -> EarlySurfaceStyleFillArea {
    EarlySurfaceStyleFillArea {
        fill_area: buf.step_id(l2.fill_area),
    }
}

/// Lift L2 `SurfaceSideStyle` → L1. Each member SELECT entry is resolved to
/// its output step id (the member's kind no longer matters once it is a bare
/// `#N` ref — that distinction was the read-side concern).
pub(crate) fn lift_surface_side_style(
    buf: &WriteBuffer,
    l2: &SurfaceSideStyle,
) -> EarlySurfaceSideStyle {
    let styles = l2
        .styles
        .iter()
        .map(|entry| match entry {
            SurfaceSideStyleEntry::FillArea(id) => buf.step_id(*id),
            SurfaceSideStyleEntry::Rendering(id) => buf.step_id(*id),
        })
        .collect();
    EarlySurfaceSideStyle {
        name: l2.name.clone(),
        styles,
    }
}

/// Lift L2 `ViewVolume` → L1. Unlike the founded-item refs above,
/// `projection_point` / `view_window` are geometry refs emitted on demand
/// (`emit_point` / `emit_planar_extent`), so this lift takes `&mut` and the L1
/// node carries their output step ids. Field order matches the entity's
/// positional encoding so the two sub-entities emit in the same order as the
/// previous handler.
pub(crate) fn lift_view_volume(
    buf: &mut WriteBuffer,
    l2: &ViewVolume,
) -> Result<EarlyViewVolume, WriteError> {
    Ok(EarlyViewVolume {
        projection_type: l2.projection_type,
        projection_point: buf.emit_point(l2.projection_point)?,
        view_plane_distance: l2.view_plane_distance,
        front_plane_distance: l2.front_plane_distance,
        front_plane_clipping: l2.front_plane_clipping,
        back_plane_distance: l2.back_plane_distance,
        back_plane_clipping: l2.back_plane_clipping,
        view_volume_sides_clipping: l2.view_volume_sides_clipping,
        view_window: buf.emit_planar_extent(l2.view_window)?,
    })
}

/// Lift L2 `SurfaceStyleUsage` → L1. `style` resolves to the output step id of
/// the referenced founded item.
pub(crate) fn lift_surface_style_usage(
    buf: &WriteBuffer,
    l2: &SurfaceStyleUsage,
) -> EarlySurfaceStyleUsage {
    EarlySurfaceStyleUsage {
        side: l2.side,
        style: buf.step_id(l2.style),
    }
}

/// Lift L2 `PointStyle` → L1. The marker / size ref variants and the colour
/// resolve to output step ids; the inline enum / measure variants pass through.
pub(crate) fn lift_point_style(buf: &WriteBuffer, l2: &PointStyle) -> EarlyPointStyle {
    let marker = match &l2.marker {
        Marker::Predefined(id) => EarlyMarker::Predefined(buf.step_id(*id)),
        Marker::Type(t) => EarlyMarker::Type(t.clone()),
    };
    let marker_size = match &l2.marker_size {
        MarkerSize::PositiveLength(v) => EarlyMarkerSize::PositiveLength(*v),
        MarkerSize::MeasureWithUnit(id) => EarlyMarkerSize::MeasureWithUnit(buf.step_id(*id)),
        MarkerSize::Descriptive(s) => EarlyMarkerSize::Descriptive(s.clone()),
    };
    EarlyPointStyle {
        name: l2.name.clone(),
        marker,
        marker_size,
        marker_colour: buf.step_id(l2.marker_colour),
    }
}
