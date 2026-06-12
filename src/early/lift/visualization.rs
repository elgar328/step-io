//! Visualization-domain `lift` fns (founded items, view volume, point style —
//! the pilot cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyCameraUsage, EarlyColourRgb, EarlyCompositeText, EarlyDraughtingPreDefinedColour,
    EarlyDraughtingPreDefinedCurveFont, EarlyFillAreaStyle, EarlyFillAreaStyleColour, EarlyMarker,
    EarlyMarkerSize, EarlyPointStyle, EarlyPreDefinedCurveFont, EarlyPreDefinedMarker,
    EarlyPreDefinedPointMarkerSymbol, EarlyPreDefinedSymbol, EarlyPreDefinedTerminatorSymbol,
    EarlyPresentationLayerAssignment, EarlySurfaceSideStyle, EarlySurfaceStyleBoundary,
    EarlySurfaceStyleFillArea, EarlySurfaceStyleTransparent, EarlySurfaceStyleUsage,
    EarlySymbolColour, EarlySymbolStyle, EarlyTextStyleForDefinedFont, EarlyViewVolume,
};
use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::fill_area_style_colour::FillAreaStyleColourHandler;
use crate::ir::visualization::{
    FillAreaStyle, Marker, MarkerSize, PointStyle, SurfaceSideStyle, SurfaceSideStyleEntry,
    SurfaceStyleFillArea, SurfaceStyleUsage, ViewVolume,
};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Lift L2 `FillAreaStyle` → L1. The inlined `FILL_AREA_STYLE_COLOUR` members
/// are emitted on demand (so `lift` takes `&mut`); the L1 node carries their
/// output step ids, matching the previous handler's emission order.
pub(crate) fn lift_fill_area_style(
    buf: &mut WriteBuffer,
    l2: &FillAreaStyle,
) -> Result<EarlyFillAreaStyle, WriteError> {
    let mut fill_styles = Vec::with_capacity(l2.fill_styles.len());
    for fasc in &l2.fill_styles {
        fill_styles.push(FillAreaStyleColourHandler::write(buf, fasc.clone())?);
    }
    Ok(EarlyFillAreaStyle {
        name: l2.name.clone(),
        fill_styles,
    })
}

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
    // L2 `PointStyle` is non-optional, so every field is always present `Some`
    // in the faithfully-optional L1.
    EarlyPointStyle {
        name: l2.name.clone(),
        marker: Some(marker),
        marker_size: Some(marker_size),
        marker_colour: Some(buf.step_id(l2.marker_colour)),
    }
}

/// Lift one `PRE_DEFINED_MARKER` (name pass-through).
pub(crate) fn lift_pre_defined_marker(name: String) -> EarlyPreDefinedMarker {
    EarlyPreDefinedMarker { name }
}

/// Lift one `PRE_DEFINED_POINT_MARKER_SYMBOL` (name pass-through).
pub(crate) fn lift_pre_defined_point_marker_symbol(
    name: String,
) -> EarlyPreDefinedPointMarkerSymbol {
    EarlyPreDefinedPointMarkerSymbol { name }
}

/// Lift one `DRAUGHTING_PRE_DEFINED_COLOUR` (name pass-through).
pub(crate) fn lift_draughting_pre_defined_colour(name: String) -> EarlyDraughtingPreDefinedColour {
    EarlyDraughtingPreDefinedColour { name }
}

/// Lift one `COLOUR_RGB` (pass-through).
pub(crate) fn lift_colour_rgb(c: crate::ir::visualization::ColourRgb) -> EarlyColourRgb {
    EarlyColourRgb {
        name: c.name,
        red: c.red,
        green: c.green,
        blue: c.blue,
    }
}

/// Lift one `PRE_DEFINED_CURVE_FONT` (name pass-through).
pub(crate) fn lift_pre_defined_curve_font(name: String) -> EarlyPreDefinedCurveFont {
    EarlyPreDefinedCurveFont { name }
}

/// Lift one `DRAUGHTING_PRE_DEFINED_CURVE_FONT` (name pass-through).
pub(crate) fn lift_draughting_pre_defined_curve_font(
    name: String,
) -> EarlyDraughtingPreDefinedCurveFont {
    EarlyDraughtingPreDefinedCurveFont { name }
}

/// Lift one `PRE_DEFINED_SYMBOL` (name pass-through).
pub(crate) fn lift_pre_defined_symbol(name: String) -> EarlyPreDefinedSymbol {
    EarlyPreDefinedSymbol { name }
}

/// Lift one `PRE_DEFINED_TERMINATOR_SYMBOL` (name pass-through).
pub(crate) fn lift_pre_defined_terminator_symbol(name: String) -> EarlyPreDefinedTerminatorSymbol {
    EarlyPreDefinedTerminatorSymbol { name }
}

/// Lift one `SYMBOL_COLOUR` (colour pre-resolved).
pub(crate) fn lift_symbol_colour(colour_of_symbol: u64) -> EarlySymbolColour {
    EarlySymbolColour { colour_of_symbol }
}

/// Lift one `SURFACE_STYLE_BOUNDARY` (style pre-resolved via `emit_select`).
pub(crate) fn lift_surface_style_boundary(style_of_boundary: u64) -> EarlySurfaceStyleBoundary {
    EarlySurfaceStyleBoundary { style_of_boundary }
}

/// Lift one `TEXT_STYLE_FOR_DEFINED_FONT` (colour pre-resolved).
pub(crate) fn lift_text_style_for_defined_font(text_colour: u64) -> EarlyTextStyleForDefinedFont {
    EarlyTextStyleForDefinedFont { text_colour }
}

/// Lift one `FILL_AREA_STYLE_COLOUR` (colour pre-resolved).
pub(crate) fn lift_fill_area_style_colour(
    name: String,
    fill_colour: u64,
) -> EarlyFillAreaStyleColour {
    EarlyFillAreaStyleColour { name, fill_colour }
}

/// Lift one `SURFACE_STYLE_TRANSPARENT`.
pub(crate) fn lift_surface_style_transparent(transparency: f64) -> EarlySurfaceStyleTransparent {
    EarlySurfaceStyleTransparent { transparency }
}

/// Lift one `SYMBOL_STYLE` (colour pre-resolved).
pub(crate) fn lift_symbol_style(name: String, style_of_symbol: u64) -> EarlySymbolStyle {
    EarlySymbolStyle {
        name,
        style_of_symbol,
    }
}

/// Lift one `COMPOSITE_TEXT` (members pre-resolved via `emit_select`).
pub(crate) fn lift_composite_text(name: String, collected_text: Vec<u64>) -> EarlyCompositeText {
    EarlyCompositeText {
        name,
        collected_text,
    }
}

/// Lift one `CAMERA_USAGE` (both refs pre-resolved).
pub(crate) fn lift_camera_usage(
    mapping_origin: u64,
    mapped_representation: u64,
) -> EarlyCameraUsage {
    EarlyCameraUsage {
        mapping_origin,
        mapped_representation,
    }
}

/// Lift one `PRESENTATION_LAYER_ASSIGNMENT` (items pre-resolved via
/// `emit_select`; legacy always emitted `description` as a String).
pub(crate) fn lift_presentation_layer_assignment(
    name: String,
    description: String,
    assigned_items: Vec<u64>,
) -> EarlyPresentationLayerAssignment {
    EarlyPresentationLayerAssignment {
        name,
        description,
        assigned_items,
    }
}
