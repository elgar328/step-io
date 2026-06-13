//! Visualization-domain `lower` fns (founded items, view volume, point style —
//! the pilot cluster). See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyAppliedPresentedItem, EarlyCameraUsage, EarlyColourRgb, EarlyCompositeText,
    EarlyDraughtingPreDefinedColour, EarlyDraughtingPreDefinedCurveFont, EarlyFillAreaStyle,
    EarlyFillAreaStyleColour, EarlyFillAreaStyleId, EarlyMarker, EarlyMarkerSize, EarlyPointStyle,
    EarlyPointStyleId, EarlyPreDefinedCurveFont, EarlyPreDefinedMarker,
    EarlyPreDefinedPointMarkerSymbol, EarlyPreDefinedSymbol, EarlyPreDefinedTerminatorSymbol,
    EarlyPresentationLayerAssignment, EarlyPresentedItemRepresentation,
    EarlyShellBasedSurfaceModel, EarlySurfaceSideStyle, EarlySurfaceSideStyleId,
    EarlySurfaceStyleBoundary, EarlySurfaceStyleFillArea, EarlySurfaceStyleFillAreaId,
    EarlySurfaceStyleTransparent, EarlySurfaceStyleUsage, EarlySurfaceStyleUsageId,
    EarlySymbolColour, EarlySymbolStyle, EarlyTextStyleForDefinedFont, EarlyViewVolume,
    EarlyViewVolumeId,
};
use crate::ir::id::{
    ColourId, MeasureWithUnitId, PlanarExtentId, PointId, PreDefinedMarkerId, ShellId,
    SurfaceStyleRenderingId,
};
use crate::ir::shape_rep::{CameraUsage, RepresentationMap};
use crate::ir::visualization::{
    AppliedPresentedItem, Colour, ColourRgb, CompositeText, CurveOrRender,
    DraughtingPreDefinedColour, DraughtingPreDefinedCurveFont, FillAreaStyle, FillAreaStyleColour,
    FoundedItem, GeometricRepresentationItem, Marker, MarkerSize, PointStyle, PreDefinedCurveFont,
    PreDefinedCurveFontData, PreDefinedMarker, PreDefinedMarkerData, PreDefinedPointMarkerSymbol,
    PreDefinedSymbol, PreDefinedSymbolData, PreDefinedTerminatorSymbol,
    PresentationLayerAssignment, PresentationLayerAssignmentItem, PresentationReprSelect,
    PresentedItem, PresentedItemRepresentation, ShellBasedSurfaceModel, SurfaceSideStyle,
    SurfaceSideStyleEntry, SurfaceStyleBoundary, SurfaceStyleFillArea, SurfaceStyleUsage,
    SymbolColour, SymbolStyle, TextOrCharacter, TextStyleForDefinedFont, ViewVolume,
    VisualizationPool,
};
use crate::reader::ReaderContext;

/// Lower one `FILL_AREA_STYLE` into the `founded_items` arena. Each
/// `fill_styles` member resolves through the reader's `viz_fasc_map`
/// value-cache (the L2 inlines `FILL_AREA_STYLE_COLOUR`); unresolved members
/// are skipped, matching the previous handler. Registers the typed
/// [`EarlyFillAreaStyleId`] key for the `surface_style_fill_area` consumer.
pub(crate) fn lower_fill_area_style(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyFillAreaStyle,
) {
    let mut fill_styles = Vec::with_capacity(early.fill_styles.len());
    for r in early.fill_styles {
        if let Some(fasc) = ctx.viz_fasc_map.get(&r).cloned() {
            fill_styles.push(fasc);
        }
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::FillAreaStyle(FillAreaStyle {
            name: early.name,
            fill_styles,
        }));
    let early_id: EarlyFillAreaStyleId = ctx.early.record_lowered(id);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `SURFACE_STYLE_FILL_AREA` into the `founded_items` arena and
/// record the read-side L1→L2 correspondence keyed by a typed
/// [`EarlySurfaceStyleFillAreaId`] (replacing `viz_ssfa_id_map`).
///
/// `fill_area` resolves through the typed [`EarlyFillAreaStyleId`] bucket;
/// an unresolved ref drops the entity, matching the previous handler.
pub(crate) fn lower_surface_style_fill_area(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceStyleFillArea,
) {
    let Some(fill_area) = ctx
        .id_cache
        .get::<EarlyFillAreaStyleId>(early.fill_area)
        .map(|e| ctx.early.lookup_lowered(e))
    else {
        return;
    };
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let l2 = pool
        .founded_items
        .push(FoundedItem::SurfaceStyleFillArea(SurfaceStyleFillArea {
            fill_area,
        }));

    // Record the L1→L2 correspondence + register the typed cache key so
    // surface_side_style can resolve this member by L1 type (no viz_ssfa_id_map).
    let early_id: EarlySurfaceStyleFillAreaId = ctx.early.record_lowered(l2);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `SURFACE_SIDE_STYLE`. Each style member is disambiguated by L1
/// type: a `FillArea` member resolves through the typed
/// [`EarlySurfaceStyleFillAreaId`] cache bucket (no `viz_ssfa_id_map`); a
/// `Rendering` member through the existing `SurfaceStyleRenderingId` bucket
/// (unchanged). Unresolved members are dropped, matching the previous handler.
pub(crate) fn lower_surface_side_style(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySurfaceSideStyle,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for r in early.styles {
        if let Some(ssfa_id) = ctx.id_cache.get::<EarlySurfaceStyleFillAreaId>(r) {
            styles.push(SurfaceSideStyleEntry::FillArea(
                ctx.early.lookup_lowered(ssfa_id),
            ));
        } else if let Some(ssr_id) = ctx.id_cache.get::<SurfaceStyleRenderingId>(r) {
            styles.push(SurfaceSideStyleEntry::Rendering(ssr_id));
        }
    }
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = pool
        .founded_items
        .push(FoundedItem::SurfaceSideStyle(SurfaceSideStyle {
            name: early.name,
            styles,
        }));
    // Register the typed cache key so the migrated surface_style_usage resolves
    // this by L1 type (replaces viz_sss_id_map, now removed).
    let early_id: EarlySurfaceSideStyleId = ctx.early.record_lowered(id);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `SURFACE_STYLE_USAGE`. `style` resolves through the typed
/// [`EarlySurfaceSideStyleId`] bucket (no `viz_sss_id_map`); an unresolved ref
/// drops the entity, matching the previous handler. Registers the typed
/// [`EarlySurfaceStyleUsageId`] key for the PSA consumer.
pub(crate) fn lower_surface_style_usage(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceStyleUsage,
) {
    let Some(style) = ctx
        .id_cache
        .get::<EarlySurfaceSideStyleId>(early.style)
        .map(|e| ctx.early.lookup_lowered(e))
    else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::SurfaceStyleUsage(SurfaceStyleUsage {
            side: early.side,
            style,
        }));
    let early_id: EarlySurfaceStyleUsageId = ctx.early.record_lowered(id);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `POINT_STYLE`. Marker / size / colour refs resolve through the
/// existing `id_cache` (`PreDefinedMarkerId` / `MeasureWithUnitId` / `ColourId`);
/// any unresolved ref drops the entity, matching the previous handler.
/// Registers the typed [`EarlyPointStyleId`] key for the PSA consumer.
pub(crate) fn lower_point_style(ctx: &mut ReaderContext, entity_id: u64, early: EarlyPointStyle) {
    // L1 is faithfully optional; L2 `PointStyle` is non-optional, so an absent
    // marker / size / colour collapses to dropping the entity (the collapse the
    // design assigns to `lower`).
    let marker = match early.marker {
        Some(EarlyMarker::Predefined(n)) => {
            let Some(id) = ctx.id_cache.get::<PreDefinedMarkerId>(n) else {
                return;
            };
            Marker::Predefined(id)
        }
        Some(EarlyMarker::Type(t)) => Marker::Type(t),
        None => return,
    };
    let marker_size = match early.marker_size {
        Some(EarlyMarkerSize::PositiveLength(v)) => MarkerSize::PositiveLength(v),
        Some(EarlyMarkerSize::Descriptive(s)) => MarkerSize::Descriptive(s),
        Some(EarlyMarkerSize::MeasureWithUnit(n)) => {
            let Some(id) = ctx.id_cache.get::<MeasureWithUnitId>(n) else {
                return;
            };
            MarkerSize::MeasureWithUnit(id)
        }
        None => return,
    };
    let Some(marker_colour_ref) = early.marker_colour else {
        return;
    };
    let Some(marker_colour) = ctx.id_cache.get::<ColourId>(marker_colour_ref) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::PointStyle(PointStyle {
            name: early.name,
            marker,
            marker_size,
            marker_colour,
        }));
    let early_id: EarlyPointStyleId = ctx.early.record_lowered(id);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `VIEW_VOLUME` into the `founded_items` arena and record the
/// read-side L1→L2 correspondence keyed by [`EarlyViewVolumeId`] (replacing
/// `viz_view_volume_id_map`; the camera-model handlers consult it).
///
/// `projection_point` (`PointId`) and `view_window` (`PlanarExtentId`) resolve
/// through the existing `id_cache`; an unresolved ref drops the entity,
/// matching the previous handler.
pub(crate) fn lower_view_volume(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyViewVolume) {
    let Some(projection_point) = ctx.id_cache.get::<PointId>(early.projection_point) else {
        return;
    };
    let Some(view_window) = ctx.id_cache.get::<PlanarExtentId>(early.view_window) else {
        return;
    };
    let l2 = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::ViewVolume(ViewVolume {
            projection_type: early.projection_type,
            projection_point,
            view_plane_distance: early.view_plane_distance,
            front_plane_distance: early.front_plane_distance,
            front_plane_clipping: early.front_plane_clipping,
            back_plane_distance: early.back_plane_distance,
            back_plane_clipping: early.back_plane_clipping,
            view_volume_sides_clipping: early.view_volume_sides_clipping,
            view_window,
        }));
    let early_id: EarlyViewVolumeId = ctx.early.record_lowered(l2);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `PRE_DEFINED_MARKER` (`Plain` variant).
pub(crate) fn lower_pre_defined_marker(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPreDefinedMarker,
) {
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = viz
        .pre_defined_markers
        .push(PreDefinedMarker::Plain(PreDefinedMarkerData {
            name: early.name,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRE_DEFINED_POINT_MARKER_SYMBOL` (subtype variant in the
/// shared `pre_defined_markers` arena).
pub(crate) fn lower_pre_defined_point_marker_symbol(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPreDefinedPointMarkerSymbol,
) {
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = viz
        .pre_defined_markers
        .push(PreDefinedMarker::PointMarkerSymbol(
            PreDefinedPointMarkerSymbol { name: early.name },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DRAUGHTING_PRE_DEFINED_COLOUR` (`Colour::PreDefined`).
pub(crate) fn lower_draughting_pre_defined_colour(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingPreDefinedColour,
) {
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = pool
        .colours
        .push(Colour::PreDefined(DraughtingPreDefinedColour {
            name: early.name,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COLOUR_RGB` (`Colour::Rgb`).
pub(crate) fn lower_colour_rgb(ctx: &mut ReaderContext, entity_id: u64, early: EarlyColourRgb) {
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = pool.colours.push(Colour::Rgb(ColourRgb {
        name: early.name,
        red: early.red,
        green: early.green,
        blue: early.blue,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRE_DEFINED_CURVE_FONT` (`Plain` variant).
pub(crate) fn lower_pre_defined_curve_font(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPreDefinedCurveFont,
) {
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id =
        viz.pre_defined_curve_fonts
            .push(PreDefinedCurveFont::Plain(PreDefinedCurveFontData {
                name: early.name,
            }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DRAUGHTING_PRE_DEFINED_CURVE_FONT` (`Draughting` variant).
pub(crate) fn lower_draughting_pre_defined_curve_font(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingPreDefinedCurveFont,
) {
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = viz
        .pre_defined_curve_fonts
        .push(PreDefinedCurveFont::Draughting(
            DraughtingPreDefinedCurveFont { name: early.name },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRE_DEFINED_SYMBOL` (`Plain` variant).
pub(crate) fn lower_pre_defined_symbol(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPreDefinedSymbol,
) {
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = pool
        .pre_defined_symbols
        .push(PreDefinedSymbol::Plain(PreDefinedSymbolData {
            name: early.name,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRE_DEFINED_TERMINATOR_SYMBOL` (`Terminator` variant).
pub(crate) fn lower_pre_defined_terminator_symbol(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPreDefinedTerminatorSymbol,
) {
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id =
        pool.pre_defined_symbols
            .push(PreDefinedSymbol::Terminator(PreDefinedTerminatorSymbol {
                name: early.name,
            }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SYMBOL_COLOUR` (unresolved colour = silent drop).
pub(crate) fn lower_symbol_colour(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySymbolColour,
) {
    let Some(colour_of_symbol) = ctx
        .id_cache
        .get::<crate::ir::id::ColourId>(early.colour_of_symbol)
    else {
        return;
    };
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = viz.symbol_colours.push(SymbolColour { colour_of_symbol });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SURFACE_STYLE_BOUNDARY` (unresolved SELECT member = silent
/// drop, legacy leniency). Round-tripped via the arena only — nothing
/// consumes it through a map, so no id is registered.
pub(crate) fn lower_surface_style_boundary(
    ctx: &mut ReaderContext,
    early: &EarlySurfaceStyleBoundary,
) {
    let Some(style) = CurveOrRender::resolve_select(ctx, early.style_of_boundary) else {
        return;
    };
    ctx.visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::SurfaceStyleBoundary(SurfaceStyleBoundary {
            style_of_boundary: style,
        }));
}

/// Lower one `TEXT_STYLE_FOR_DEFINED_FONT` (unresolved colour = silent drop).
pub(crate) fn lower_text_style_for_defined_font(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyTextStyleForDefinedFont,
) {
    let Some(text_colour) = ctx
        .id_cache
        .get::<crate::ir::id::ColourId>(early.text_colour)
    else {
        return;
    };
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = viz
        .text_styles_for_defined_font
        .push(TextStyleForDefinedFont { text_colour });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `FILL_AREA_STYLE_COLOUR` (unresolved colour = silent skip —
/// symmetric ignorance; payload stored in the dispatch-side value map).
pub(crate) fn lower_fill_area_style_colour(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyFillAreaStyleColour,
) {
    let Some(colour) = ctx
        .id_cache
        .get::<crate::ir::id::ColourId>(early.fill_colour)
    else {
        return;
    };
    ctx.viz_fasc_map.insert(
        entity_id,
        FillAreaStyleColour {
            name: early.name,
            colour,
        },
    );
}

/// Lower one `SURFACE_STYLE_TRANSPARENT` (dispatch-side value map).
pub(crate) fn lower_surface_style_transparent(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceStyleTransparent,
) {
    ctx.viz_transparent_map
        .insert(entity_id, early.transparency);
}

/// Lower one `SYMBOL_STYLE` (unresolved colour = silent drop).
pub(crate) fn lower_symbol_style(ctx: &mut ReaderContext, early: EarlySymbolStyle) {
    let Some(style_of_symbol) = ctx
        .id_cache
        .get::<crate::ir::id::SymbolColourId>(early.style_of_symbol)
    else {
        return;
    };
    let viz = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    viz.founded_items
        .push(FoundedItem::SymbolStyle(SymbolStyle {
            name: early.name,
            style_of_symbol,
        }));
}

/// Lower one `COMPOSITE_TEXT`. Members resolved by the `StepSelect` derive;
/// unmodelled members drop silently. EXPRESS SET\[2:?\] cardinality — the
/// carrier drops if fewer than 2 members survived (emit symmetry).
pub(crate) fn lower_composite_text(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCompositeText,
) {
    let mut collected_text = Vec::with_capacity(early.collected_text.len());
    for n in early.collected_text {
        if let Some(tc) = TextOrCharacter::resolve_select(ctx, n) {
            collected_text.push(tc);
        }
    }
    if collected_text.len() < 2 {
        return;
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .composite_texts
        .push(CompositeText {
            name: early.name,
            collected_text,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CAMERA_USAGE` (either side unresolved = silent drop).
pub(crate) fn lower_camera_usage(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCameraUsage,
) {
    let Some(mapping_origin) = ctx
        .id_cache
        .get::<crate::ir::id::CameraModelId>(early.mapping_origin)
    else {
        return;
    };
    let Some(mapped_representation) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.mapped_representation)
    else {
        return;
    };
    let id = ctx
        .representation_maps
        .push(RepresentationMap::CameraUsage(CameraUsage {
            mapping_origin,
            mapped_representation,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTATION_LAYER_ASSIGNMENT` (unresolved SELECT members
/// drop silently per-item).
pub(crate) fn lower_presentation_layer_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPresentationLayerAssignment,
) {
    let mut assigned_items = Vec::with_capacity(early.assigned_items.len());
    for r in early.assigned_items {
        if let Some(item) = PresentationLayerAssignmentItem::resolve_select(ctx, r) {
            assigned_items.push(item);
        }
    }
    let pool = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default);
    let id = pool
        .presentation_layer_assignments
        .push(PresentationLayerAssignment {
            name: early.name,
            description: early.description,
            assigned_items,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTED_ITEM_REPRESENTATION` (either side unresolved =
/// silent drop; `presentation` probes representation-then-set).
pub(crate) fn lower_presented_item_representation(
    ctx: &mut ReaderContext,
    early: &EarlyPresentedItemRepresentation,
) {
    let presentation = if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationRepresentationId>(early.presentation)
    {
        PresentationReprSelect::Representation(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationSetId>(early.presentation)
    {
        PresentationReprSelect::Set(id)
    } else {
        return;
    };
    let Some(item) = ctx
        .id_cache
        .get::<crate::ir::id::AppliedPresentedItemId>(early.item)
    else {
        return;
    };
    let _id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presented_item_representations
        .push(PresentedItemRepresentation { presentation, item });
}

/// Lower one `APPLIED_PRESENTED_ITEM`. `presented_item` SELECT — only the
/// `product_definition` member is modelled; the other PLM members skip
/// per-item (legacy leniency).
pub(crate) fn lower_applied_presented_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedPresentedItem,
) {
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Ok(pid) = ctx.resolve_product_by_pdef(0, r, "presented_item") {
            items.push(PresentedItem::ProductDefinition(pid));
        }
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .applied_presented_items
        .push(AppliedPresentedItem { items });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SHELL_BASED_SURFACE_MODEL`. Resolves the boundary list into
/// `ShellId`s via `id_cache`, caches the per-SBSM list in `sbsm_shells_map`
/// (so MSSR can flatten one or more SBSMs into a `SurfaceBody`), and pushes the
/// item into the unified `geometric_representation_items` arena (indexed by
/// `sbsm_id_map`) so a `STYLED_ITEM` can resolve it as a single id.
pub(crate) fn lower_shell_based_surface_model(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyShellBasedSurfaceModel,
) {
    let shells: Vec<ShellId> = early
        .sbsm_boundary
        .iter()
        .filter_map(|r| ctx.id_cache.get::<ShellId>(*r))
        .collect();
    ctx.sbsm_shells_map.insert(entity_id, shells.clone());
    let gri_id = ctx.geometry.geometric_representation_items.push(
        GeometricRepresentationItem::ShellBasedSurfaceModel(ShellBasedSurfaceModel {
            name: early.name.clone(),
            shells,
        }),
    );
    ctx.sbsm_id_map.insert(entity_id, gri_id);
}
