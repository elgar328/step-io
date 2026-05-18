//! `APPLIED_GROUP_ASSIGNMENT` handler — Pass 9-21 plm Group linker.
//! STEP positional shape `(assigned_group, items)` per `AP214e3` schema
//! (`group_assignment` supertype lines 5795–5802 + own `items`).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedGroupAssignment, GroupItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct AppliedGroupAssignmentHandler;

#[step_entity(name = "APPLIED_GROUP_ASSIGNMENT", pass = Pass9PlmGa)]
impl SimpleEntityHandler for AppliedGroupAssignmentHandler {
    type WriteInput = AppliedGroupAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "APPLIED_GROUP_ASSIGNMENT")?;
        let group_ref = read_entity_ref(attrs, 0, entity_id, "assigned_group")?;
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let Some(&assigned_group) = ctx.plm_group_id_map.get(&group_ref) else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(GroupItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.group_assignments.push(AppliedGroupAssignment {
            assigned_group,
            items,
        });
        ctx.plm_ga_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedGroupAssignment) -> Result<u64, WriteError> {
        let group_step = buf.plm_group_step_ids[a.assigned_group.0 as usize];
        let mut item_refs = Vec::with_capacity(a.items.len());
        for item in a.items {
            let step_id = match item {
                GroupItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "APPLIED_GROUP_ASSIGNMENT",
            vec![Attribute::EntityRef(group_step), Attribute::List(item_refs)],
        ))
    }
}
