//! `FACE_BOUND` handler — Pass 5-5 (intermediate map).
//!
//! Shares its read/write body with `FACE_OUTER_BOUND` via the
//! `read_face_bound_body` / `write_face_bound_body` helpers below; the
//! sister handler (`face_outer_bound.rs`) imports those helpers.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Orientation;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::topology::Wire;
use crate::parser::entity::Attribute;
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};

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
    let _name = read_string(attrs, 0, entity_id, "name")?;
    let loop_ref = read_entity_ref(attrs, 1, entity_id, "bound")?;
    let orientation = read_bool(attrs, 2, entity_id, "orientation")?;

    // `loop_ref` targets either an EDGE_LOOP (edges) or a VERTEX_LOOP
    // (degenerate single vertex). Try both maps before reporting a
    // missing reference.
    let wire = if ctx.edge_loop_map.contains_key(&loop_ref) {
        let edges = ctx.resolve_edge_loop(entity_id, loop_ref, "bound")?;
        Wire {
            edges,
            vertex: None,
            is_outer,
            orientation: bool_to_orientation(orientation),
        }
    } else if let Some(&vertex) = ctx.vertex_loop_map.get(&loop_ref) {
        Wire {
            edges: Vec::new(),
            vertex: Some(vertex),
            is_outer,
            orientation: bool_to_orientation(orientation),
        }
    } else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: loop_ref,
            field_name: "bound",
        });
    };
    let id = ctx.topology.wires.push(wire);
    ctx.face_bound_map.insert(entity_id, id);
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

impl SimpleEntityHandler for FaceBoundHandler {
    const NAME: &'static str = "FACE_BOUND";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5FaceBound;
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static FACE_BOUND_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: FaceBoundHandler::NAME,
    pass_level: FaceBoundHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: FaceBoundHandler::read,
    },
};
