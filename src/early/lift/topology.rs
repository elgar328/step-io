//! Topology-domain `lift` fns (W-RECURSIVE spread). See the [module docs](super)
//! for the lifting contract. Child refs are pre-resolved to output step ids by
//! the handler's emit recursion; `name` is the legacy empty string.

use crate::early::model::{
    EarlyAdvancedFace, EarlyClosedShell, EarlyFaceBound, EarlyFaceOuterBound, EarlyFaceSurface,
    EarlyManifoldSolidBrep, EarlyOpenShell, EarlyVertexLoop,
};
use crate::ir::topology::Orientation;

/// `Orientation` → the BOOLEAN the legacy writer emitted (`Forward` = `T`).
fn orientation_to_bool(o: Orientation) -> bool {
    matches!(o, Orientation::Forward)
}

/// Lift one `FACE_BOUND` (bound = the loop's output step id).
pub(crate) fn lift_face_bound(bound: u64, orientation: Orientation) -> EarlyFaceBound {
    EarlyFaceBound {
        name: String::new(),
        bound,
        orientation: orientation_to_bool(orientation),
    }
}

/// Lift one `FACE_OUTER_BOUND`.
pub(crate) fn lift_face_outer_bound(bound: u64, orientation: Orientation) -> EarlyFaceOuterBound {
    EarlyFaceOuterBound {
        name: String::new(),
        bound,
        orientation: orientation_to_bool(orientation),
    }
}

/// Lift one `OPEN_SHELL` (faces = child face output step ids).
pub(crate) fn lift_open_shell(cfs_faces: Vec<u64>) -> EarlyOpenShell {
    EarlyOpenShell {
        name: String::new(),
        cfs_faces,
    }
}

/// Lift one `CLOSED_SHELL`.
pub(crate) fn lift_closed_shell(cfs_faces: Vec<u64>) -> EarlyClosedShell {
    EarlyClosedShell {
        name: String::new(),
        cfs_faces,
    }
}

/// Lift one `VERTEX_LOOP` (`loop_vertex` = child `VERTEX_POINT` output step id).
pub(crate) fn lift_vertex_loop(loop_vertex: u64) -> EarlyVertexLoop {
    EarlyVertexLoop {
        name: String::new(),
        loop_vertex,
    }
}

/// Lift one `ADVANCED_FACE` (bounds/surface = child output step ids; legacy
/// emits empty name).
pub(crate) fn lift_advanced_face(
    bounds: Vec<u64>,
    face_geometry: u64,
    orientation: Orientation,
) -> EarlyAdvancedFace {
    EarlyAdvancedFace {
        name: String::new(),
        bounds,
        face_geometry,
        same_sense: orientation_to_bool(orientation),
    }
}

/// Lift one `FACE_SURFACE`.
pub(crate) fn lift_face_surface(
    bounds: Vec<u64>,
    face_geometry: u64,
    orientation: Orientation,
) -> EarlyFaceSurface {
    EarlyFaceSurface {
        name: String::new(),
        bounds,
        face_geometry,
        same_sense: orientation_to_bool(orientation),
    }
}

/// Lift one `MANIFOLD_SOLID_BREP` (name preserved; outer = child shell step id).
pub(crate) fn lift_manifold_solid_brep(name: String, outer: u64) -> EarlyManifoldSolidBrep {
    EarlyManifoldSolidBrep { name, outer }
}
