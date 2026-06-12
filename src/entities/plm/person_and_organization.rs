//! `PERSON_AND_ORGANIZATION` handler — plm (2-layer path). The dangling
//! probe (`graph.get(..).is_none()`) stays in the handler; lower receives
//! the pre-computed flags (lower takes no `&EntityGraph`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::PersonAndOrganization;
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
        let early = bind::bind_person_and_organization(entity_id, attrs)?;
        let person_dangling = graph.get(early.the_person).is_none();
        let org_dangling = graph.get(early.the_organization).is_none();
        lower::lower_person_and_organization(ctx, entity_id, &early, person_dangling, org_dangling)
    }

    fn write(buf: &mut WriteBuffer, p: PersonAndOrganization) -> Result<u64, WriteError> {
        let person_step = buf.step_id(p.the_person);
        let org_step = buf.step_id(p.the_organization);
        let early = lift::lift_person_and_organization(person_step, org_step);
        Ok(serialize::serialize_person_and_organization(buf, &early))
    }
}
