//! Geometry-domain `lift` fns (W-RECURSIVE pilot). See the [module docs](super)
//! for the lifting contract. The legacy writer synthesised an empty `name`
//! (`String::new()`), so these lifts always set `name: String::new()`.

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement3d, EarlyCartesianPoint, EarlyCircle, EarlyDirection,
    EarlyLine, EarlyPlane, EarlyVector, EarlyVertexPoint,
};
use crate::ir::geometry::{Direction3, Point3};

/// Lift one `CARTESIAN_POINT` from its arena `Point3`.
pub(crate) fn lift_cartesian_point(p: Point3) -> EarlyCartesianPoint {
    EarlyCartesianPoint {
        name: String::new(),
        coordinates: vec![p.x, p.y, p.z],
    }
}

/// Lift one `DIRECTION` from its arena `Direction3`.
pub(crate) fn lift_direction(d: Direction3) -> EarlyDirection {
    EarlyDirection {
        name: String::new(),
        direction_ratios: vec![d.x, d.y, d.z],
    }
}

/// Lift one `VERTEX_POINT` (the child point's step id is pre-resolved by the
/// handler's `emit_point` recursion).
pub(crate) fn lift_vertex_point(vertex_geometry: u64) -> EarlyVertexPoint {
    EarlyVertexPoint {
        name: String::new(),
        vertex_geometry,
    }
}

/// Lift one `VECTOR` (orientation = child direction's output step id).
pub(crate) fn lift_vector(orientation: u64, magnitude: f64) -> EarlyVector {
    EarlyVector {
        name: String::new(),
        orientation,
        magnitude,
    }
}

/// Lift one `LINE` (pnt/dir = child point/VECTOR output step ids).
pub(crate) fn lift_line(pnt: u64, dir: u64) -> EarlyLine {
    EarlyLine {
        name: String::new(),
        pnt,
        dir,
    }
}

/// Lift one `AXIS1_PLACEMENT` (location/axis = child output step ids; axis is
/// always present in practice, so `Some`).
pub(crate) fn lift_axis1_placement(location: u64, axis: u64) -> EarlyAxis1Placement {
    EarlyAxis1Placement {
        name: String::new(),
        location,
        axis: Some(axis),
    }
}

/// Lift one `AXIS2_PLACEMENT_3D` (optional `axis`/`ref_direction` pass through;
/// `None` → `$`).
pub(crate) fn lift_axis2_placement_3d(
    location: u64,
    axis: Option<u64>,
    ref_direction: Option<u64>,
) -> EarlyAxis2Placement3d {
    EarlyAxis2Placement3d {
        name: String::new(),
        location,
        axis,
        ref_direction,
    }
}

/// Lift one `CIRCLE` (position = child placement step id).
pub(crate) fn lift_circle(position: u64, radius: f64) -> EarlyCircle {
    EarlyCircle {
        name: String::new(),
        position,
        radius,
    }
}

/// Lift one `PLANE` (position = child placement step id).
pub(crate) fn lift_plane(position: u64) -> EarlyPlane {
    EarlyPlane {
        name: String::new(),
        position,
    }
}
