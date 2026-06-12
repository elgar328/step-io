//! `APPLIED_GROUP_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedGroupAssignment, GroupItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedGroupAssignmentHandler;

#[step_entity(name = "APPLIED_GROUP_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedGroupAssignmentHandler {
    type WriteInput = AppliedGroupAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_group_assignment(entity_id, attrs)?;
        lower::lower_applied_group_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedGroupAssignment) -> Result<u64, WriteError> {
        let assigned_group_step = buf.step_id(a.assigned_group);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                GroupItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_group_assignment(assigned_group_step, items);
        Ok(serialize::serialize_applied_group_assignment(buf, &early))
    }
}
