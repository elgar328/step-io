//! `PERSON_AND_ORGANIZATION_ROLE` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::PersonAndOrganizationRole;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonAndOrganizationRoleHandler;

#[step_entity(name = "PERSON_AND_ORGANIZATION_ROLE")]
impl SimpleEntityHandler for PersonAndOrganizationRoleHandler {
    type WriteInput = PersonAndOrganizationRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_person_and_organization_role(entity_id, attrs)?;
        lower::lower_person_and_organization_role(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: PersonAndOrganizationRole) -> Result<u64, WriteError> {
        let early = lift::lift_person_and_organization_role(v);
        Ok(serialize::serialize_person_and_organization_role(
            buf, &early,
        ))
    }
}
