//! `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedExternalIdentificationAssignment, IdentificationItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AppliedExternalIdentificationAssignmentHandler;

#[step_entity(name = "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedExternalIdentificationAssignmentHandler {
    type WriteInput = AppliedExternalIdentificationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_applied_external_identification_assignment(entity_id, attrs)?;
        lower::lower_applied_external_identification_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedExternalIdentificationAssignment,
    ) -> Result<u64, WriteError> {
        let role_step = buf.step_id(a.role);
        let source_step = buf.step_id(a.source);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                IdentificationItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_external_identification_assignment(
            role_step,
            source_step,
            a.assigned_id,
            items,
        );
        Ok(serialize::serialize_applied_external_identification_assignment(buf, &early))
    }
}
