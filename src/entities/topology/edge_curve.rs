//! `EDGE_CURVE` handler (2-layer path).
//!
//! `read` pre-resolves the refs before the strict `bind` (see the resolve-first
//! note below), then `lower` builds the `Edge`. The writer keeps the cache
//! check + `set_step_id` (edges are shared, emitted once); `emit_edge`
//! (called by `emit_oriented_edge` etc.) delegates here.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::EdgeId;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::topology::Edge;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct EdgeCurveHandler;

#[step_entity(name = "EDGE_CURVE")]
impl SimpleEntityHandler for EdgeCurveHandler {
    type WriteInput = EdgeId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // Resolve-first: a few ABC-dataset exporters emit a malformed edge with
        // `edge_geometry = #0` (undefined) AND an empty `same_sense` slot.
        // Resolving the refs *before* the strict `bind` reads `same_sense` makes
        // the dangling `edge_geometry` surface as a `MissingReference` (→ the
        // dispatcher drops it as a dangling-reference normalization, NORM),
        // rather than the empty `same_sense` raising an attribute defect (LOSS)
        // that masks the real cause. The resolves are side-effect-free id-cache
        // lookups, so re-resolving in `lower` is harmless; valid edges are
        // unaffected.
        check_count(attrs, 5, entity_id, "EDGE_CURVE")?;
        let start_ref = read_entity_ref(attrs, 1, entity_id, "edge_start")?;
        let end_ref = read_entity_ref(attrs, 2, entity_id, "edge_end")?;
        let curve_ref = read_entity_ref(attrs, 3, entity_id, "edge_geometry")?;
        ctx.resolve_vertex(entity_id, start_ref, "edge_start")?;
        ctx.resolve_vertex(entity_id, end_ref, "edge_end")?;
        ctx.resolve_curve(entity_id, curve_ref, "edge_geometry")?;

        let early = bind::bind_edge_curve(entity_id, attrs)?;
        lower::lower_edge_curve(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: EdgeId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let e: Edge = buf
            .model
            .topology
            .edges
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("EdgeId({})", id.0),
            })?;
        let start = buf.emit_vertex(e.vertices.0)?;
        let end = buf.emit_vertex(e.vertices.1)?;
        // `edge_geometry` is a plain 3D curve or a surface_curve-family node.
        let curve_ref = match e.edge_geometry {
            crate::ir::topology::EdgeGeometry::Curve3d(c) => buf.emit_curve(c)?,
            crate::ir::topology::EdgeGeometry::SurfaceCurve(scid) => {
                buf.emit_surface_curve_node(scid)?
            }
        };
        let early = lift::lift_edge_curve(start, end, curve_ref, e.orientation);
        let n = serialize::serialize_edge_curve(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
