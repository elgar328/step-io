//! `ORIENTED_EDGE` handler (2-layer path; intermediate `oriented_edge_map`).
//!
//! No IR arena: `read` stores `OrientedEdge{edge, orientation}` in
//! `oriented_edge_map`; the writer reconstructs it from `Wire::edges` at emit
//! time. `edge_start`/`edge_end` are EXPRESS Derived (`*`) — gen-early omits
//! them from L1 and re-emits `*` (see mapping `[derived]`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::OrientedEdge;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OrientedEdgeHandler;

#[step_entity(name = "ORIENTED_EDGE")]
impl SimpleEntityHandler for OrientedEdgeHandler {
    type WriteInput = OrientedEdge;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_oriented_edge(entity_id, attrs)?;
        lower::lower_oriented_edge(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, oe: OrientedEdge) -> Result<u64, WriteError> {
        let edge_ref = buf.emit_edge(oe.edge)?;
        let early = lift::lift_oriented_edge(edge_ref, oe.orientation);
        Ok(serialize::serialize_oriented_edge(buf, &early))
    }
}
