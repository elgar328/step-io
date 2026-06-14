//! Topology-domain `lower` fns (W-RECURSIVE spread). See the [module docs](super)
//! for the lowering contract. Refs resolve through the shared reader resolvers;
//! a dangling child surfaces as `MissingReference`.

use crate::early::model::{
    EarlyAdvancedFace, EarlyClosedShell, EarlyEdgeCurve, EarlyEdgeLoop, EarlyFaceBound,
    EarlyFaceOuterBound, EarlyFaceSurface, EarlyManifoldSolidBrep, EarlyOpenShell, EarlyVertexLoop,
};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Edge, Face, FaceData, Orientation, Shell, Solid, Wire, WireData};
use crate::reader::{ReaderContext, bool_to_orientation};

/// Lower one `EDGE_LOOP` — resolve each `ORIENTED_EDGE` member (via the reader's
/// `edge_loop_map`/`oriented_edge` resolvers). Classify each: resolved, a
/// dangling-reference cascade drop (`nonstandard_dropped_refs`), or a genuine
/// miss. A genuine miss is a defect. If a member is a cascade drop (and none is
/// a genuine miss) the whole loop drops; its *resolved* members emit only via
/// this loop, so they orphan — record those, then return a `MissingReference`
/// to the cascade member so the dispatcher reclassifies the loop and seeds it.
pub(crate) fn lower_edge_loop(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyEdgeLoop,
) -> Result<(), ConvertError> {
    let mut edges = Vec::with_capacity(early.edge_list.len());
    let mut resolved_members = 0usize;
    let mut cascade_member: Option<u64> = None;
    for &r in &early.edge_list {
        match ctx.resolve_oriented_edge(entity_id, r, "edge_list") {
            Ok(oe) => {
                edges.push(oe);
                resolved_members += 1;
            }
            Err(e) => {
                if ctx.nonstandard_dropped_refs.contains(&r) {
                    cascade_member = Some(r);
                } else {
                    return Err(e); // genuine missing ref → defect, no orphan record
                }
            }
        }
    }
    if let Some(bad) = cascade_member {
        for _ in 0..resolved_members {
            ctx.ns_record(
                crate::reader::NsCase::DanglingReferenceOrphan,
                "ORIENTED_EDGE".to_string(),
                "dropped (orphaned by dangling-dropped loop)",
            );
        }
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: bad,
            field_name: "edge_list",
        });
    }
    ctx.edge_loop_map.insert(entity_id, edges);
    Ok(())
}

/// Lower one `EDGE_CURVE`. The handler pre-resolves the refs (so a dangling
/// `edge_geometry` surfaces as a `MissingReference` before the strict bind reads
/// `same_sense`); this re-resolves them (side-effect-free id-cache lookups) and
/// builds the `Edge`. If `edge_geometry` was a `SURFACE_CURVE` / `SEAM_CURVE`,
/// its wrapper (captured by that handler keyed on the wrapper id) rides along.
pub(crate) fn lower_edge_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyEdgeCurve,
) -> Result<(), ConvertError> {
    let start = ctx.resolve_vertex(entity_id, early.edge_start, "edge_start")?;
    let end = ctx.resolve_vertex(entity_id, early.edge_end, "edge_end")?;
    let curve = ctx.resolve_curve(entity_id, early.edge_geometry, "edge_geometry")?;
    let surface_curve = ctx.surface_curve_map.get(&early.edge_geometry).cloned();
    let edge = Edge {
        curve,
        vertices: (start, end),
        trim: (0.0, 0.0),
        orientation: bool_to_orientation(early.same_sense),
        surface_curve,
    };
    let id = ctx.topology.edges.push(edge);
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

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

/// Shared `ADVANCED_FACE` / `FACE_SURFACE` lowering: resolve the bound wires
/// + the surface, build a `FaceData` under the given variant.
fn lower_face_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    bound_refs: &[u64],
    face_geometry: u64,
    same_sense: bool,
    variant: fn(FaceData) -> Face,
) -> Result<(), ConvertError> {
    let mut bounds = Vec::with_capacity(bound_refs.len());
    for &r in bound_refs {
        bounds.push(ctx.resolve_face_bound(entity_id, r, "bounds")?);
    }
    let surface = ctx.resolve_surface(entity_id, face_geometry, "face_geometry")?;
    let id = ctx.topology.faces.push(variant(FaceData {
        surface,
        bounds,
        orientation: bool_to_orientation(same_sense),
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `ADVANCED_FACE`.
pub(crate) fn lower_advanced_face(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAdvancedFace,
) -> Result<(), ConvertError> {
    lower_face_body(
        ctx,
        entity_id,
        &early.bounds,
        early.face_geometry,
        early.same_sense,
        Face::AdvancedFace,
    )
}

/// Lower one `FACE_SURFACE`.
pub(crate) fn lower_face_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyFaceSurface,
) -> Result<(), ConvertError> {
    lower_face_body(
        ctx,
        entity_id,
        &early.bounds,
        early.face_geometry,
        early.same_sense,
        Face::FaceSurface,
    )
}

/// Lower one `MANIFOLD_SOLID_BREP` (outer shell; empty name collapses to
/// `None` as the legacy read did).
pub(crate) fn lower_manifold_solid_brep(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyManifoldSolidBrep,
) -> Result<(), ConvertError> {
    let outer = ctx.resolve_shell(entity_id, early.outer, "outer")?;
    let name = if early.name.is_empty() {
        None
    } else {
        Some(early.name.clone())
    };
    let id = ctx
        .topology
        .solids
        .push(Solid::ManifoldSolidBrep { outer, name });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}
