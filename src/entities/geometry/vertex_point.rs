//! `VERTEX_POINT` handler — Pass 5-1 leaf vertex.
//!
//! Mirrors the legacy `ReaderContext::convert_vertex_point` and
//! `WriteBuffer::emit_vertex` one-to-one. The writer entry point keeps
//! its existing wrapper (`emit_vertex`) so callers in adjacent emit
//! functions don't have to import the handler.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::VertexId;
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Vertex;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct VertexPointHandler;

impl SimpleEntityHandler for VertexPointHandler {
    const NAME: &'static str = "VERTEX_POINT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5Vertex;
    type WriteInput = VertexId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "VERTEX_POINT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pt_ref = read_entity_ref(attrs, 1, entity_id, "vertex_geometry")?;

        let point = ctx.resolve_point(entity_id, pt_ref, "vertex_geometry")?;

        let vertex = Vertex { point };
        let id = ctx.geometry.vertices.push(vertex);
        ctx.vertex_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: VertexId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.vertex_ids.get(&id) {
            return Ok(n);
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
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "VERTEX_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(point),
                ],
            },
        });
        buf.vertex_ids.insert(id, n);
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static VERTEX_POINT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: VertexPointHandler::NAME,
    pass_level: VertexPointHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: VertexPointHandler::read,
    },
};
