//! `VERTEX_LOOP` handler (intermediate map).
//!
//! Degenerate single-vertex face boundary used by spheres and some
//! revolutions. Mirrors `ReaderContext::convert_vertex_loop` and
//! `WriteBuffer::emit_vertex_loop`.

// IR_PRESSURE: VERTEX_LOOP is read into `vertex_loop_map` keyed by entity
// id while EDGE_LOOP lives in `edge_loop_map`. FACE_BOUND probes both. A
// later IR refactor may unify the two into a single `Loop`
// arena variant so the lookup is symmetric.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::vertex_point::VertexPointHandler;
use crate::ir::VertexId;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
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
        check_count(attrs, 2, entity_id, "VERTEX_LOOP")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let vertex_ref = read_entity_ref(attrs, 1, entity_id, "loop_vertex")?;
        let vertex = ctx.resolve_vertex(entity_id, vertex_ref, "loop_vertex")?;
        ctx.vertex_loop_map.insert(entity_id, vertex);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vid: VertexId) -> Result<u64, WriteError> {
        let vertex_ref = VertexPointHandler::write(buf, vid)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "VERTEX_LOOP".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(vertex_ref),
                ],
            },
        });
        Ok(n)
    }
}
