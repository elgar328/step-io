//! `VERTEX_POINT` handler — leaf vertex wrapping a point (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::VertexId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct VertexPointHandler;

#[step_entity(name = "VERTEX_POINT")]
impl SimpleEntityHandler for VertexPointHandler {
    type WriteInput = VertexId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_vertex_point(entity_id, attrs)?;
        lower::lower_vertex_point(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: VertexId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let v = buf
            .model
            .geometry
            .vertices
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("VertexId({})", id.0),
            })?;
        let point = buf.emit_point(v.point)?;
        let early = lift::lift_vertex_point(point);
        let n = serialize::serialize_vertex_point(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
