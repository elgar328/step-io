//! `PERSON_AND_ORGANIZATION_ROLE` handler — Pass 9-5 plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PersonAndOrganizationRole, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonAndOrganizationRoleHandler;

#[step_entity(name = "PERSON_AND_ORGANIZATION_ROLE", pass = Pass9PlmPoLeaves)]
impl SimpleEntityHandler for PersonAndOrganizationRoleHandler {
    type WriteInput = PersonAndOrganizationRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PERSON_AND_ORGANIZATION_ROLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let r_id = pool.p_and_o_roles.push(PersonAndOrganizationRole { name });
        ctx.plm_p_and_o_role_id_map.insert(entity_id, r_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: PersonAndOrganizationRole) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PERSON_AND_ORGANIZATION_ROLE",
            vec![Attribute::String(r.name)],
        ))
    }
}
