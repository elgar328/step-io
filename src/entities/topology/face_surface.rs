//! `FACE_SURFACE` handler.
//!
//! Sister handler of `ADVANCED_FACE`. Both share the read/write body in
//! `advanced_face.rs`; the read side picks the `Face` variant, and the
//! write body keys off the IR-stored variant so a single helper covers
//! both entity names.

use crate::entities::SimpleEntityHandler;
use crate::entities::topology::advanced_face::{read_face_body, write_face_body};
use crate::ir::FaceId;
use crate::ir::error::ConvertError;
use crate::ir::topology::Face;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct FaceSurfaceHandler;

#[step_entity(name = "FACE_SURFACE")]
impl SimpleEntityHandler for FaceSurfaceHandler {
    type WriteInput = FaceId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_face_body(ctx, entity_id, attrs, "FACE_SURFACE", Face::FaceSurface)
    }

    fn write(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
        write_face_body(buf, id)
    }
}
