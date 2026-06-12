//! `APPROVAL_DATE_TIME` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::ApprovalDateTime;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalDateTimeHandler;

#[step_entity(name = "APPROVAL_DATE_TIME")]
impl SimpleEntityHandler for ApprovalDateTimeHandler {
    type WriteInput = ApprovalDateTime;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_approval_date_time(entity_id, attrs)?;
        lower::lower_approval_date_time(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: ApprovalDateTime) -> Result<u64, WriteError> {
        let dt_step = a.date_time.emit_select(buf);
        let approval_step = buf.step_id(a.dated_approval);
        let early = lift::lift_approval_date_time(dt_step, approval_step);
        Ok(serialize::serialize_approval_date_time(buf, &early))
    }
}
