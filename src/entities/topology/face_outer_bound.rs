//! `FACE_OUTER_BOUND` handler — Pass 5-5 (intermediate map).
//!
//! Sister handler of `FACE_BOUND`. Both share the read/write body in
//! `face_bound.rs`; only the `is_outer` flag flips.

use crate::entities::topology::face_bound::{read_face_bound_body, write_face_bound_body};
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Orientation;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct FaceOuterBoundHandler;

impl SimpleEntityHandler for FaceOuterBoundHandler {
    const NAME: &'static str = "FACE_OUTER_BOUND";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5FaceBound;
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_face_bound_body(ctx, entity_id, attrs, true)
    }

    fn write(
        buf: &mut WriteBuffer,
        (inner_loop_ref, orientation): (u64, Orientation),
    ) -> Result<u64, WriteError> {
        Ok(write_face_bound_body(
            buf,
            inner_loop_ref,
            orientation,
            true,
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static FACE_OUTER_BOUND_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: FaceOuterBoundHandler::NAME,
    pass_level: FaceOuterBoundHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: FaceOuterBoundHandler::read,
    },
};
