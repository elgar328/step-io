//! `EDGE_CURVE` handler.
//!
//! Mirrors the legacy `ReaderContext::convert_edge_curve` and
//! `WriteBuffer::emit_edge` one-to-one. The writer keeps `emit_edge` as a
//! pub(crate) wrapper (called by `emit_oriented_edge` etc.).

use crate::entities::SimpleEntityHandler;
use crate::ir::EdgeId;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::topology::Edge;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct EdgeCurveHandler;

#[step_entity(name = "EDGE_CURVE")]
impl SimpleEntityHandler for EdgeCurveHandler {
    type WriteInput = EdgeId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "EDGE_CURVE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let start_ref = read_entity_ref(attrs, 1, entity_id, "edge_start")?;
        let end_ref = read_entity_ref(attrs, 2, entity_id, "edge_end")?;
        let curve_ref = read_entity_ref(attrs, 3, entity_id, "edge_geometry")?;
        // Resolve the refs before reading `same_sense`. A few ABC-dataset
        // exporters emit a malformed edge with `edge_geometry = #0` (undefined
        // in the file) AND an empty `same_sense` slot. Resolving first makes the
        // dangling `edge_geometry` surface as a `MissingReference`, so the
        // dispatcher drops the EDGE_CURVE as a dangling-reference normalization
        // (NS-dangling-reference-drop) instead of the empty `same_sense` raising
        // an attribute defect that masks the real cause. Valid edges are
        // unaffected (both reads are side-effect-free; only one error survives).
        let start = ctx.resolve_vertex(entity_id, start_ref, "edge_start")?;
        let end = ctx.resolve_vertex(entity_id, end_ref, "edge_end")?;
        let curve = ctx.resolve_curve(entity_id, curve_ref, "edge_geometry")?;
        let same_sense = read_bool(attrs, 4, entity_id, "same_sense")?;
        // If edge_geometry referenced a SURFACE_CURVE / SEAM_CURVE, its
        // wrapper was captured by the `SURFACE_CURVE` / `SEAM_CURVE` handler keyed by that wrapper id. Direct
        // 3D-curve refs return None.
        let surface_curve = ctx.surface_curve_map.get(&curve_ref).cloned();

        let edge = Edge {
            curve,
            vertices: (start, end),
            trim: (0.0, 0.0),
            orientation: bool_to_orientation(same_sense),
            surface_curve,
        };
        let id = ctx.topology.edges.push(edge);
        ctx.edge_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: EdgeId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.edge_ids.get(&id) {
            return Ok(n);
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
        let curve_3d = buf.emit_curve(e.curve)?;
        // Wrap in SURFACE_CURVE / SEAM_CURVE if the edge carried one;
        // otherwise emit EDGE_CURVE against the 3D curve directly.
        let curve_ref = match &e.surface_curve {
            Some(wrapper) => buf.emit_surface_curve_wrapper(curve_3d, wrapper)?,
            None => curve_3d,
        };
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "EDGE_CURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(start),
                    Attribute::EntityRef(end),
                    Attribute::EntityRef(curve_ref),
                    orientation_bool(e.orientation),
                ],
            },
        });
        buf.edge_ids.insert(id, n);
        Ok(n)
    }
}
