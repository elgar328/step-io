//! `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedSecurityClassificationAssignment, SecurityClassificationItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedSecurityClassificationAssignmentHandler;

#[step_entity(name = "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedSecurityClassificationAssignmentHandler {
    type WriteInput = AppliedSecurityClassificationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_security_classification_assignment(entity_id, attrs)?;
        lower::lower_applied_security_classification_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedSecurityClassificationAssignment,
    ) -> Result<u64, WriteError> {
        let assigned_security_classification_step = buf.step_id(a.assigned_security_classification);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                SecurityClassificationItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_security_classification_assignment(
            assigned_security_classification_step,
            items,
        );
        Ok(serialize::serialize_applied_security_classification_assignment(buf, &early))
    }
}
