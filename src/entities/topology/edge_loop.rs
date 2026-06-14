//! `EDGE_LOOP` handler (intermediate map).
//!
//! Mirrors `ReaderContext::convert_edge_loop`. The writer side does not
//! own a top-level `emit_edge_loop`; `emit_wire` inlines the `EDGE_LOOP`
//! body when a `Wire` carries `edges`. The handler's `write` lifts that
//! inline emission into the registry so callers can use it directly.

use crate::entities::SimpleEntityHandler;
use crate::entities::topology::oriented_edge::OrientedEdgeHandler;
use crate::ir::OrientedEdge;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
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
        check_count(attrs, 2, entity_id, "EDGE_LOOP")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let edge_refs = read_entity_ref_list(attrs, 1, entity_id, "edge_list")?;

        // Resolve every member, classifying each: resolved (good), a
        // dangling-reference cascade drop (in `nonstandard_dropped_refs`), or a
        // genuine missing ref. A genuine miss stays a defect. If a member is a
        // cascade drop (and no genuine miss), the whole loop drops — but its
        // *resolved* members are good edges that emit only via this loop, so
        // they orphan. Record those orphans, then return a MissingReference to
        // the cascade member so the dispatcher reclassifies the loop and seeds
        // it. NsCase::DanglingReferenceOrphan
        let mut edges = Vec::with_capacity(edge_refs.len());
        let mut resolved_members = 0usize;
        let mut cascade_member: Option<u64> = None;
        for &r in &edge_refs {
            match ctx.resolve_oriented_edge(entity_id, r, "edge_list") {
                Ok(oe) => {
                    edges.push(oe);
                    resolved_members += 1;
                }
                Err(e) => {
                    if ctx.nonstandard_dropped_refs.contains(&r) {
                        cascade_member = Some(r);
                    } else {
                        return Err(e); // genuine missing ref → defect, no orphan record
                    }
                }
            }
        }
        if let Some(bad) = cascade_member {
            for _ in 0..resolved_members {
                ctx.ns_record(
                    crate::reader::NsCase::DanglingReferenceOrphan,
                    "ORIENTED_EDGE".to_string(),
                    "dropped (orphaned by dangling-dropped loop)",
                );
            }
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: bad,
                field_name: "edge_list",
            });
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
