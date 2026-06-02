//! `FACE_OUTER_BOUND` handler — Pass 5-5 (intermediate map).
//!
//! Sister handler of `FACE_BOUND`. Both share the read/write body in
//! `face_bound.rs`; only the `is_outer` flag flips.

use crate::entities::SimpleEntityHandler;
use crate::entities::topology::face_bound::{read_face_bound_body, write_face_bound_body};
use crate::ir::Orientation;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct FaceOuterBoundHandler;

#[step_entity(name = "FACE_OUTER_BOUND")]
impl SimpleEntityHandler for FaceOuterBoundHandler {
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
