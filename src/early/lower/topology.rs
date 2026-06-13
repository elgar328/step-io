//! Topology-domain `lower` fns (W-RECURSIVE spread). See the [module docs](super)
//! for the lowering contract. Refs resolve through the shared reader resolvers;
//! a dangling child surfaces as `MissingReference`.

use crate::early::model::{
    EarlyClosedShell, EarlyFaceBound, EarlyFaceOuterBound, EarlyOpenShell, EarlyVertexLoop,
};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Orientation, Shell, Wire, WireData};
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

/// Shared `OPEN_SHELL` / `CLOSED_SHELL` lowering: resolve the face list
/// (a dangling face is a `MissingReference`) and push a `Shell`.
fn lower_shell_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    faces_refs: &[u64],
    is_open: bool,
) -> Result<(), ConvertError> {
    let mut faces = Vec::with_capacity(faces_refs.len());
    for &r in faces_refs {
        faces.push(ctx.resolve_face(entity_id, r, "cfs_faces")?);
    }
    let id = ctx.topology.shells.push(Shell {
        faces,
        orientation: Orientation::Forward,
        is_open,
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `OPEN_SHELL`.
pub(crate) fn lower_open_shell(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyOpenShell,
) -> Result<(), ConvertError> {
    lower_shell_body(ctx, entity_id, &early.cfs_faces, true)
}

/// Lower one `CLOSED_SHELL`.
pub(crate) fn lower_closed_shell(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyClosedShell,
) -> Result<(), ConvertError> {
    lower_shell_body(ctx, entity_id, &early.cfs_faces, false)
}

/// Lower one `VERTEX_LOOP` (single vertex; registered in `vertex_loop_map`).
pub(crate) fn lower_vertex_loop(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVertexLoop,
) -> Result<(), ConvertError> {
    let vertex = ctx.resolve_vertex(entity_id, early.loop_vertex, "loop_vertex")?;
    ctx.vertex_loop_map.insert(entity_id, vertex);
    Ok(())
}
