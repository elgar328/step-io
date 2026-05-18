//! `PERSON_AND_ORGANIZATION` handler — Pass 9-6 plm. Depends on Person +
//! Organization arenas populated by `Pass9PlmPoLeaves`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PersonAndOrganization, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonAndOrganizationHandler;

#[step_entity(name = "PERSON_AND_ORGANIZATION", pass = Pass9PlmPersonAndOrganization)]
impl SimpleEntityHandler for PersonAndOrganizationHandler {
    type WriteInput = PersonAndOrganization;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PERSON_AND_ORGANIZATION")?;
        let person_ref = read_entity_ref(attrs, 0, entity_id, "the_person")?;
        let org_ref = read_entity_ref(attrs, 1, entity_id, "the_organization")?;
        let Some(&the_person) = ctx.plm_person_id_map.get(&person_ref) else {
            return Ok(());
        };
        let Some(&the_organization) = ctx.plm_organization_id_map.get(&org_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.person_and_organizations.push(PersonAndOrganization {
            the_person,
            the_organization,
        });
        ctx.plm_p_and_o_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: PersonAndOrganization) -> Result<u64, WriteError> {
        let person_step = buf.plm_person_step_ids[p.the_person.0 as usize];
        let org_step = buf.plm_organization_step_ids[p.the_organization.0 as usize];
        Ok(buf.push_simple(
            "PERSON_AND_ORGANIZATION",
            vec![
                Attribute::EntityRef(person_step),
                Attribute::EntityRef(org_step),
            ],
        ))
    }
}
