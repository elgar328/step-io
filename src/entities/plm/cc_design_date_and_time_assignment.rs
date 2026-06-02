//! `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` handler — Pass 9-4 plm. Variant
//! of the `date_and_time_assignment` arena enum. Same field shape as
//! `APPLIED_DATE_AND_TIME_ASSIGNMENT`; differs only in AP214 context.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{CcDesignDateAndTimeAssignment, DateAndTimeAssignment, DateTimeItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct CcDesignDateAndTimeAssignmentHandler;

#[step_entity(name = "CC_DESIGN_DATE_AND_TIME_ASSIGNMENT")]
impl SimpleEntityHandler for CcDesignDateAndTimeAssignmentHandler {
    type WriteInput = CcDesignDateAndTimeAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CC_DESIGN_DATE_AND_TIME_ASSIGNMENT")?;
        let dt_ref = read_entity_ref(attrs, 0, entity_id, "assigned_date_and_time")?;
        let role_ref = read_entity_ref(attrs, 1, entity_id, "role")?;
        let item_refs = read_entity_ref_list(attrs, 2, entity_id, "items")?;
        let Some(&assigned_date_and_time) = ctx.plm_date_and_time_id_map.get(&dt_ref) else {
            return Ok(());
        };
        let Some(&role) = ctx.plm_date_time_role_id_map.get(&role_ref) else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(DateTimeItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .date_and_time_assignments
            .push(DateAndTimeAssignment::CcDesign(
                CcDesignDateAndTimeAssignment {
                    assigned_date_and_time,
                    role,
                    items,
                },
            ));
        ctx.plm_dta_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cc: CcDesignDateAndTimeAssignment) -> Result<u64, WriteError> {
        let dt_step = buf.plm_date_and_time_step_ids[cc.assigned_date_and_time.0 as usize];
        let role_step = buf.plm_date_time_role_step_ids[cc.role.0 as usize];
        let mut item_refs = Vec::with_capacity(cc.items.len());
        for item in cc.items {
            let step_id = match item {
                DateTimeItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "CC_DESIGN_DATE_AND_TIME_ASSIGNMENT",
            vec![
                Attribute::EntityRef(dt_step),
                Attribute::EntityRef(role_step),
                Attribute::List(item_refs),
            ],
        ))
    }
}
