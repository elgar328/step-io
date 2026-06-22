//! `FACE_SURFACE` handler (2-layer path; shares the face writer with
//! `ADVANCED_FACE`).

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::entities::topology::advanced_face::write_face_body;
use crate::ir::FaceId;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_face_surface(entity_id, attrs)?;
        lower::lower_face_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
        write_face_body(buf, id)
    }
}
