//! `PERSON_AND_ORGANIZATION` handler plm. Depends on Person +
//! Organization arenas populated by the Person / Organization handlers.

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

#[step_entity(name = "PERSON_AND_ORGANIZATION")]
impl SimpleEntityHandler for PersonAndOrganizationHandler {
    type WriteInput = PersonAndOrganization;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PERSON_AND_ORGANIZATION")?;
        let person_ref = read_entity_ref(attrs, 0, entity_id, "the_person")?;
        let org_ref = read_entity_ref(attrs, 1, entity_id, "the_organization")?;
        let person = ctx.plm_person_id_map.get(&person_ref).copied();
        let org = ctx.plm_organization_id_map.get(&org_ref).copied();
        let (Some(the_person), Some(the_organization)) = (person, org) else {
            // A required reference did not resolve. When it is dangling — points
            // to no entity defined in the file (e.g. the #18446744073709551615
            // sentinel some anonymizers emit for a scrubbed person) — the P&O is
            // non-standard: drop it as a normalization and record the id so the
            // referencing assignments / approvals drop as normalizations too.
            // (A ref that *is* defined but whose PERSON / ORGANIZATION step-io
            // did not model is a separate gap — keep the silent drop.)
            if graph.get(person_ref).is_none() || graph.get(org_ref).is_none() {
                ctx.warnings.push(ConvertError::NonStandardInput {
                    field: "PERSON_AND_ORGANIZATION".into(),
                    count: 1,
                    normalized_to: "dropped (dangling person/organization reference, non-standard)"
                        .into(),
                });
                ctx.nonstd_person_org_refs.insert(entity_id);
            }
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
