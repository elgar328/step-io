//! `GEOMETRIC_SET` handler — Pass 6-4f. Sister of `geometric_curve_set`;
//! shares the same EXPRESS shape but allows loose points (and rarely
//! surfaces) alongside curves. Imports the shared body from the curve-set
//! module and registers the alternate entity name.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::geometric_curve_set::{
    CurveSetWriteInput, read_geometric_curve_set_body, write_geometric_curve_set,
};

pub(crate) struct GeometricSetHandler;

impl SimpleEntityHandler for GeometricSetHandler {
    const NAME: &'static str = "GEOMETRIC_SET";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6CurveSet;
    type WriteInput = CurveSetWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        read_geometric_curve_set_body(ctx, entity_id, attrs, "GEOMETRIC_SET")
    }

    fn write(buf: &mut WriteBuffer, input: CurveSetWriteInput) -> Result<u64, WriteError> {
        write_geometric_curve_set(buf, "GEOMETRIC_SET", input)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static GS_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: GeometricSetHandler::NAME,
    pass_level: GeometricSetHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: GeometricSetHandler::read,
    },
};
