//! `GEOMETRIC_SET` handler. Sister of `geometric_curve_set`;
//! shares the same EXPRESS shape but allows loose points (and rarely
//! surfaces) alongside curves. Imports the shared body from the curve-set
//! module and registers the alternate entity name.

use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

use super::geometric_curve_set::{
    CurveSetWriteInput, read_geometric_curve_set_body, write_geometric_curve_set,
};

pub(crate) struct GeometricSetHandler;

#[step_entity(name = "GEOMETRIC_SET")]
impl SimpleEntityHandler for GeometricSetHandler {
    type WriteInput = CurveSetWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_geometric_curve_set_body(ctx, entity_id, attrs, "GEOMETRIC_SET")
    }

    fn write(buf: &mut WriteBuffer, input: CurveSetWriteInput) -> Result<u64, WriteError> {
        write_geometric_curve_set(buf, "GEOMETRIC_SET", input)
    }
}
