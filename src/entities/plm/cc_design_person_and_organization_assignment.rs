//! `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{CcDesignPersonAndOrganizationAssignment, PersonOrganizationItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CcDesignPersonAndOrganizationAssignmentHandler;

#[step_entity(name = "CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT")]
impl SimpleEntityHandler for CcDesignPersonAndOrganizationAssignmentHandler {
    type WriteInput = CcDesignPersonAndOrganizationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cc_design_person_and_organization_assignment(entity_id, attrs)?;
        lower::lower_cc_design_person_and_organization_assignment(ctx, entity_id, early)
    }

    fn write(
        buf: &mut WriteBuffer,
        c: CcDesignPersonAndOrganizationAssignment,
    ) -> Result<u64, WriteError> {
        let po_step = buf.step_id(c.assigned_person_and_organization);
        let role_step = buf.step_id(c.role);
        let items: Vec<u64> = c
            .items
            .into_iter()
            .map(|item| match item {
                PersonOrganizationItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early =
            lift::lift_cc_design_person_and_organization_assignment(po_step, role_step, items);
        Ok(serialize::serialize_cc_design_person_and_organization_assignment(buf, &early))
    }
}
