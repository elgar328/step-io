//! `APPLICATION_CONTEXT` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::ApplicationContext;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApplicationContextHandler;

#[step_entity(name = "APPLICATION_CONTEXT")]
impl SimpleEntityHandler for ApplicationContextHandler {
    type WriteInput = ApplicationContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_application_context(entity_id, attrs)?;
        lower::lower_application_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: ApplicationContext) -> Result<u64, WriteError> {
        let early = lift::lift_application_context(v);
        Ok(serialize::serialize_application_context(buf, &early))
    }
}
