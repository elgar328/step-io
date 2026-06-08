//! `FACE_BOUND` handler (intermediate map).
//!
//! Shares its read/write body with `FACE_OUTER_BOUND` via the
//! `read_face_bound_body` / `write_face_bound_body` helpers below; the
//! sister handler (`face_outer_bound.rs`) imports those helpers.

use crate::entities::SimpleEntityHandler;
use crate::ir::Orientation;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Wire, WireData};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

/// Reader body shared by `FACE_BOUND` and `FACE_OUTER_BOUND`. The only
/// difference is the `is_outer` flag stored on the resulting `Wire`.
pub(super) fn read_face_bound_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    is_outer: bool,
) -> Result<(), ConvertError> {
    let entity_name = if is_outer {
        "FACE_OUTER_BOUND"
    } else {
        "FACE_BOUND"
    };
    check_count(attrs, 3, entity_id, entity_name)?;
    let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
    let loop_ref = read_entity_ref(attrs, 1, entity_id, "bound")?;
    let orientation = read_bool(attrs, 2, entity_id, "orientation")?;

    // `loop_ref` targets either an EDGE_LOOP (edges) or a VERTEX_LOOP
    // (degenerate single vertex). Try both maps before reporting a
    // missing reference.
    let (edges, vertex) = if ctx.edge_loop_map.contains_key(&loop_ref) {
        (ctx.resolve_edge_loop(entity_id, loop_ref, "bound")?, None)
    } else if let Some(&vertex) = ctx.vertex_loop_map.get(&loop_ref) {
        (Vec::new(), Some(vertex))
    } else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: loop_ref,
            field_name: "bound",
        });
    };
    let data = WireData {
        edges,
        vertex,
        orientation: bool_to_orientation(orientation),
    };
    let wire = if is_outer {
        Wire::FaceOuterBound(data)
    } else {
        Wire::FaceBound(data)
    };
    let id = ctx.topology.wires.push(wire);
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Writer body shared by `FACE_BOUND` and `FACE_OUTER_BOUND`. Caller
/// supplies the already-emitted inner-loop reference (`EDGE_LOOP` or
/// `VERTEX_LOOP`) and the wire's orientation.
pub(super) fn write_face_bound_body(
    buf: &mut WriteBuffer,
    inner_loop_ref: u64,
    orientation: Orientation,
    is_outer: bool,
) -> u64 {
    let name = if is_outer {
        "FACE_OUTER_BOUND"
    } else {
        "FACE_BOUND"
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: name.into(),
            attrs: vec![
                Attribute::String(String::new()),
                Attribute::EntityRef(inner_loop_ref),
                orientation_bool(orientation),
            ],
        },
    });
    n
}

pub(crate) struct FaceBoundHandler;

#[step_entity(name = "FACE_BOUND")]
impl SimpleEntityHandler for FaceBoundHandler {
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_face_bound_body(ctx, entity_id, attrs, false)
    }

    fn write(
        buf: &mut WriteBuffer,
        (inner_loop_ref, orientation): (u64, Orientation),
    ) -> Result<u64, WriteError> {
        Ok(write_face_bound_body(
            buf,
            inner_loop_ref,
            orientation,
            false,
        ))
    }
}
