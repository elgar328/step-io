//! Topology-domain `lower` fns (W-RECURSIVE spread). See the [module docs](super)
//! for the lowering contract. Refs resolve through the shared reader resolvers;
//! a dangling child surfaces as `MissingReference`.

use crate::early::model::{EarlyFaceBound, EarlyFaceOuterBound};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Wire, WireData};
use crate::reader::{ReaderContext, bool_to_orientation};

/// Shared `FACE_BOUND` / `FACE_OUTER_BOUND` lowering: the `bound` loop is
/// either an edge loop (resolved to its edges) or a vertex loop; an
/// unresolved loop is a `MissingReference`.
fn lower_face_bound_common(
    ctx: &mut ReaderContext,
    entity_id: u64,
    bound: u64,
    orientation: bool,
    is_outer: bool,
) -> Result<(), ConvertError> {
    let (edges, vertex) = if ctx.edge_loop_map.contains_key(&bound) {
        (ctx.resolve_edge_loop(entity_id, bound, "bound")?, None)
    } else if let Some(&vertex) = ctx.vertex_loop_map.get(&bound) {
        (Vec::new(), Some(vertex))
    } else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: bound,
            field_name: "bound",
        });
    };
    let data = WireData {
        edges,
        vertex,
        orientation: bool_to_orientation(orientation),
    };
    let wire = if is_outer {
        Wire::FaceOuterBound(data)
    } else {
        Wire::FaceBound(data)
    };
    let id = ctx.topology.wires.push(wire);
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `FACE_BOUND`.
pub(crate) fn lower_face_bound(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyFaceBound,
) -> Result<(), ConvertError> {
    lower_face_bound_common(ctx, entity_id, early.bound, early.orientation, false)
}

/// Lower one `FACE_OUTER_BOUND`.
pub(crate) fn lower_face_outer_bound(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyFaceOuterBound,
) -> Result<(), ConvertError> {
    lower_face_bound_common(ctx, entity_id, early.bound, early.orientation, true)
}
