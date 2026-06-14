//! `EDGE_LOOP` handler (2-layer path; intermediate `edge_loop_map`).
//!
//! The writer has no top-level `emit_edge_loop`; `emit_wire` delegates to this
//! handler's `write` when a `Wire` carries `edges`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::topology::oriented_edge::OrientedEdgeHandler;
use crate::ir::OrientedEdge;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct EdgeLoopHandler;

#[step_entity(name = "EDGE_LOOP")]
impl SimpleEntityHandler for EdgeLoopHandler {
    type WriteInput = Vec<OrientedEdge>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_edge_loop(entity_id, attrs)?;
        lower::lower_edge_loop(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, edges: Vec<OrientedEdge>) -> Result<u64, WriteError> {
        let mut oe_refs = Vec::with_capacity(edges.len());
        for oe in &edges {
            oe_refs.push(OrientedEdgeHandler::write(buf, *oe)?);
        }
        let early = lift::lift_edge_loop(oe_refs);
        Ok(serialize::serialize_edge_loop(buf, &early))
    }
}
