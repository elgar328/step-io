//! `APPROVAL` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Approval;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalHandler;

#[step_entity(name = "APPROVAL")]
impl SimpleEntityHandler for ApprovalHandler {
    type WriteInput = Approval;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_approval(entity_id, attrs)?;
        lower::lower_approval(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: Approval) -> Result<u64, WriteError> {
        let status_step = buf.step_id(a.status);
        let early = lift::lift_approval(status_step, a.level);
        Ok(serialize::serialize_approval(buf, &early))
    }
}
