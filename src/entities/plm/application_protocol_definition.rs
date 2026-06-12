//! `APPLICATION_PROTOCOL_DEFINITION` handler — plm (2-layer path).
//! The writer's `emit_application_context` routes every APD line (arena
//! and synthesised fallbacks) through this handler's `write`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Pre-resolved APD attrs (`application` = the AC's output step id).
pub(crate) struct ApplicationProtocolDefinitionWriteInput {
    pub(crate) status: String,
    pub(crate) application_interpreted_model_schema_name: String,
    pub(crate) application_protocol_year: i64,
    pub(crate) application: u64,
}

pub(crate) struct ApplicationProtocolDefinitionHandler;

#[step_entity(name = "APPLICATION_PROTOCOL_DEFINITION")]
impl SimpleEntityHandler for ApplicationProtocolDefinitionHandler {
    type WriteInput = ApplicationProtocolDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_application_protocol_definition(entity_id, attrs)?;
        lower::lower_application_protocol_definition(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ApplicationProtocolDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_application_protocol_definition(
            input.status,
            input.application_interpreted_model_schema_name,
            input.application_protocol_year,
            input.application,
        );
        Ok(serialize::serialize_application_protocol_definition(
            buf, &early,
        ))
    }
}
