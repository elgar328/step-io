//! `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` handler plm.
//! Variant of the `security_classification_assignment` arena enum.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{
    AppliedSecurityClassificationAssignment, PlmPool, SecurityClassificationAssignment,
    SecurityClassificationItem,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct AppliedSecurityClassificationAssignmentHandler;

#[step_entity(name = "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedSecurityClassificationAssignmentHandler {
    type WriteInput = AppliedSecurityClassificationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            2,
            entity_id,
            "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT",
        )?;
        let sc_ref = read_entity_ref(attrs, 0, entity_id, "assigned_security_classification")?;
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let Some(assigned_security_classification) =
            ctx.id_cache
                .get::<crate::ir::SecurityClassificationId>(sc_ref)
        else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(SecurityClassificationItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.security_classification_assignments.push(
            SecurityClassificationAssignment::Applied(AppliedSecurityClassificationAssignment {
                assigned_security_classification,
                items,
            }),
        );
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedSecurityClassificationAssignment,
    ) -> Result<u64, WriteError> {
        let sc_step = buf.step_id(a.assigned_security_classification);
        let mut item_refs = Vec::with_capacity(a.items.len());
        for item in a.items {
            let step_id = match item {
                SecurityClassificationItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT",
            vec![Attribute::EntityRef(sc_step), Attribute::List(item_refs)],
        ))
    }
}
