//! `ORIENTED_EDGE` handler (intermediate map).
//!
//! Mirrors `ReaderContext::convert_oriented_edge` and
//! `WriteBuffer::emit_oriented_edge`. `ORIENTED_EDGE` has no IR arena; the
//! reader stores `(EdgeId, Orientation)` in `oriented_edge_map` keyed by
//! the entity id, and the writer reconstructs an `OrientedEdge` value
//! from `Wire::edges` at emit time. Same shape as the VECTOR handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::OrientedEdge;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};
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
        check_count(attrs, 5, entity_id, "ORIENTED_EDGE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        // attrs[1] and [2] are derived (*) — skip them.
        let edge_ref = read_entity_ref(attrs, 3, entity_id, "edge_element")?;
        let orientation = read_bool(attrs, 4, entity_id, "orientation")?;

        let edge = ctx.resolve_edge(entity_id, edge_ref, "edge_element")?;

        let oe = OrientedEdge {
            edge,
            orientation: bool_to_orientation(orientation),
        };
        ctx.oriented_edge_map.insert(entity_id, oe);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, oe: OrientedEdge) -> Result<u64, WriteError> {
        let edge_ref = buf.emit_edge(oe.edge)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ORIENTED_EDGE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::Derived,
                    Attribute::Derived,
                    Attribute::EntityRef(edge_ref),
                    orientation_bool(oe.orientation),
                ],
            },
        });
        Ok(n)
    }
}
