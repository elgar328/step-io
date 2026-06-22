//! `PROPERTY_DEFINITION_REPRESENTATION` handler — 2-layer path. `read` =
//! generated `bind` (two refs) then hand `lower_property_definition_representation`
//! (which takes the `graph`: the bound `REPRESENTATION` is walked directly — a
//! generic name shared with MDGPR / SR — and builds the `Property` bundle). The
//! orchestrator (`buffer/property.rs::emit_property`) resolves the PD/REPR step
//! ids and emits the wrapping `REPRESENTATION`; `write` lifts + generated-
//! serializes the two-attr PDR line.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PropertyDefinitionRepresentationWriteInput {
    pub(crate) pd: u64,
    pub(crate) repr: u64,
}

pub(crate) struct PropertyDefinitionRepresentationHandler;

#[step_entity(name = "PROPERTY_DEFINITION_REPRESENTATION")]
impl SimpleEntityHandler for PropertyDefinitionRepresentationHandler {
    type WriteInput = PropertyDefinitionRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        eg: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_property_definition_representation(entity_id, attrs)?;
        lower::lower_property_definition_representation(ctx, entity_id, early, eg)
    }

    fn write(
        buf: &mut WriteBuffer,
        PropertyDefinitionRepresentationWriteInput { pd, repr }: PropertyDefinitionRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_property_definition_representation(pd, repr);
        Ok(serialize::serialize_property_definition_representation(
            buf, &early,
        ))
    }
}
