//! `APPROVAL_PERSON_ORGANIZATION` handler plm. Depends on
//! the `APPROVAL` handler (`authorized_approval` ref),
//! the `PERSON_AND_ORGANIZATION` handler (`person_organization` SELECT) and
//! the approval-leaf handlers (`role` ref).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalPersonOrganization, PersonOrganizationSelect, PlmPool};
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
        check_count(attrs, 3, entity_id, "APPROVAL_PERSON_ORGANIZATION")?;
        let po_ref = read_entity_ref(attrs, 0, entity_id, "person_organization")?;
        let approval_ref = read_entity_ref(attrs, 1, entity_id, "authorized_approval")?;
        let role_ref = read_entity_ref(attrs, 2, entity_id, "role")?;
        let Some(po_id) = ctx
            .id_cache
            .get::<crate::ir::PersonAndOrganizationId>(po_ref)
        else {
            // The P&O was dropped as a dangling-reference cascade → surface a
            // MissingReference so the dispatcher reclassifies this approval the
            // same way (NS-dangling-reference-drop). Otherwise it is an
            // unsupported SELECT variant (direct PERSON / ORGANIZATION) → silent.
            if ctx.nonstandard_dropped_refs.contains(&po_ref) {
                return Err(ConvertError::MissingReference {
                    from: entity_id,
                    to: po_ref,
                    field_name: "person_organization",
                });
            }
            return Ok(());
        };
        let Some(authorized_approval) = ctx.id_cache.get::<crate::ir::ApprovalId>(approval_ref)
        else {
            return Ok(());
        };
        let Some(role) = ctx.id_cache.get::<crate::ir::ApprovalRoleId>(role_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .approval_person_organizations
            .push(ApprovalPersonOrganization {
                person_organization: PersonOrganizationSelect::PersonAndOrganization(po_id),
                authorized_approval,
                role,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: ApprovalPersonOrganization) -> Result<u64, WriteError> {
        let po_step = match a.person_organization {
            PersonOrganizationSelect::PersonAndOrganization(id) => buf.step_id(id),
        };
        let approval_step = buf.step_id(a.authorized_approval);
        let role_step = buf.step_id(a.role);
        Ok(buf.push_simple(
            "APPROVAL_PERSON_ORGANIZATION",
            vec![
                Attribute::EntityRef(po_step),
                Attribute::EntityRef(approval_step),
                Attribute::EntityRef(role_step),
            ],
        ))
    }
}
