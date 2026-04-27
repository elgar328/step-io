//! `FACE_SURFACE` handler — Pass 5-6.
//!
//! Sister handler of `ADVANCED_FACE`. Both share the read/write body in
//! `advanced_face.rs`; the read side flips the `FaceKind` tag, and the
//! write body keys off the IR-stored kind so a single helper covers
//! both entity names.

use crate::entities::topology::advanced_face::{read_face_body, write_face_body};
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::FaceId;
use crate::ir::error::ConvertError;
use crate::ir::topology::FaceKind;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct FaceSurfaceHandler;

impl SimpleEntityHandler for FaceSurfaceHandler {
    const NAME: &'static str = "FACE_SURFACE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5Face;
    type WriteInput = FaceId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        read_face_body(ctx, entity_id, attrs, FaceKind::General)
    }

    fn write(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
        write_face_body(buf, id)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static FACE_SURFACE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: FaceSurfaceHandler::NAME,
    pass_level: FaceSurfaceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: FaceSurfaceHandler::read,
    },
};
