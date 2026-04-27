//! `EDGE_LOOP` handler — Pass 5-4 (intermediate map).
//!
//! Mirrors `ReaderContext::convert_edge_loop`. The writer side does not
//! own a top-level `emit_edge_loop`; `emit_wire` inlines the `EDGE_LOOP`
//! body when a `Wire` carries `edges`. The handler's `write` lifts that
//! inline emission into the registry so callers can use it directly.

use crate::entities::topology::oriented_edge::OrientedEdgeHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::OrientedEdge;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct EdgeLoopHandler;

impl SimpleEntityHandler for EdgeLoopHandler {
    const NAME: &'static str = "EDGE_LOOP";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5EdgeLoop;
    type WriteInput = Vec<OrientedEdge>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "EDGE_LOOP")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let edge_refs = read_entity_ref_list(attrs, 1, entity_id, "edge_list")?;

        let mut edges = Vec::with_capacity(edge_refs.len());
        for &r in &edge_refs {
            let oe = ctx.resolve_oriented_edge(entity_id, r, "edge_list")?;
            edges.push(oe);
        }
        ctx.edge_loop_map.insert(entity_id, edges);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, edges: Vec<OrientedEdge>) -> Result<u64, WriteError> {
        let mut oe_refs = Vec::with_capacity(edges.len());
        for oe in &edges {
            oe_refs.push(OrientedEdgeHandler::write(buf, *oe)?);
        }
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "EDGE_LOOP".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(oe_refs.into_iter().map(Attribute::EntityRef).collect()),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static EDGE_LOOP_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: EdgeLoopHandler::NAME,
    pass_level: EdgeLoopHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: EdgeLoopHandler::read,
    },
};
