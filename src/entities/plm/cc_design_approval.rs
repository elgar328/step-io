//! `CC_DESIGN_APPROVAL` handler plm. Variant of the
//! `approval_assignment` arena enum. STEP entity name lacks the
//! `_ASSIGNMENT` suffix that the Applied sibling carries.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalAssignment, ApprovalItem, CcDesignApproval, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
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
        check_count(attrs, 2, entity_id, "CC_DESIGN_APPROVAL")?;
        let approval_ref = read_entity_ref(attrs, 0, entity_id, "assigned_approval")?;
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let Some(&assigned_approval) = ctx.plm_approval_id_map.get(&approval_ref) else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(ApprovalItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .approval_assignments
            .push(ApprovalAssignment::CcDesign(CcDesignApproval {
                assigned_approval,
                items,
            }));
        ctx.plm_aa_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CcDesignApproval) -> Result<u64, WriteError> {
        let approval_step = buf.plm_approval_step_ids[c.assigned_approval.0 as usize];
        let mut item_refs = Vec::with_capacity(c.items.len());
        for item in c.items {
            let step_id = match item {
                ApprovalItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "CC_DESIGN_APPROVAL",
            vec![
                Attribute::EntityRef(approval_step),
                Attribute::List(item_refs),
            ],
        ))
    }
}
