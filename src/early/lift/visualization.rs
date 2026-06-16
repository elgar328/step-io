//! Visualization-domain `lift` fns (founded items, view volume, point style —
//! the pilot cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyAppliedPresentedItem, EarlyCameraModelD3, EarlyCameraModelD3MultiClipping,
    EarlyCameraModelD3WithHlhsr, EarlyCameraUsage, EarlyColour, EarlyColourRgb, EarlyCompositeText,
    EarlyContextDependentOverRidingStyledItem, EarlyCurveStyle, EarlyDraughtingPreDefinedColour,
    EarlyDraughtingPreDefinedCurveFont, EarlyFillAreaStyle, EarlyFillAreaStyleColour,
    EarlyGeometricCurveSet, EarlyGeometricSet, EarlyInvisibility, EarlyMarker, EarlyMarkerSize,
    EarlyNullStyle, EarlyOverRidingStyledItem, EarlyPointStyle, EarlyPreDefinedCurveFont,
    EarlyPreDefinedMarker, EarlyPreDefinedPointMarkerSymbol, EarlyPreDefinedSymbol,
    EarlyPreDefinedTerminatorSymbol, EarlyPresentationLayerAssignment,
    EarlyPresentationStyleAssignment, EarlyPresentationStyleByContext,
    EarlyPresentationStyleSelect, EarlyPresentedItemRepresentation, EarlyShellBasedSurfaceModel,
    EarlyStyledItem, EarlySurfaceSideStyle, EarlySurfaceStyleBoundary, EarlySurfaceStyleFillArea,
    EarlySurfaceStyleTransparent, EarlySurfaceStyleUsage, EarlySymbolColour, EarlySymbolStyle,
    EarlyTextStyleForDefinedFont, EarlyViewVolume,
};
use crate::early::model::{
    EarlyAreaInSet, EarlyDefinedSymbol, EarlyPresentationSet, EarlyPresentationSize,
    EarlySymbolTarget, EarlyTextLiteral,
};
use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::fill_area_style_colour::FillAreaStyleColourHandler;
use crate::ir::visualization::{
    AreaInSet, ContextDependentOverRidingStyledItem, CurveStyle, CurveWidth, DefinedSymbol,
    DefinedSymbolDefinition, FillAreaStyle, Invisibility, InvisibleItem, Marker, MarkerSize,
    OverRidingStyledItem, PlainStyledItem, PointStyle, PresentationSize,
    PresentationSizeAssignment, PresentationStyleAssignmentData, PresentationStyleByContext,
    PsaStyle, StyleContext, StyleContextRef, SurfaceSideStyle, SurfaceSideStyleEntry,
    SurfaceStyleFillArea, SurfaceStyleUsage, SymbolPlacement, SymbolTarget, TextPath, ViewVolume,
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

/// Lift L2 `CurveStyle` → L1. `curve_font` (optional) and `curve_colour`
/// resolve to output step ids; `curve_width` (`size_select`) mirrors the
/// `MarkerSize` lift. L2-required `curve_width` / `curve_colour` are always
/// emitted `Some` into the faithfully-optional L1.
pub(crate) fn lift_curve_style(buf: &WriteBuffer, l2: &CurveStyle) -> EarlyCurveStyle {
    let curve_width = match &l2.curve_width {
        CurveWidth::PositiveLengthMeasure(v) => EarlyMarkerSize::PositiveLength(*v),
        CurveWidth::MeasureWithUnit(id) => EarlyMarkerSize::MeasureWithUnit(buf.step_id(*id)),
        CurveWidth::Descriptive(s) => EarlyMarkerSize::Descriptive(s.clone()),
    };
    EarlyCurveStyle {
        name: l2.name.clone(),
        curve_font: l2.curve_font.map(|id| buf.step_id(id)),
        curve_width: Some(curve_width),
        curve_colour: Some(buf.step_id(l2.curve_colour)),
    }
}

/// Shared `styles` SET lifting for `PRESENTATION_STYLE_ASSIGNMENT` /
/// `PRESENTATION_STYLE_BY_CONTEXT`. Each `PsaStyle` resolves to its output step
/// id (`Surface`/`Point`/`Curve`) or the `NULL_STYLE` placeholder (matching the
/// previous `emit_psa_styles`).
pub(crate) fn lift_psa_styles(
    buf: &WriteBuffer,
    styles: &[PsaStyle],
) -> Vec<EarlyPresentationStyleSelect> {
    styles
        .iter()
        .map(|style| match style {
            // Surface/Point are both `FoundedItemId`; merged to satisfy
            // clippy::match_same_arms (Curve is `CurveStyleId`, distinct type).
            PsaStyle::Surface(id) | PsaStyle::Point(id) => {
                EarlyPresentationStyleSelect::EntityRef(buf.step_id(*id))
            }
            PsaStyle::Curve(id) => EarlyPresentationStyleSelect::EntityRef(buf.step_id(*id)),
            PsaStyle::Null => EarlyPresentationStyleSelect::NullStyle(EarlyNullStyle::Null),
        })
        .collect()
}

/// Lift L2 `PresentationStyleAssignmentData` → L1 (`Itself` carrier).
pub(crate) fn lift_presentation_style_assignment(
    buf: &WriteBuffer,
    data: &PresentationStyleAssignmentData,
) -> EarlyPresentationStyleAssignment {
    EarlyPresentationStyleAssignment {
        styles: lift_psa_styles(buf, &data.styles),
    }
}

/// Lift L2 `PresentationStyleByContext` → L1. `styles` reuses
/// [`lift_psa_styles`]; `style_context` resolves to the representation step id
/// or the (fallible) representation-item ref emit, so this lift takes `&mut`
/// and returns `Result`.
pub(crate) fn lift_presentation_style_by_context(
    buf: &mut WriteBuffer,
    psbc: &PresentationStyleByContext,
) -> Result<EarlyPresentationStyleByContext, WriteError> {
    let styles = lift_psa_styles(buf, &psbc.styles);
    let style_context = match &psbc.style_context {
        StyleContext::Representation(rid) => buf.step_id(*rid),
        StyleContext::Item(item) => buf.emit_representation_item_ref(*item)?,
    };
    Ok(EarlyPresentationStyleByContext {
        styles,
        style_context,
    })
}

/// Lift L2 `Invisibility` → L1. Each `InvisibleItem` (5 variants) resolves to
/// its output step id. `presentation_context` is never emitted (matching the
/// previous handler), so it is ignored here.
pub(crate) fn lift_invisibility(buf: &WriteBuffer, inv: &Invisibility) -> EarlyInvisibility {
    let invisible_items = inv
        .invisible_items
        .iter()
        .map(|item| match item {
            InvisibleItem::AnnotationOccurrence(id) => buf.step_id(*id),
            InvisibleItem::StyledItem(id) => buf.step_id(*id),
            InvisibleItem::Representation(id) => buf.step_id(*id),
            InvisibleItem::DraughtingCallout(id) => buf.step_id(*id),
            InvisibleItem::PresentationLayerAssignment(id) => buf.step_id(*id),
        })
        .collect();
    EarlyInvisibility { invisible_items }
}

/// Shared `styles` SET lifting for the `STYLED_ITEM` family — each PSA id
/// resolves to its output step id.
pub(crate) fn lift_styled_item_styles(
    buf: &WriteBuffer,
    styles: &[crate::ir::id::PresentationStyleAssignmentId],
) -> Vec<u64> {
    styles.iter().map(|psa_id| buf.step_id(*psa_id)).collect()
}

/// Lift one plain `STYLED_ITEM`. `item` resolves through the (fallible)
/// `emit_representation_item_ref`, so this lift takes `&mut` and returns `Result`.
pub(crate) fn lift_styled_item(
    buf: &mut WriteBuffer,
    si: &PlainStyledItem,
) -> Result<EarlyStyledItem, WriteError> {
    let item = buf.emit_representation_item_ref(si.item)?;
    Ok(EarlyStyledItem {
        name: si.name.clone(),
        styles: lift_styled_item_styles(buf, &si.styles),
        item,
    })
}

/// Lift one `OVER_RIDING_STYLED_ITEM`. `over_ridden_style` resolves to the
/// (already-emitted) plain styled item's step id.
pub(crate) fn lift_over_riding_styled_item(
    buf: &mut WriteBuffer,
    osi: &OverRidingStyledItem,
) -> Result<EarlyOverRidingStyledItem, WriteError> {
    let item = buf.emit_representation_item_ref(osi.item)?;
    Ok(EarlyOverRidingStyledItem {
        name: osi.name.clone(),
        styles: lift_styled_item_styles(buf, &osi.styles),
        item,
        over_ridden_style: buf.step_id(osi.over_ridden_style),
    })
}

/// Lift one `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM`. `style_context` members
/// resolve to a representation-item ref emit or a representation-relationship
/// step id (guarded against a `0` / un-emitted RR complex).
pub(crate) fn lift_context_dependent_over_riding_styled_item(
    buf: &mut WriteBuffer,
    cd: &ContextDependentOverRidingStyledItem,
) -> Result<EarlyContextDependentOverRidingStyledItem, WriteError> {
    let item = buf.emit_representation_item_ref(cd.item)?;
    let styles = lift_styled_item_styles(buf, &cd.styles);
    let over_ridden_style = buf.step_id(cd.over_ridden_style);
    let mut style_context = Vec::with_capacity(cd.style_context.len());
    for ctx_ref in &cd.style_context {
        match ctx_ref {
            StyleContextRef::RepresentationItem(target) => {
                style_context.push(buf.emit_representation_item_ref(*target)?);
            }
            StyleContextRef::RepresentationRelationship(rrid) => {
                let step_id = buf.step_id(*rrid);
                if step_id == 0 {
                    // The placement's RR complex was never emitted (its parent
                    // product chain was skipped). Surface rather than writing a
                    // dangling `#0` reference.
                    return Err(WriteError::DanglingId {
                        detail: format!(
                            "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM.style_context \
                             RepresentationRelationshipId({}) has no emitted RR complex",
                            rrid.0
                        ),
                    });
                }
                style_context.push(step_id);
            }
        }
    }
    Ok(EarlyContextDependentOverRidingStyledItem {
        name: cd.name.clone(),
        styles,
        item,
        over_ridden_style,
        style_context,
    })
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

/// Lift one `PRESENTED_ITEM_REPRESENTATION` (refs pre-resolved).
pub(crate) fn lift_presented_item_representation(
    presentation: u64,
    item: u64,
) -> EarlyPresentedItemRepresentation {
    EarlyPresentedItemRepresentation { presentation, item }
}

/// Lift one `APPLIED_PRESENTED_ITEM` (items pre-resolved).
pub(crate) fn lift_applied_presented_item(items: Vec<u64>) -> EarlyAppliedPresentedItem {
    EarlyAppliedPresentedItem { items }
}

/// Lift one `SHELL_BASED_SURFACE_MODEL` (shells = child shell step ids). The
/// legacy writer synthesised an empty `name`, so the lift always sets `name:
/// String::new()`.
pub(crate) fn lift_shell_based_surface_model(
    sbsm_boundary: Vec<u64>,
) -> EarlyShellBasedSurfaceModel {
    EarlyShellBasedSurfaceModel {
        name: String::new(),
        sbsm_boundary,
    }
}

/// Lift one `GEOMETRIC_CURVE_SET` — `elements` = emitted curve refs then point
/// refs (the writer's `(curves, points)` order). Name synthesised empty.
pub(crate) fn lift_geometric_curve_set(elements: Vec<u64>) -> EarlyGeometricCurveSet {
    EarlyGeometricCurveSet {
        name: String::new(),
        elements,
    }
}

/// Lift one `GEOMETRIC_SET` (same shape as `GEOMETRIC_CURVE_SET`).
pub(crate) fn lift_geometric_set(elements: Vec<u64>) -> EarlyGeometricSet {
    EarlyGeometricSet {
        name: String::new(),
        elements,
    }
}

/// Lift one `CAMERA_MODEL_D3` (refs pre-resolved to emitted step ids).
pub(crate) fn lift_camera_model_d3(
    name: String,
    view_reference_system: u64,
    perspective_of_volume: u64,
) -> EarlyCameraModelD3 {
    EarlyCameraModelD3 {
        name,
        view_reference_system,
        perspective_of_volume,
    }
}

/// Lift one `CAMERA_MODEL_D3_WITH_HLHSR` (flat L1; the L2 `inherited` body is
/// flattened by the caller).
pub(crate) fn lift_camera_model_d3_with_hlhsr(
    name: String,
    view_reference_system: u64,
    perspective_of_volume: u64,
    hidden_line_surface_removal: bool,
) -> EarlyCameraModelD3WithHlhsr {
    EarlyCameraModelD3WithHlhsr {
        name,
        view_reference_system,
        perspective_of_volume,
        hidden_line_surface_removal,
    }
}

/// Lift one `CAMERA_MODEL_D3_MULTI_CLIPPING` (flat L1; `shape_clipping` members
/// pre-resolved to emitted surface step ids).
pub(crate) fn lift_camera_model_d3_multi_clipping(
    name: String,
    view_reference_system: u64,
    perspective_of_volume: u64,
    shape_clipping: Vec<u64>,
) -> EarlyCameraModelD3MultiClipping {
    EarlyCameraModelD3MultiClipping {
        name,
        view_reference_system,
        perspective_of_volume,
        shape_clipping,
    }
}

/// Lift one `PRESENTATION_SET` (0-attr carrier).
pub(crate) fn lift_colour() -> EarlyColour {
    EarlyColour {}
}

/// Lift one `TEXT_LITERAL`. `placement`/`font` are the child output step ids
/// (the handler emits the 2D/3D placement and the `font_select` first); `path`
/// passes through as the strict `TextPath`.
pub(crate) fn lift_text_literal(
    name: String,
    literal: String,
    placement: u64,
    alignment: String,
    path: TextPath,
    font: u64,
) -> EarlyTextLiteral {
    EarlyTextLiteral {
        name,
        literal,
        placement,
        alignment,
        path,
        font,
    }
}

pub(crate) fn lift_presentation_set() -> EarlyPresentationSet {
    EarlyPresentationSet {}
}

/// Lift one `AREA_IN_SET` (`area`/`in_set` → cached step ids).
pub(crate) fn lift_area_in_set(buf: &WriteBuffer, ais: AreaInSet) -> EarlyAreaInSet {
    EarlyAreaInSet {
        area: buf.step_id(ais.area),
        in_set: buf.step_id(ais.in_set),
    }
}

/// Lift one `PRESENTATION_SIZE`. `size` emits through the (fallible)
/// `PLANAR_BOX` emitter; `unit` resolves to the cached step id of whichever
/// SELECT variant it carries.
pub(crate) fn lift_presentation_size(
    buf: &mut WriteBuffer,
    ps: PresentationSize,
) -> Result<EarlyPresentationSize, WriteError> {
    let unit = match ps.unit {
        PresentationSizeAssignment::View(id) | PresentationSizeAssignment::Area(id) => {
            buf.step_id(id)
        }
        PresentationSizeAssignment::AreaInSet(id) => buf.step_id(id),
    };
    let size = buf.emit_planar_extent(ps.size)?;
    Ok(EarlyPresentationSize { unit, size })
}

/// Lift one `SYMBOL_TARGET`. `placement` emits through the (fallible)
/// `AXIS2_PLACEMENT_3D` emitter (step-io models only the 3D variant).
pub(crate) fn lift_symbol_target(
    buf: &mut WriteBuffer,
    t: SymbolTarget,
) -> Result<EarlySymbolTarget, WriteError> {
    let SymbolPlacement::Placement3d(placement_id) = t.placement;
    Ok(EarlySymbolTarget {
        name: t.name,
        placement: buf.emit_axis2_placement_3d(placement_id)?,
        x_scale: t.x_scale,
        y_scale: t.y_scale,
    })
}

/// Lift one `DEFINED_SYMBOL` (`definition`/`target` → cached step ids).
pub(crate) fn lift_defined_symbol(buf: &WriteBuffer, d: DefinedSymbol) -> EarlyDefinedSymbol {
    let definition = match d.definition {
        DefinedSymbolDefinition::PreDefinedSymbol(id) => buf.step_id(id),
    };
    EarlyDefinedSymbol {
        name: d.name,
        definition,
        target: buf.step_id(d.target),
    }
}
