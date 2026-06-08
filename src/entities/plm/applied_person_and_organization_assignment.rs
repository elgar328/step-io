//! `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` handler plm.
//! Variant of the `person_and_organization_assignment` arena enum.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{
    AppliedPersonAndOrganizationAssignment, PersonAndOrganizationAssignment,
    PersonOrganizationItem, PlmPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct AppliedPersonAndOrganizationAssignmentHandler;

#[step_entity(name = "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedPersonAndOrganizationAssignmentHandler {
    type WriteInput = AppliedPersonAndOrganizationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT",
        )?;
        let po_ref = read_entity_ref(attrs, 0, entity_id, "assigned_person_and_organization")?;
        let role_ref = read_entity_ref(attrs, 1, entity_id, "role")?;
        let item_refs = read_entity_ref_list(attrs, 2, entity_id, "items")?;
        let Some(assigned_person_and_organization) = ctx
            .id_cache
            .get::<crate::ir::PersonAndOrganizationId>(po_ref)
        else {
            return Ok(());
        };
        let Some(role) = ctx
            .id_cache
            .get::<crate::ir::PersonAndOrganizationRoleId>(role_ref)
        else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(PersonOrganizationItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.person_and_organization_assignments.push(
            PersonAndOrganizationAssignment::Applied(AppliedPersonAndOrganizationAssignment {
                assigned_person_and_organization,
                role,
                items,
            }),
        );
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedPersonAndOrganizationAssignment,
    ) -> Result<u64, WriteError> {
        let po_step = buf.step_id(a.assigned_person_and_organization);
        let role_step = buf.step_id(a.role);
        let mut item_refs = Vec::with_capacity(a.items.len());
        for item in a.items {
            let step_id = match item {
                PersonOrganizationItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT",
            vec![
                Attribute::EntityRef(po_step),
                Attribute::EntityRef(role_step),
                Attribute::List(item_refs),
            ],
        ))
    }
}
