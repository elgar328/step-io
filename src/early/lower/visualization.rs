//! Visualization-domain `lower` fns (founded items, view volume, point style —
//! the pilot cluster). See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyAppliedPresentedItem, EarlyCameraModelD3, EarlyCameraModelD3MultiClipping,
    EarlyCameraModelD3WithHlhsr, EarlyCameraUsage, EarlyColour, EarlyColourRgb, EarlyCompositeText,
    EarlyContextDependentOverRidingStyledItem, EarlyCurveStyle, EarlyDraughtingPreDefinedColour,
    EarlyDraughtingPreDefinedCurveFont, EarlyFillAreaStyle, EarlyFillAreaStyleColour,
    EarlyFillAreaStyleId, EarlyGeometricCurveSet, EarlyGeometricSet, EarlyInvisibility,
    EarlyMarker, EarlyMarkerSize, EarlyOverRidingStyledItem, EarlyPointStyle, EarlyPointStyleId,
    EarlyPreDefinedCurveFont, EarlyPreDefinedMarker, EarlyPreDefinedPointMarkerSymbol,
    EarlyPreDefinedSymbol, EarlyPreDefinedTerminatorSymbol, EarlyPresentationLayerAssignment,
    EarlyPresentationStyleAssignment, EarlyPresentationStyleByContext,
    EarlyPresentationStyleSelect, EarlyPresentedItemRepresentation, EarlyShellBasedSurfaceModel,
    EarlyStyledItem, EarlySurfaceSideStyle, EarlySurfaceSideStyleId, EarlySurfaceStyleBoundary,
    EarlySurfaceStyleFillArea, EarlySurfaceStyleFillAreaId, EarlySurfaceStyleRendering,
    EarlySurfaceStyleRenderingWithProperties, EarlySurfaceStyleTransparent, EarlySurfaceStyleUsage,
    EarlySurfaceStyleUsageId, EarlySymbolColour, EarlySymbolStyle, EarlyTextStyleForDefinedFont,
    EarlyViewVolume, EarlyViewVolumeId,
};
use crate::early::model::{
    EarlyAreaInSet, EarlyBoxCharacteristicSelect, EarlyDefinedSymbol, EarlyDirectionCountSelect,
    EarlyPresentationArea, EarlyPresentationSet, EarlyPresentationSize, EarlyPresentationView,
    EarlySurfaceStyleParameterLine, EarlySymbolTarget, EarlyTextLiteral,
    EarlyTextStyleWithBoxCharacteristics,
};
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::id::{
    ColourId, CurveId, CurveStyleId, MeasureWithUnitId, PlanarExtentId, PointId,
    PreDefinedCurveFontId, PreDefinedMarkerId, ShellId, SurfaceStyleRenderingId,
};
use crate::ir::shape_rep::{CameraUsage, RepresentationMap};
use crate::ir::visualization::{
    AppliedPresentedItem, Axis2Placement, BoxCharacteristic, CameraModel, CameraModelD3,
    CameraModelD3MultiClipping, CameraModelD3WithHlhsr, CharacterStyle, Colour, ColourRgb,
    CompositeText, ContextDependentOverRidingStyledItem, CurveOrRender, CurveStyle, CurveWidth,
    DirectionCount, DraughtingPreDefinedColour, DraughtingPreDefinedCurveFont, FillAreaStyle,
    FillAreaStyleColour, FontSelect, FoundedItem, GeometricCurveSet, GeometricRepresentationItem,
    GeometricSet, Invisibility, InvisibleItem, Marker, MarkerSize, OverRidingStyledItem,
    PlainStyledItem, PointStyle, PreDefinedCurveFont, PreDefinedCurveFontData, PreDefinedMarker,
    PreDefinedMarkerData, PreDefinedPointMarkerSymbol, PreDefinedSymbol, PreDefinedSymbolData,
    PreDefinedTerminatorSymbol, PresentationLayerAssignment, PresentationLayerAssignmentItem,
    PresentationReprData, PresentationReprSelect, PresentationRepresentation,
    PresentationStyleAssignment, PresentationStyleAssignmentData, PresentationStyleByContext,
    PresentedItem, PresentedItemRepresentation, PsaStyle, RenderingProperty, ShapeClipping,
    ShellBasedSurfaceModel, StyleContext, StyledItem, SurfaceSideStyle, SurfaceSideStyleEntry,
    SurfaceStyleBoundary, SurfaceStyleFillArea, SurfaceStyleParameterLine, SurfaceStyleRendering,
    SurfaceStyleRenderingData, SurfaceStyleRenderingWithProperties, SurfaceStyleUsage,
    SymbolColour, SymbolStyle, TextLiteral, TextOrCharacter, TextStyle, TextStyleData,
    TextStyleForDefinedFont, TextStyleWithBoxCharacteristics, ViewVolume, VisualizationPool,
};
use crate::ir::visualization::{
    AreaInSet, DefinedSymbol, DefinedSymbolDefinition, PresentationSet, PresentationSize,
    PresentationSizeAssignment, SymbolPlacement, SymbolTarget,
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

/// Lower one `CURVE_STYLE` into its own `curve_styles` arena. `curve_font`
/// (an absent `$` stays `None`), `curve_width` (`size_select`) and `curve_colour`
/// resolve through the `id_cache`; an unresolved ref — or an absent L2-required
/// `curve_width` / `curve_colour` — drops the entity (matching the previous
/// handler). The arena id (`CurveStyleId`) is registered directly, the key the
/// PSA / styled-item consumers probe.
pub(crate) fn lower_curve_style(ctx: &mut ReaderContext, entity_id: u64, early: EarlyCurveStyle) {
    let curve_font = match early.curve_font {
        None => None,
        Some(font_ref) => {
            let Some(id) = ctx.id_cache.get::<PreDefinedCurveFontId>(font_ref) else {
                return; // unsupported curve_font SELECT variant — drop
            };
            Some(id)
        }
    };
    let curve_width = match early.curve_width {
        Some(EarlyMarkerSize::PositiveLength(v)) => CurveWidth::PositiveLengthMeasure(v),
        Some(EarlyMarkerSize::Descriptive(s)) => CurveWidth::Descriptive(s),
        Some(EarlyMarkerSize::MeasureWithUnit(n)) => {
            let Some(id) = ctx.id_cache.get::<MeasureWithUnitId>(n) else {
                return;
            };
            CurveWidth::MeasureWithUnit(id)
        }
        None => return,
    };
    let Some(curve_colour_ref) = early.curve_colour else {
        return;
    };
    let Some(curve_colour) = ctx.id_cache.get::<ColourId>(curve_colour_ref) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .curve_styles
        .push(CurveStyle {
            name: early.name,
            curve_font,
            curve_width,
            curve_colour,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Shared `styles` SET lowering for `PRESENTATION_STYLE_ASSIGNMENT` /
/// `PRESENTATION_STYLE_BY_CONTEXT`. Each member is either an `EntityRef`
/// resolved by multi-probe (`SURFACE_STYLE_USAGE` / `POINT_STYLE` via the typed
/// L1 keys, `CURVE_STYLE` via `CurveStyleId`) or the `NULL_STYLE` placeholder.
/// An entity matching no modelled style flavour is skipped (`FILL_AREA_STYLE`
/// direct / `SYMBOL_STYLE` / etc.), matching the previous `parse_psa_styles`.
/// The non-standard `$`/bare-`.NULL.` forms were normalized (and `NsCase`-
/// recorded) before bind, so the L1 here is already standard.
pub(crate) fn lower_psa_styles(
    ctx: &ReaderContext,
    sels: Vec<EarlyPresentationStyleSelect>,
) -> Vec<PsaStyle> {
    let mut styles = Vec::with_capacity(sels.len());
    for sel in sels {
        match sel {
            EarlyPresentationStyleSelect::EntityRef(r) => {
                if let Some(ssu_id) = ctx
                    .id_cache
                    .get::<EarlySurfaceStyleUsageId>(r)
                    .map(|e| ctx.early.lookup_lowered(e))
                {
                    styles.push(PsaStyle::Surface(ssu_id));
                } else if let Some(ps_id) = ctx
                    .id_cache
                    .get::<EarlyPointStyleId>(r)
                    .map(|e| ctx.early.lookup_lowered(e))
                {
                    styles.push(PsaStyle::Point(ps_id));
                } else if let Some(cs_id) = ctx.id_cache.get::<CurveStyleId>(r) {
                    styles.push(PsaStyle::Curve(cs_id));
                }
                // Unmodelled style flavours (FILL_AREA_STYLE direct, SYMBOL_STYLE,
                // etc.) silently skipped — matches the previous handler.
            }
            EarlyPresentationStyleSelect::NullStyle(_) => styles.push(PsaStyle::Null),
        }
    }
    styles
}

/// Lower one `PRESENTATION_STYLE_ASSIGNMENT` (`Itself` carrier).
pub(crate) fn lower_presentation_style_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPresentationStyleAssignment,
) {
    let styles = lower_psa_styles(ctx, early.styles);
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_style_assignments
        .push(PresentationStyleAssignment::Itself(
            PresentationStyleAssignmentData { styles },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTATION_STYLE_BY_CONTEXT` (PSA subtype). `styles` reuses
/// [`lower_psa_styles`]; `style_context` resolves through the `RepresentationId`
/// probe then `resolve_representation_item_ref` (an unmodelled member drops the
/// carrier, matching the previous handler). Same `presentation_style_assignments`
/// arena.
pub(crate) fn lower_presentation_style_by_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPresentationStyleByContext,
) {
    let style_context = if let Some(rid) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.style_context)
    {
        StyleContext::Representation(rid)
    } else if let Some(item) =
        crate::entities::visualization::styled_item::resolve_representation_item_ref(
            ctx,
            early.style_context,
        )
    {
        StyleContext::Item(item)
    } else {
        return; // unmodelled style_context member — drop carrier
    };
    let styles = lower_psa_styles(ctx, early.styles);
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_style_assignments
        .push(PresentationStyleAssignment::PresentationStyleByContext(
            PresentationStyleByContext {
                styles,
                style_context,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `INVISIBILITY`. `invisible_items` (all-entity SET) resolves by
/// 5-bucket multi-probe; an empty SET (non-standard `SET[1:?]`) drops as
/// `NsCase::EmptyInvisibility`, and a non-empty-but-all-unresolved set surfaces
/// a warning and drops — matching the previous handler. Same arena; the emit is
/// scheduled late (after draughting) in the writer's `emit_pools`.
pub(crate) fn lower_invisibility(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyInvisibility,
) {
    if early.invisible_items.is_empty() {
        ctx.ns_push(
            crate::reader::NsCase::EmptyInvisibility,
            "INVISIBILITY".into(),
            1,
            "dropped (empty invisible_items, non-standard SET[1:?])".into(),
        );
        return;
    }
    let mut invisible_items = Vec::with_capacity(early.invisible_items.len());
    for r in early.invisible_items {
        if let Some(id) = ctx.id_cache.get::<crate::ir::id::StyledItemId>(r) {
            invisible_items.push(InvisibleItem::StyledItem(id));
        } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::AnnotationOccurrenceId>(r) {
            invisible_items.push(InvisibleItem::AnnotationOccurrence(id));
        } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::RepresentationId>(r) {
            invisible_items.push(InvisibleItem::Representation(id));
        } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::DraughtingCalloutId>(r) {
            invisible_items.push(InvisibleItem::DraughtingCallout(id));
        } else if let Some(id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationLayerAssignmentId>(r)
        {
            invisible_items.push(InvisibleItem::PresentationLayerAssignment(id));
        }
        // Dropped item instances are skipped per-item — the entity survives on
        // its resolved items.
    }
    if invisible_items.is_empty() {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from(
                    "INVISIBILITY invisible_items did not resolve to any modelled \
                     styled_item / annotation_occurrence / representation / \
                     draughting_callout / presentation_layer_assignment — dropping",
                ),
            });
        return;
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .invisibilities
        .push(Invisibility {
            invisible_items,
            presentation_context: None,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Shared `styles` SET lowering for the `STYLED_ITEM` family — each ref resolves
/// to a `PresentationStyleAssignmentId`; unresolved refs are skipped (matching
/// the previous handlers).
pub(crate) fn lower_styled_item_styles(
    ctx: &ReaderContext,
    style_refs: Vec<u64>,
) -> Vec<crate::ir::id::PresentationStyleAssignmentId> {
    let mut styles = Vec::with_capacity(style_refs.len());
    for r in style_refs {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    styles
}

/// Lower one plain `STYLED_ITEM`. `item` resolves via the shared
/// `resolve_representation_item_ref`; an unresolved target that was a
/// dangling-cascade drop normalizes (`NsCase::DanglingReferenceDrop`), else
/// surfaces a warning — both drop the entity (matching the previous handler).
pub(crate) fn lower_styled_item(ctx: &mut ReaderContext, entity_id: u64, early: EarlyStyledItem) {
    let styles = lower_styled_item_styles(ctx, early.styles);
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        if ctx.nonstandard_dropped_refs.contains(&early.item) {
            ctx.ns_record(
                crate::reader::NsCase::DanglingReferenceDrop,
                "STYLED_ITEM".to_string(),
                "dropped (dangling/cascade reference)",
            );
            ctx.nonstandard_dropped_refs.insert(entity_id);
            return;
        }
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "STYLED_ITEM target #{} did not resolve to a modelled \
                     representation_item kind (likely cascade from a dropped dependency)",
                    early.item
                ),
            });
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .styled_items
        .push(StyledItem::Plain(PlainStyledItem {
            name: early.name,
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `OVER_RIDING_STYLED_ITEM`. `over_ridden_style` (the `$` form is
/// pre-bind dropped as `NsCase::OrsiOverRiddenUnset` in the handler) resolves to
/// a previously-loaded `StyledItemId`; an unresolved `item` / `over_ridden_style`
/// surfaces a warning and drops (matching the previous handler).
pub(crate) fn lower_over_riding_styled_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyOverRidingStyledItem,
) {
    let styles = lower_styled_item_styles(ctx, early.styles);
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "OVER_RIDING_STYLED_ITEM target #{} did not resolve to a modelled \
                     representation_item kind (likely cascade from a dropped dependency)",
                    early.item
                ),
            });
        return;
    };
    let Some(over_ridden_style) = ctx
        .id_cache
        .get::<crate::ir::id::StyledItemId>(early.over_ridden_style)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "OVER_RIDING_STYLED_ITEM over_ridden_style #{} did not resolve \
                     to a previously-loaded STYLED_ITEM",
                    early.over_ridden_style
                ),
            });
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .styled_items
        .push(StyledItem::OverRiding(OverRidingStyledItem {
            name: early.name,
            styles,
            item,
            over_ridden_style,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM`. `style_context` targets
/// are mostly assembly RR-complexes that only gain an id after the placements
/// materialize, so the styled item is pushed now with an empty `style_context`
/// and the refs are deferred to the `resolve_cdorsi_style_contexts` post-pass
/// via `pending_cdorsi` (matching the previous handler).
pub(crate) fn lower_context_dependent_over_riding_styled_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyContextDependentOverRidingStyledItem,
) {
    let styles = lower_styled_item_styles(ctx, early.styles);
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM target #{} did not resolve \
                     to a modelled representation_item kind (likely cascade from a dropped \
                     dependency)",
                    early.item
                ),
            });
        return;
    };
    let Some(over_ridden_style) = ctx
        .id_cache
        .get::<crate::ir::id::StyledItemId>(early.over_ridden_style)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM over_ridden_style \
                     #{} did not resolve to a previously-loaded STYLED_ITEM",
                    early.over_ridden_style
                ),
            });
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .styled_items
        .push(StyledItem::ContextDependent(
            ContextDependentOverRidingStyledItem {
                name: early.name,
                styles,
                item,
                over_ridden_style,
                style_context: Vec::new(),
            },
        ));
    ctx.id_cache.insert(entity_id, id);
    ctx.pending_cdorsi
        .push(crate::reader::PendingCdorsiStyleContext {
            styled_item_id: id,
            context_refs: early.style_context,
            entity_id,
        });
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

/// Lower one `SURFACE_STYLE_PARAMETER_LINE` (`founded_item` subtype). `style`
/// resolves through `CurveOrRender::resolve_select` (unresolved → drop). The
/// `direction_count_select` members map to the L2 `DirectionCount` enum; an empty
/// set drops the carrier. Round-tripped via the arena only — no id registered
/// (1:1 with the legacy handler).
pub(crate) fn lower_surface_style_parameter_line(
    ctx: &mut ReaderContext,
    early: EarlySurfaceStyleParameterLine,
) {
    let Some(style_of_parameter_lines) =
        CurveOrRender::resolve_select(ctx, early.style_of_parameter_lines)
    else {
        return;
    };
    let direction_counts: Vec<DirectionCount> = early
        .direction_counts
        .into_iter()
        .map(|dc| match dc {
            EarlyDirectionCountSelect::UDirectionCount(n) => DirectionCount::U(n),
            EarlyDirectionCountSelect::VDirectionCount(n) => DirectionCount::V(n),
        })
        .collect();
    if direction_counts.is_empty() {
        return;
    }
    ctx.visualization
        .get_or_insert_with(VisualizationPool::default)
        .founded_items
        .push(FoundedItem::SurfaceStyleParameterLine(
            SurfaceStyleParameterLine {
                style_of_parameter_lines,
                direction_counts,
            },
        ));
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

/// Shared lower for `GEOMETRIC_CURVE_SET` / `GEOMETRIC_SET`: split `elements`
/// into resolvable curves / points (anything else — e.g. a stray surface — is
/// skipped), then record in `curve_set_map` (wireframe flatten path) and the
/// unified `geometric_representation_items` arena (`STYLED_ITEM` target).
fn lower_geometric_set_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: &str,
    elements: &[u64],
    is_curve_set: bool,
) {
    let mut curves = Vec::new();
    let mut points = Vec::new();
    for &r in elements {
        if let Some(cid) = ctx.id_cache.get::<CurveId>(r) {
            curves.push(cid);
        } else if let Some(pid) = ctx.id_cache.get::<PointId>(r) {
            points.push(pid);
        }
    }
    ctx.curve_set_map
        .insert(entity_id, (curves.clone(), points.clone()));
    let variant = if is_curve_set {
        GeometricRepresentationItem::GeometricCurveSet(GeometricCurveSet {
            name: name.to_owned(),
            curves,
            points,
        })
    } else {
        GeometricRepresentationItem::GeometricSet(GeometricSet {
            name: name.to_owned(),
            curves,
            points,
        })
    };
    let gri_id = ctx.geometry.geometric_representation_items.push(variant);
    ctx.curve_set_id_map.insert(entity_id, gri_id);
}

/// Lower one `GEOMETRIC_CURVE_SET`.
pub(crate) fn lower_geometric_curve_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyGeometricCurveSet,
) {
    lower_geometric_set_body(ctx, entity_id, &early.name, &early.elements, true);
}

/// Lower one `GEOMETRIC_SET` (allows loose points / surfaces).
pub(crate) fn lower_geometric_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyGeometricSet,
) {
    lower_geometric_set_body(ctx, entity_id, &early.name, &early.elements, false);
}

/// Shared `CAMERA_MODEL_D3` body resolution (mirrors the legacy
/// `read_cmd3_body`): `view_reference_system` via `placement_map`,
/// `perspective_of_volume` via the `EarlyViewVolumeId` lowered lookup. Either
/// unresolved → `None` (the caller drops the camera, symmetric on re-read).
fn lower_cmd3_body(
    ctx: &ReaderContext,
    name: String,
    vrs_ref: u64,
    pov_ref: u64,
) -> Option<CameraModelD3> {
    let &view_reference_system = ctx.placement_map.get(&vrs_ref)?;
    let perspective_of_volume = ctx
        .id_cache
        .get::<EarlyViewVolumeId>(pov_ref)
        .map(|v| ctx.early.lookup_lowered(v))?;
    Some(CameraModelD3 {
        name,
        view_reference_system,
        perspective_of_volume,
    })
}

/// Lower one `CAMERA_MODEL_D3` into `visualization.camera_models`.
pub(crate) fn lower_camera_model_d3(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCameraModelD3,
) {
    let Some(d3) = lower_cmd3_body(
        ctx,
        early.name,
        early.view_reference_system,
        early.perspective_of_volume,
    ) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .camera_models
        .push(CameraModel::CameraModelD3(d3));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CAMERA_MODEL_D3_WITH_HLHSR` (inherits the D3 body + a boolean).
pub(crate) fn lower_camera_model_d3_with_hlhsr(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCameraModelD3WithHlhsr,
) {
    let Some(inherited) = lower_cmd3_body(
        ctx,
        early.name,
        early.view_reference_system,
        early.perspective_of_volume,
    ) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .camera_models
        .push(CameraModel::CameraModelD3WithHlhsr(
            CameraModelD3WithHlhsr {
                inherited,
                hidden_line_surface_removal: early.hidden_line_surface_removal,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CAMERA_MODEL_D3_MULTI_CLIPPING` (inherits the D3 body + a
/// `shape_clipping` SET). Each clipping ref keeps only the `plane` SELECT member
/// (`SurfaceId` → `ShapeClipping::Plane`); `camera_model_d3_multi_clipping_union`
/// members are not modelled and skipped. An empty result drops the camera,
/// symmetric on re-read.
pub(crate) fn lower_camera_model_d3_multi_clipping(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCameraModelD3MultiClipping,
) {
    let Some(inherited) = lower_cmd3_body(
        ctx,
        early.name,
        early.view_reference_system,
        early.perspective_of_volume,
    ) else {
        return;
    };
    let mut shape_clipping = Vec::with_capacity(early.shape_clipping.len());
    for r in early.shape_clipping {
        if let Some(sid) = ctx.id_cache.get::<crate::ir::id::SurfaceId>(r) {
            shape_clipping.push(ShapeClipping::Plane(sid));
        }
    }
    if shape_clipping.is_empty() {
        return;
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .camera_models
        .push(CameraModel::CameraModelD3MultiClipping(
            CameraModelD3MultiClipping {
                inherited,
                shape_clipping,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTATION_SET` (0-attr carrier) → the `presentation_sets` arena.
pub(crate) fn lower_presentation_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    _early: EarlyPresentationSet,
) {
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_sets
        .push(PresentationSet);
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COLOUR` — the attributeless bare-colour placeholder. Pushes a
/// `Colour::Itself` into the `colours` pool (consumers resolve the `ColourId`
/// through the shared map, as the RGB / pre-defined flavours do).
pub(crate) fn lower_colour(ctx: &mut ReaderContext, entity_id: u64, _early: EarlyColour) {
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .colours
        .push(Colour::Itself);
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TEXT_LITERAL`. `placement` discriminates 2D vs 3D (2D arena via
/// `id_cache`, 3D via `placement_map`); an unresolved placement or an unmodelled
/// `font_select` member drops the carrier (1:1 with the legacy handler). `path`
/// is a strict `TextPath` (bound by the generated `bind_text_path`).
pub(crate) fn lower_text_literal(ctx: &mut ReaderContext, entity_id: u64, early: EarlyTextLiteral) {
    let placement = if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::Placement2dId>(early.placement)
    {
        Axis2Placement::D2(id)
    } else if let Some(&id) = ctx.placement_map.get(&early.placement) {
        Axis2Placement::D3(id)
    } else {
        return;
    };
    let Some(font) = FontSelect::resolve_select(ctx, early.font) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .text_literals
        .push(TextLiteral {
            name: early.name,
            literal: early.literal,
            placement,
            alignment: early.alignment,
            path: early.path,
            font,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TEXT_STYLE_WITH_BOX_CHARACTERISTICS`. `character_appearance`
/// resolves through `CharacterStyle::resolve_select` (an unmodelled glyph-style
/// member drops the carrier). The `box_characteristic_select` members map to the
/// L2 `BoxCharacteristic` enum; an empty set drops the carrier (1:1 with the
/// legacy handler).
pub(crate) fn lower_text_style_with_box_characteristics(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTextStyleWithBoxCharacteristics,
) {
    let Some(character_appearance) =
        CharacterStyle::resolve_select(ctx, early.character_appearance)
    else {
        return;
    };
    let characteristics: Vec<BoxCharacteristic> = early
        .characteristics
        .into_iter()
        .map(|bc| match bc {
            EarlyBoxCharacteristicSelect::Height(v) => BoxCharacteristic::Height(v),
            EarlyBoxCharacteristicSelect::Width(v) => BoxCharacteristic::Width(v),
            EarlyBoxCharacteristicSelect::SlantAngle(v) => BoxCharacteristic::SlantAngle(v),
            EarlyBoxCharacteristicSelect::RotateAngle(v) => BoxCharacteristic::RotateAngle(v),
        })
        .collect();
    if characteristics.is_empty() {
        return;
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .text_styles
        .push(TextStyle::WithBoxCharacteristics(
            TextStyleWithBoxCharacteristics {
                inherited: TextStyleData {
                    name: early.name,
                    character_appearance,
                },
                characteristics,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `AREA_IN_SET`. An unresolved `area`/`in_set` drops the carrier.
pub(crate) fn lower_area_in_set(ctx: &mut ReaderContext, entity_id: u64, early: EarlyAreaInSet) {
    let Some(area) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationRepresentationId>(early.area)
    else {
        return;
    };
    let Some(in_set) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationSetId>(early.in_set)
    else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .area_in_sets
        .push(AreaInSet { area, in_set });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTATION_SIZE`. The `unit` SELECT resolves to `AreaInSet` or
/// (defaulting) a presentation `View`; an unresolved `unit`/`size` drops the
/// carrier. No `id_cache` entry — nothing references a `PRESENTATION_SIZE`.
pub(crate) fn lower_presentation_size(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: EarlyPresentationSize,
) {
    let unit = if let Some(id) = ctx.id_cache.get::<crate::ir::id::AreaInSetId>(early.unit) {
        PresentationSizeAssignment::AreaInSet(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationRepresentationId>(early.unit)
    {
        // Spec narrows to View / Area variants — read cannot tell which without
        // inspecting the arena entry. Default to View; emit reconstructs via
        // the cached step id either way.
        PresentationSizeAssignment::View(id)
    } else {
        return;
    };
    let Some(size) = ctx.id_cache.get::<PlanarExtentId>(early.size) else {
        return;
    };
    ctx.visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_sizes
        .push(PresentationSize { unit, size });
}

/// Lower one `SYMBOL_TARGET` → the geometry `GeometricRepresentationItem` arena.
/// step-io models only the 3D placement; a 2D `axis2_placement` drops the
/// carrier. Records into `symbol_target_id_map` for `DEFINED_SYMBOL`.
pub(crate) fn lower_symbol_target(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySymbolTarget,
) {
    let Some(&placement_id) = ctx.placement_map.get(&early.placement) else {
        return;
    };
    let id = ctx.geometry.geometric_representation_items.push(
        GeometricRepresentationItem::SymbolTarget(SymbolTarget {
            name: early.name,
            placement: SymbolPlacement::Placement3d(placement_id),
            x_scale: early.x_scale,
            y_scale: early.y_scale,
        }),
    );
    ctx.symbol_target_id_map.insert(entity_id, id);
}

/// Lower one `DEFINED_SYMBOL`. `definition` resolves through the
/// pre-defined-symbol map (other members drop the carrier); `target` resolves
/// through `symbol_target_id_map`.
pub(crate) fn lower_defined_symbol(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDefinedSymbol,
) {
    let Some(pds_id) = ctx
        .id_cache
        .get::<crate::ir::id::PreDefinedSymbolId>(early.definition)
    else {
        return;
    };
    let Some(&target) = ctx.symbol_target_id_map.get(&early.target) else {
        return;
    };
    let id = ctx.geometry.geometric_representation_items.push(
        GeometricRepresentationItem::DefinedSymbol(DefinedSymbol {
            name: early.name,
            definition: DefinedSymbolDefinition::PreDefinedSymbol(pds_id),
            target,
        }),
    );
    ctx.defined_symbol_id_map.insert(entity_id, id);
}

/// Lower one `SURFACE_STYLE_RENDERING` (`Itself`). `surface_colour` resolves
/// through the shared colour cache (the handler pre-bind injected a placeholder
/// `COLOUR()` for a non-standard Unset, so the ref always resolves); an
/// unresolved ref drops the entity (legacy leniency).
pub(crate) fn lower_surface_style_rendering(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceStyleRendering,
) {
    let Some(surface_colour) = ctx.id_cache.get::<ColourId>(early.surface_colour) else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .surface_style_renderings
        .push(SurfaceStyleRendering::Itself(SurfaceStyleRenderingData {
            rendering_method: early.rendering_method,
            surface_colour,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`. Same colour resolution;
/// `properties` keeps only the modelled `SURFACE_STYLE_TRANSPARENT` members
/// (others silently dropped, preserving round-trip on the supported subset).
pub(crate) fn lower_surface_style_rendering_with_properties(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySurfaceStyleRenderingWithProperties,
) {
    let Some(surface_colour) = ctx.id_cache.get::<ColourId>(early.surface_colour) else {
        return;
    };
    let mut properties = Vec::with_capacity(early.properties.len());
    for r in early.properties {
        if let Some(&t) = ctx.viz_transparent_map.get(&r) {
            properties.push(RenderingProperty::Transparent(t));
        }
    }
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .surface_style_renderings
        .push(SurfaceStyleRendering::SurfaceStyleRenderingWithProperties(
            SurfaceStyleRenderingWithProperties {
                rendering_method: early.rendering_method,
                surface_colour,
                properties,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Shared `presentation_representation` subtype lowering: resolve `items`
/// (filter through `resolve_representation_item_ref`) + `context_of_items`
/// (`resolve_repr_context`). An empty item list OR an unresolved context drops
/// the representation (symmetric on re-read; the legacy read dropped on empty
/// items, and a `None` context cannot round-trip through the strict required
/// `context_of_items` ref so it normalizes to a drop). Verbatim resolve of the
/// legacy `read_presentation_repr`.
fn lower_presentation_repr_data(
    ctx: &ReaderContext,
    name: String,
    item_refs: &[u64],
    context_ref: u64,
) -> Option<PresentationReprData> {
    let mut items = Vec::with_capacity(item_refs.len());
    for &r in item_refs {
        if let Some(item) = resolve_representation_item_ref(ctx, r) {
            items.push(item);
        }
    }
    if items.is_empty() {
        return None;
    }
    let context = ctx.resolve_repr_context(context_ref)?;
    Some(PresentationReprData {
        name,
        items,
        context,
    })
}

/// Lower one `PRESENTATION_VIEW` into the shared `presentation_representations` arena.
pub(crate) fn lower_presentation_view(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPresentationView,
) {
    let Some(data) =
        lower_presentation_repr_data(ctx, early.name, &early.items, early.context_of_items)
    else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_representations
        .push(PresentationRepresentation::View(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PRESENTATION_AREA` into the shared `presentation_representations` arena.
pub(crate) fn lower_presentation_area(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPresentationArea,
) {
    let Some(data) =
        lower_presentation_repr_data(ctx, early.name, &early.items, early.context_of_items)
    else {
        return;
    };
    let id = ctx
        .visualization
        .get_or_insert_with(VisualizationPool::default)
        .presentation_representations
        .push(PresentationRepresentation::Area(data));
    ctx.id_cache.insert(entity_id, id);
}
