//! `APPLIED_DATE_AND_TIME_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedDateAndTimeAssignment, DateTimeItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedDateAndTimeAssignmentHandler;

#[step_entity(name = "APPLIED_DATE_AND_TIME_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedDateAndTimeAssignmentHandler {
    type WriteInput = AppliedDateAndTimeAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_date_and_time_assignment(entity_id, attrs)?;
        lower::lower_applied_date_and_time_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedDateAndTimeAssignment) -> Result<u64, WriteError> {
        let assigned_date_and_time_step = buf.step_id(a.assigned_date_and_time);
        let role_step = buf.step_id(a.role);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                DateTimeItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_date_and_time_assignment(
            assigned_date_and_time_step,
            role_step,
            items,
        );
        Ok(serialize::serialize_applied_date_and_time_assignment(
            buf, &early,
        ))
    }
}
