//! `IDENTIFICATION_ROLE` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::IdentificationRole;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct IdentificationRoleHandler;

#[step_entity(name = "IDENTIFICATION_ROLE")]
impl SimpleEntityHandler for IdentificationRoleHandler {
    type WriteInput = IdentificationRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_identification_role(entity_id, attrs)?;
        lower::lower_identification_role(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: IdentificationRole) -> Result<u64, WriteError> {
        let early = lift::lift_identification_role(v);
        Ok(serialize::serialize_identification_role(buf, &early))
    }
}
