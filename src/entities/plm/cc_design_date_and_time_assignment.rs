//! `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{CcDesignDateAndTimeAssignment, DateTimeItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CcDesignDateAndTimeAssignmentHandler;

#[step_entity(name = "CC_DESIGN_DATE_AND_TIME_ASSIGNMENT")]
impl SimpleEntityHandler for CcDesignDateAndTimeAssignmentHandler {
    type WriteInput = CcDesignDateAndTimeAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cc_design_date_and_time_assignment(entity_id, attrs)?;
        lower::lower_cc_design_date_and_time_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CcDesignDateAndTimeAssignment) -> Result<u64, WriteError> {
        let dt_step = buf.step_id(c.assigned_date_and_time);
        let role_step = buf.step_id(c.role);
        let items: Vec<u64> = c
            .items
            .into_iter()
            .map(|item| match item {
                DateTimeItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_cc_design_date_and_time_assignment(dt_step, role_step, items);
        Ok(serialize::serialize_cc_design_date_and_time_assignment(
            buf, &early,
        ))
    }
}
