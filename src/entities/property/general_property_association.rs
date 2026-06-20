//! `GENERAL_PROPERTY_ASSOCIATION` handler — property (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::property::GeneralPropertyAssociation;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GeneralPropertyAssociationWriteInput {
    pub(crate) gpa: GeneralPropertyAssociation,
    pub(crate) base_step: u64,
    pub(crate) derived_step: u64,
}

pub(crate) struct GeneralPropertyAssociationHandler;

#[step_entity(name = "GENERAL_PROPERTY_ASSOCIATION")]
impl SimpleEntityHandler for GeneralPropertyAssociationHandler {
    type WriteInput = GeneralPropertyAssociationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_general_property_association(entity_id, attrs)?;
        lower::lower_general_property_association(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        GeneralPropertyAssociationWriteInput {
            gpa,
            base_step,
            derived_step,
        }: GeneralPropertyAssociationWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_general_property_association(
            gpa.name,
            gpa.description,
            base_step,
            derived_step,
        );
        Ok(serialize::serialize_general_property_association(
            buf, &early,
        ))
    }
}
