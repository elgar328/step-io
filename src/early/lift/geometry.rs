//! Geometry-domain `lift` fns (W-RECURSIVE pilot). See the [module docs](super)
//! for the lifting contract. The legacy writer synthesised an empty `name`
//! (`String::new()`), so these lifts always set `name: String::new()`.

use crate::early::model::{EarlyCartesianPoint, EarlyDirection, EarlyVertexPoint};
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
