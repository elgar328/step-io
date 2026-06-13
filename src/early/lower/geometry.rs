//! Geometry-domain `lower` fns (the W-RECURSIVE pilot: leaf points/directions
//! + vertex). See the [module docs](super) for the lowering contract.
//!
//! The 2D/3D `CARTESIAN_POINT` / `DIRECTION` sister handlers both run on the
//! same entity and self-claim by coordinate count; these lowers reproduce the
//! 3D branch exactly (2-count → the 2D arena claims it, so we no-op).

use crate::early::model::{
    EarlyCartesianPoint, EarlyDirection, EarlyLine, EarlyVector, EarlyVertexPoint,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Direction3, Line3, Point3, Vertex};
use crate::reader::ReaderContext;

/// Lower one 3D `CARTESIAN_POINT`. A 2-coordinate point belongs to the 2D
/// sister arena (no-op here); anything but 2 or 3 is malformed (Err, matching
/// the legacy handler).
pub(crate) fn lower_cartesian_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCartesianPoint,
) -> Result<(), ConvertError> {
    let coords = early.coordinates;
    match coords.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("CARTESIAN_POINT must have 2 or 3 coordinates, got {n}"),
            });
        }
    }
    let id = ctx.geometry.points.push(Point3 {
        x: coords[0],
        y: coords[1],
        z: coords[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one 3D `DIRECTION` (same 2D/3D self-claim shape as the point).
pub(crate) fn lower_direction(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDirection,
) -> Result<(), ConvertError> {
    let ratios = early.direction_ratios;
    match ratios.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("DIRECTION must have 2 or 3 direction_ratios, got {n}"),
            });
        }
    }
    let id = ctx.geometry.directions.push(Direction3 {
        x: ratios[0],
        y: ratios[1],
        z: ratios[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `VERTEX_POINT` (resolves `vertex_geometry` through the shared
/// point resolver; a dangling ref surfaces as `MissingReference`). Registers
/// in the `vertex_map` named cache (not `id_cache`), as the legacy handler did.
pub(crate) fn lower_vertex_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVertexPoint,
) -> Result<(), ConvertError> {
    let point = ctx.resolve_point(entity_id, early.vertex_geometry, "vertex_geometry")?;
    let id = ctx.geometry.vertices.push(Vertex { point });
    ctx.vertex_map.insert(entity_id, id);
    Ok(())
}

/// Lower one `VECTOR` (2D sister claims a 2D-direction orientation; a 3D
/// orientation resolves to a `DirectionId`, stored in the `vector_map` named
/// cache as the legacy handler did).
pub(crate) fn lower_vector(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVector,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Direction2dId>(early.orientation)
    {
        return Ok(());
    }
    let dir_id = ctx.resolve_direction(entity_id, early.orientation, "orientation")?;
    ctx.vector_map.insert(entity_id, (dir_id, early.magnitude));
    Ok(())
}

/// Lower one `LINE` (2D sister claims a 2D-point pnt; otherwise resolve the
/// point and the VECTOR ref into `Line3`).
pub(crate) fn lower_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLine,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::Point2dId>(early.pnt) {
        return Ok(());
    }
    let point = ctx.resolve_point(entity_id, early.pnt, "pnt")?;
    let (direction, magnitude) = ctx.resolve_vector(entity_id, early.dir, "dir")?;
    let id = ctx
        .geometry
        .curves
        .push(crate::ir::geometry::Curve::Line(Line3 {
            point,
            direction,
            magnitude,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}
