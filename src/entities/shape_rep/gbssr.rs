//! `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` handler —
//! Pass 6-4g. Sister of `gbwsr`. Imports the shared body and registers
//! the `_SURFACE_` wrapper name so the writer round-trips the spelling
//! the source file used.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::WireframeReprKind;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

use super::gbwsr::{
    WireframeRepresentationWriteInput, read_wireframe_representation_body,
    write_wireframe_representation,
};

pub(crate) struct GbssrHandler;

#[step_entity(name = "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION", pass = Pass6Gbsr)]
impl SimpleEntityHandler for GbssrHandler {
    type WriteInput = WireframeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
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
