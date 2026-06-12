//! `CC_DESIGN_APPROVAL` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalItem, CcDesignApproval};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CcDesignApprovalHandler;

#[step_entity(name = "CC_DESIGN_APPROVAL")]
impl SimpleEntityHandler for CcDesignApprovalHandler {
    type WriteInput = CcDesignApproval;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cc_design_approval(entity_id, attrs)?;
        lower::lower_cc_design_approval(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CcDesignApproval) -> Result<u64, WriteError> {
        let approval_step = buf.step_id(c.assigned_approval);
        let items: Vec<u64> = c
            .items
            .into_iter()
            .map(|item| match item {
                ApprovalItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_cc_design_approval(approval_step, items);
        Ok(serialize::serialize_cc_design_approval(buf, &early))
    }
}
