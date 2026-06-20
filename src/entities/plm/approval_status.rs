//! `APPROVAL_STATUS` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::ApprovalStatus;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalStatusHandler;

#[step_entity(name = "APPROVAL_STATUS")]
impl SimpleEntityHandler for ApprovalStatusHandler {
    type WriteInput = ApprovalStatus;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_approval_status(entity_id, attrs)?;
        lower::lower_approval_status(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: ApprovalStatus) -> Result<u64, WriteError> {
        let early = lift::lift_approval_status(v);
        Ok(serialize::serialize_approval_status(buf, &early))
    }
}
