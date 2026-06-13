//! Geometry-domain `lift` fns (W-RECURSIVE pilot). See the [module docs](super)
//! for the lifting contract. The legacy writer synthesised an empty `name`
//! (`String::new()`), so these lifts always set `name: String::new()`.

use crate::early::model::{
    EarlyCartesianPoint, EarlyDirection, EarlyLine, EarlyVector, EarlyVertexPoint,
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
