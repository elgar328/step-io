//! `PROPERTY_DEFINITION` handler — 2-layer path. `read` = generated `bind`
//! (mechanical 3-attr extraction) then hand `lower_property_definition` (the
//! `characterized_definition` SELECT dispatch + arena push, in
//! `early::lower::property`). `write` lifts + generated-serializes the bare PD
//! line; the orchestrator (`buffer/property.rs`) resolves the definition to its
//! emitted step id (`pdef_id`) and emits the surrounding `REPRESENTATION` + PDR.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PropertyDefinitionWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) pdef_id: u64,
}

pub(crate) struct PropertyDefinitionHandler;

#[step_entity(name = "PROPERTY_DEFINITION")]
impl SimpleEntityHandler for PropertyDefinitionHandler {
    type WriteInput = PropertyDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_property_definition(entity_id, attrs)?;
        lower::lower_property_definition(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        PropertyDefinitionWriteInput {
            name,
            description,
            pdef_id,
        }: PropertyDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_property_definition(name, description, pdef_id);
        Ok(serialize::serialize_property_definition(buf, &early))
    }
}
