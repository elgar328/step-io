//! `APPLIED_APPROVAL_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedApprovalAssignment, ApprovalItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedApprovalAssignmentHandler;

#[step_entity(name = "APPLIED_APPROVAL_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedApprovalAssignmentHandler {
    type WriteInput = AppliedApprovalAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_approval_assignment(entity_id, attrs)?;
        lower::lower_applied_approval_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedApprovalAssignment) -> Result<u64, WriteError> {
        let assigned_approval_step = buf.step_id(a.assigned_approval);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                ApprovalItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_approval_assignment(assigned_approval_step, items);
        Ok(serialize::serialize_applied_approval_assignment(
            buf, &early,
        ))
    }
}
