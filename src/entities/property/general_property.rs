//! `GENERAL_PROPERTY` handler — property (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::property::GeneralProperty;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GeneralPropertyHandler;

#[step_entity(name = "GENERAL_PROPERTY")]
impl SimpleEntityHandler for GeneralPropertyHandler {
    type WriteInput = GeneralProperty;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_general_property(entity_id, attrs)?;
        lower::lower_general_property(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gp: GeneralProperty) -> Result<u64, WriteError> {
        let early = lift::lift_general_property(gp);
        Ok(serialize::serialize_general_property(buf, &early))
    }
}
