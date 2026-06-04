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
        let Some(&po_id) = ctx.plm_p_and_o_id_map.get(&po_ref) else {
            // The P&O was dropped as a non-standard (dangling-ref) normalization
            // — drop this approval-person-organization as a normalization too.
            if ctx.nonstd_person_org_refs.contains(&po_ref) {
                ctx.warnings.push(ConvertError::NonStandardInput {
                    field: "APPROVAL_PERSON_ORGANIZATION".into(),
                    count: 1,
                    normalized_to: "dropped (references non-standard PERSON_AND_ORGANIZATION)"
                        .into(),
                });
            }
            // Otherwise: unsupported SELECT variant (direct PERSON / ORGANIZATION).
            return Ok(());
        };
        let Some(&authorized_approval) = ctx.plm_approval_id_map.get(&approval_ref) else {
            return Ok(());
        };
        let Some(&role) = ctx.plm_approval_role_id_map.get(&role_ref) else {
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
        ctx.plm_approval_person_organization_id_map
            .insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: ApprovalPersonOrganization) -> Result<u64, WriteError> {
        let po_step = match a.person_organization {
            PersonOrganizationSelect::PersonAndOrganization(id) => {
                buf.plm_p_and_o_step_ids[id.0 as usize]
            }
        };
        let approval_step = buf.plm_approval_step_ids[a.authorized_approval.0 as usize];
        let role_step = buf.plm_approval_role_step_ids[a.role.0 as usize];
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
