//! `APPROVAL_PERSON_ORGANIZATION` handler — plm (2-layer path: generated
//! bind/serialize + hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalPersonOrganization, PersonOrganizationSelect};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalPersonOrganizationHandler;

#[step_entity(name = "APPROVAL_PERSON_ORGANIZATION")]
impl SimpleEntityHandler for ApprovalPersonOrganizationHandler {
    type WriteInput = ApprovalPersonOrganization;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_approval_person_organization(entity_id, attrs)?;
        lower::lower_approval_person_organization(ctx, entity_id, early)
    }

    fn write(buf: &mut WriteBuffer, a: ApprovalPersonOrganization) -> Result<u64, WriteError> {
        let po_step = match a.person_organization {
            PersonOrganizationSelect::PersonAndOrganization(id) => buf.step_id(id),
        };
        let approval_step = buf.step_id(a.authorized_approval);
        let role_step = buf.step_id(a.role);
        let early = lift::lift_approval_person_organization(po_step, approval_step, role_step);
        Ok(serialize::serialize_approval_person_organization(
            buf, &early,
        ))
    }
}
