//! Tessellation-domain `lower` fns (clean leaf entities). See the
//! [module docs](super) for the lowering contract. Each `lower` reproduces the
//! previous handler's `read` body 1:1 — same arena, same resolution / drop.

use crate::early::model::{
    EarlyComplexTriangulatedFace, EarlyComplexTriangulatedSurfaceSet, EarlyCoordinatesList,
    EarlyRepositionedTessellatedGeometricSet, EarlyRepositionedTessellatedItem,
    EarlyTessellatedCurveSet, EarlyTessellatedGeometricSet, EarlyTessellatedShell,
    EarlyTessellatedSolid,
};
use crate::entities::tessellation::resolve_tessellated_item_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::tessellation::{
    ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList,
    RepositionedTessellatedGeometricSet, RepositionedTessellatedItem, TessellatedCurveSet,
    TessellatedGeometricSet, TessellatedItem, TessellatedItemRef, TessellatedShell,
    TessellatedSolid,
};
use crate::reader::ReaderContext;

/// Lower one `COORDINATES_LIST` (pure scalar/grid leaf — no resolution).
pub(crate) fn lower_coordinates_list(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCoordinatesList,
) {
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::CoordinatesList(CoordinatesList {
            name: early.name,
            npoints: early.npoints,
            position_coords: early.position_coords,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_CURVE_SET`. Unresolved `coordinates` drops the entity.
pub(crate) fn lower_tessellated_curve_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedCurveSet,
) {
    let Some(coordinates) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(early.coordinates)
    else {
        return;
    };
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::TessellatedCurveSet(TessellatedCurveSet {
            name: early.name,
            coordinates,
            line_strips: early.line_strips,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COMPLEX_TRIANGULATED_FACE` → the `tessellated_faces` arena.
/// Unresolved `coordinates` drops the face; the optional `geometric_link`
/// resolves through the shared representation-item resolver (skipped if
/// absent/unresolved).
pub(crate) fn lower_complex_triangulated_face(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyComplexTriangulatedFace,
) {
    let Some(coordinates) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(early.coordinates)
    else {
        return; // coordinates_list dropped — drop the face too
    };
    let geometric_link = early
        .geometric_link
        .and_then(|r| resolve_representation_item_ref(ctx, r));
    let id = ctx.tessellated_faces.push(ComplexTriangulatedFace {
        name: early.name,
        coordinates,
        pnmax: early.pnmax,
        normals: early.normals,
        geometric_link,
        pnindex: early.pnindex,
        triangle_strips: early.triangle_strips,
        triangle_fans: early.triangle_fans,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COMPLEX_TRIANGULATED_SURFACE_SET` → the `tessellated_surface_sets`
/// arena. Unresolved `coordinates` drops the entity.
pub(crate) fn lower_complex_triangulated_surface_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyComplexTriangulatedSurfaceSet,
) {
    let Some(coordinates) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(early.coordinates)
    else {
        return;
    };
    let id = ctx
        .tessellated_surface_sets
        .push(ComplexTriangulatedSurfaceSet {
            name: early.name,
            coordinates,
            pnmax: early.pnmax,
            normals: early.normals,
            pnindex: early.pnindex,
            triangle_strips: early.triangle_strips,
            triangle_fans: early.triangle_fans,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_GEOMETRIC_SET`. Unresolved children are filtered out.
pub(crate) fn lower_tessellated_geometric_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedGeometricSet,
) {
    let children: Vec<TessellatedItemRef> = early
        .children
        .iter()
        .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
        .collect();
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::TessellatedGeometricSet(
            TessellatedGeometricSet {
                name: early.name,
                children,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `REPOSITIONED_TESSELLATED_GEOMETRIC_SET` (5-part complex MI). Mirrors
/// the plain TGS lower plus a `location` resolved through `placement_map`
/// (drop-if-unresolved, as the legacy hand reader did).
pub(crate) fn lower_repositioned_tessellated_geometric_set(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRepositionedTessellatedGeometricSet,
) {
    let Some(&location) = ctx.placement_map.get(&early.location) else {
        return;
    };
    let children: Vec<TessellatedItemRef> = early
        .children
        .iter()
        .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
        .collect();
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::RepositionedTessellatedGeometricSet(
            RepositionedTessellatedGeometricSet {
                name: early.name,
                location,
                children,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_SOLID`. Unresolved items filtered; `geometric_link`
/// resolves through the solid arena (skipped if absent/unresolved).
pub(crate) fn lower_tessellated_solid(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedSolid,
) {
    let items: Vec<TessellatedItemRef> = early
        .items
        .iter()
        .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
        .collect();
    let geometric_link = early
        .geometric_link
        .and_then(|r| ctx.id_cache.get::<crate::ir::id::SolidId>(r));
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::TessellatedSolid(TessellatedSolid {
            name: early.name,
            items,
            geometric_link,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_SHELL`. Unresolved items filtered; `topological_link`
/// resolves through the shell arena (skipped if absent/unresolved).
pub(crate) fn lower_tessellated_shell(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedShell,
) {
    let items: Vec<TessellatedItemRef> = early
        .items
        .iter()
        .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
        .collect();
    let topological_link = early
        .topological_link
        .and_then(|r| ctx.id_cache.get::<crate::ir::id::ShellId>(r));
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::TessellatedShell(TessellatedShell {
            name: early.name,
            items,
            topological_link,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `REPOSITIONED_TESSELLATED_ITEM`. Unresolved `location` placement
/// drops the entity.
pub(crate) fn lower_repositioned_tessellated_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRepositionedTessellatedItem,
) {
    let Some(&location) = ctx.placement_map.get(&early.location) else {
        return;
    };
    let id = ctx
        .tessellated_items
        .push(TessellatedItem::RepositionedTessellatedItem(
            RepositionedTessellatedItem {
                name: early.name,
                location,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}
