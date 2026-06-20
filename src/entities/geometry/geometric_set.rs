//! `GEOMETRIC_SET` handler. Sister of `geometric_curve_set`;
//! shares the same EXPRESS shape but allows loose points (and rarely
//! surfaces) alongside curves. Imports the shared body from the curve-set
//! module and registers the alternate entity name.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

use super::geometric_curve_set::{CurveSetWriteInput, emit_curve_set_elements};

pub(crate) struct GeometricSetHandler;

#[step_entity(name = "GEOMETRIC_SET")]
impl SimpleEntityHandler for GeometricSetHandler {
    type WriteInput = CurveSetWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_geometric_set(entity_id, attrs)?;
        lower::lower_geometric_set(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: CurveSetWriteInput) -> Result<u64, WriteError> {
        let elements = emit_curve_set_elements(buf, input)?;
        let early = lift::lift_geometric_set(elements);
        Ok(serialize::serialize_geometric_set(buf, &early))
    }
}
