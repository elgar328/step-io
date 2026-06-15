//! Tessellation-domain `lift` fns (clean leaf entities). See the
//! [module docs](super) for the lifting contract. Each `lift` reproduces the
//! previous handler's `write` body 1:1 — refs resolve through the shared
//! emitters, grids/scalars pass through.

use crate::early::model::{
    EarlyComplexTriangulatedSurfaceSet, EarlyCoordinatesList, EarlyRepositionedTessellatedItem,
    EarlyTessellatedCurveSet, EarlyTessellatedGeometricSet, EarlyTessellatedShell,
    EarlyTessellatedSolid,
};
use crate::ir::tessellation::{
    ComplexTriangulatedSurfaceSet, CoordinatesList, RepositionedTessellatedItem,
    TessellatedCurveSet, TessellatedGeometricSet, TessellatedShell, TessellatedSolid,
};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Lift one `COORDINATES_LIST` (pure scalar/grid leaf — no refs, no `buf`).
pub(crate) fn lift_coordinates_list(item: CoordinatesList) -> EarlyCoordinatesList {
    EarlyCoordinatesList {
        name: item.name,
        npoints: item.npoints,
        position_coords: item.position_coords,
    }
}

/// Lift one `TESSELLATED_CURVE_SET` (`coordinates` → cached step id).
pub(crate) fn lift_tessellated_curve_set(
    buf: &WriteBuffer,
    item: TessellatedCurveSet,
) -> EarlyTessellatedCurveSet {
    EarlyTessellatedCurveSet {
        name: item.name,
        coordinates: buf.step_id(item.coordinates),
        line_strips: item.line_strips,
    }
}

/// Lift one `COMPLEX_TRIANGULATED_SURFACE_SET`.
pub(crate) fn lift_complex_triangulated_surface_set(
    buf: &WriteBuffer,
    set: ComplexTriangulatedSurfaceSet,
) -> EarlyComplexTriangulatedSurfaceSet {
    EarlyComplexTriangulatedSurfaceSet {
        name: set.name,
        coordinates: buf.step_id(set.coordinates),
        pnmax: set.pnmax,
        normals: set.normals,
        pnindex: set.pnindex,
        triangle_strips: set.triangle_strips,
        triangle_fans: set.triangle_fans,
    }
}

/// Lift one `TESSELLATED_GEOMETRIC_SET` (children → shared infallible emitter).
pub(crate) fn lift_tessellated_geometric_set(
    buf: &WriteBuffer,
    tgs: TessellatedGeometricSet,
) -> EarlyTessellatedGeometricSet {
    let children = tgs
        .children
        .iter()
        .map(|&r| buf.emit_tessellated_item_ref(r))
        .collect();
    EarlyTessellatedGeometricSet {
        name: tgs.name,
        children,
    }
}

/// Lift one `TESSELLATED_SOLID`. `geometric_link` emits through the (fallible)
/// solid emitter, so this lift takes `&mut` and returns `Result`.
pub(crate) fn lift_tessellated_solid(
    buf: &mut WriteBuffer,
    ts: TessellatedSolid,
) -> Result<EarlyTessellatedSolid, WriteError> {
    let items = ts
        .items
        .iter()
        .map(|&r| buf.emit_tessellated_item_ref(r))
        .collect();
    let geometric_link = match ts.geometric_link {
        Some(id) => Some(buf.emit_solid(id)?),
        None => None,
    };
    Ok(EarlyTessellatedSolid {
        name: ts.name,
        items,
        geometric_link,
    })
}

/// Lift one `TESSELLATED_SHELL`. `topological_link` emits through the (fallible)
/// shell emitter.
pub(crate) fn lift_tessellated_shell(
    buf: &mut WriteBuffer,
    ts: TessellatedShell,
) -> Result<EarlyTessellatedShell, WriteError> {
    let items = ts
        .items
        .iter()
        .map(|&r| buf.emit_tessellated_item_ref(r))
        .collect();
    let topological_link = match ts.topological_link {
        Some(id) => Some(buf.emit_shell(id)?),
        None => None,
    };
    Ok(EarlyTessellatedShell {
        name: ts.name,
        items,
        topological_link,
    })
}

/// Lift one `REPOSITIONED_TESSELLATED_ITEM`. `location` emits through the
/// (fallible) `AXIS2_PLACEMENT_3D` emitter.
pub(crate) fn lift_repositioned_tessellated_item(
    buf: &mut WriteBuffer,
    r: RepositionedTessellatedItem,
) -> Result<EarlyRepositionedTessellatedItem, WriteError> {
    Ok(EarlyRepositionedTessellatedItem {
        name: r.name,
        location: buf.emit_axis2_placement_3d(r.location)?,
    })
}
