//! `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` handler —
//! Pass 6-4g. Sister of `gbwsr`. Imports the shared body and registers
//! the `_SURFACE_` wrapper name so the writer round-trips the spelling
//! the source file used.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::WireframeReprKind;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::gbwsr::{
    WireframeRepresentationWriteInput, read_wireframe_representation_body,
    write_wireframe_representation,
};

pub(crate) struct GbssrHandler;

impl SimpleEntityHandler for GbssrHandler {
    const NAME: &'static str = "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6Gbsr;
    type WriteInput = WireframeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        read_wireframe_representation_body(ctx, entity_id, attrs, WireframeReprKind::Surface)
    }

    fn write(
        buf: &mut WriteBuffer,
        input: WireframeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        write_wireframe_representation(
            buf,
            "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
            input,
        )
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static GBSSR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: GbssrHandler::NAME,
    pass_level: GbssrHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: GbssrHandler::read,
    },
};
