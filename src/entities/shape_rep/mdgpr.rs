//! `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` handler.
//! Top-level visualization wrapper holding a list of
//! `STYLED_ITEM`s plus an optional unit context.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::Mdgpr;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct MdgprHandler;

#[step_entity(name = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION")]
impl SimpleEntityHandler for MdgprHandler {
    type WriteInput = Mdgpr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early =
            bind::bind_mechanical_design_geometric_presentation_representation(entity_id, attrs)?;
        lower::lower_mechanical_design_geometric_presentation_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mdgpr: Mdgpr) -> Result<u64, WriteError> {
        Ok(
            serialize::serialize_mechanical_design_geometric_presentation_representation(
                buf,
                &lift::lift_mechanical_design_geometric_presentation_representation(buf, mdgpr),
            ),
        )
    }
}
