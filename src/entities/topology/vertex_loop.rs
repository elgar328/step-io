//! `VERTEX_LOOP` handler — loop of a single vertex (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::vertex_point::VertexPointHandler;
use crate::ir::VertexId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct VertexLoopHandler;

#[step_entity(name = "VERTEX_LOOP")]
impl SimpleEntityHandler for VertexLoopHandler {
    type WriteInput = VertexId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_vertex_loop(entity_id, attrs)?;
        lower::lower_vertex_loop(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, vid: VertexId) -> Result<u64, WriteError> {
        let vertex_ref = VertexPointHandler::write(buf, vid)?;
        let early = lift::lift_vertex_loop(vertex_ref);
        Ok(serialize::serialize_vertex_loop(buf, &early))
    }
}
