//! `lower`: `EarlyModel` (L1) → `StepModel` (L2). The hand-written semantic
//! glue — SELECT resolution, flatten, normalization live here. For the pilot
//! cluster, `lower` reproduces exactly what the previous one-shot handlers
//! produced (same arena, same insertion order), so the writer is untouched and
//! corpus output stays byte-identical.

use crate::early::model::{
    EarlySurfaceSideStyle, EarlySurfaceStyleFillArea, EarlySurfaceStyleFillAreaId,
};
use crate::ir::arena::ArenaId;
use crate::ir::id::SurfaceStyleRenderingId;
use crate::ir::visualization::{
    FoundedItem, SurfaceSideStyle, SurfaceSideStyleEntry, SurfaceStyleFillArea, VisualizationPool,
};
use crate::reader::ReaderContext;

/// Lower one `SURFACE_STYLE_FILL_AREA` into the `founded_items` arena and
/// record the read-side L1→L2 correspondence keyed by a typed
/// [`EarlySurfaceStyleFillAreaId`] (replacing `viz_ssfa_id_map`).
///
/// `fill_area` is resolved through the (un-migrated) `viz_fas_id_map`; an
/// unresolved ref drops the entity, matching the previous handler.
pub(crate) fn lower_surface_style_fill_area(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceStyleFillArea,
) {
    let Some(&fill_area) = ctx.viz_fas_id_map.get(&early.fill_area) else {
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

    let idx = ctx.early.ssfa_lowered.len();
    ctx.early.ssfa_lowered.push(l2);
    let early_id = EarlySurfaceStyleFillAreaId(
        u32::try_from(idx).expect("early surface_style_fill_area overflow"),
    );
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
                ctx.early.ssfa_lowered[ssfa_id.index()],
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
    // Dual-write the still-present sss map for the un-migrated consumer
    // (surface_style_usage). Dropped once ssu migrates (Phase 5 scale-out).
    ctx.viz_sss_id_map.insert(entity_id, id);
}
