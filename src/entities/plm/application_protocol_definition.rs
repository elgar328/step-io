//! `APPLICATION_PROTOCOL_DEFINITION` handler plm. STEP
//! positional shape `(status, application_interpreted_model_schema_name,
//! application_protocol_year, application)` per `AP214e3` schema.
//! `application` is a ref to `APPLICATION_CONTEXT` populated by the
//! `APPLICATION_CONTEXT` handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_integer, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApplicationProtocolDefinition, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApplicationProtocolDefinitionHandler;

#[step_entity(name = "APPLICATION_PROTOCOL_DEFINITION")]
impl SimpleEntityHandler for ApplicationProtocolDefinitionHandler {
    type WriteInput = ApplicationProtocolDefinition;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "APPLICATION_PROTOCOL_DEFINITION")?;
        let status = read_string_or_unset(attrs, 0, entity_id, "status")?.to_owned();
        let name = read_string_or_unset(
            attrs,
            1,
            entity_id,
            "application_interpreted_model_schema_name",
        )?
        .to_owned();
        let year = read_integer(attrs, 2, entity_id, "application_protocol_year")?;
        let app_ref = read_entity_ref(attrs, 3, entity_id, "application")?;
        let Some(application) = ctx.id_cache.get::<crate::ir::ApplicationContextId>(app_ref) else {
            return Ok(()); // AC missing; drop silently
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .application_protocol_definitions
            .push(ApplicationProtocolDefinition {
                status,
                application_interpreted_model_schema_name: name,
                application_protocol_year: year,
                application,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        _buf: &mut WriteBuffer,
        _apd: ApplicationProtocolDefinition,
    ) -> Result<u64, WriteError> {
        // APD is emitted inline by `emit_application_context` in the
        // assembly chain (it needs the AC step-id which is a local
        // variable there). This handler's `write` is therefore never
        // called from the writer pipeline.
        unreachable!("APPLICATION_PROTOCOL_DEFINITION is emitted by emit_application_context")
    }
}
