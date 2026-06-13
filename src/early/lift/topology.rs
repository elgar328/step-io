//! Topology-domain `lift` fns (W-RECURSIVE spread). See the [module docs](super)
//! for the lifting contract. Child refs are pre-resolved to output step ids by
//! the handler's emit recursion; `name` is the legacy empty string.

use crate::early::model::{EarlyFaceBound, EarlyFaceOuterBound};
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
