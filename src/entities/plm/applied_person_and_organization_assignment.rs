//! `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedPersonAndOrganizationAssignment, PersonOrganizationItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedPersonAndOrganizationAssignmentHandler;

#[step_entity(name = "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedPersonAndOrganizationAssignmentHandler {
    type WriteInput = AppliedPersonAndOrganizationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_person_and_organization_assignment(entity_id, attrs)?;
        lower::lower_applied_person_and_organization_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedPersonAndOrganizationAssignment,
    ) -> Result<u64, WriteError> {
        let assigned_person_and_organization_step = buf.step_id(a.assigned_person_and_organization);
        let role_step = buf.step_id(a.role);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                PersonOrganizationItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_person_and_organization_assignment(
            assigned_person_and_organization_step,
            role_step,
            items,
        );
        Ok(serialize::serialize_applied_person_and_organization_assignment(buf, &early))
    }
}
