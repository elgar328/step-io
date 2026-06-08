//! `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` handler plm.
//! Variant of the `identification_assignment` arena enum. 4 fields —
//! `source` ref distinguishes it from the Applied sibling.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedExternalIdentificationAssignment, IdentificationItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct AppliedExternalIdentificationAssignmentHandler;

#[step_entity(name = "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT")]
impl SimpleEntityHandler for AppliedExternalIdentificationAssignmentHandler {
    type WriteInput = AppliedExternalIdentificationAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            4,
            entity_id,
            "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT",
        )?;
        let assigned_id = read_string_or_unset(attrs, 0, entity_id, "assigned_id")?.to_owned();
        let role_ref = read_entity_ref(attrs, 1, entity_id, "role")?;
        let source_ref = read_entity_ref(attrs, 2, entity_id, "source")?;
        let item_refs = read_entity_ref_list(attrs, 3, entity_id, "items")?;
        let Some(role) = ctx
            .id_cache
            .get::<crate::ir::IdentificationRoleId>(role_ref)
        else {
            return Ok(());
        };
        let Some(source) = ctx.id_cache.get::<crate::ir::ExternalSourceId>(source_ref) else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(IdentificationItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .identification_assignments
            .push(AppliedExternalIdentificationAssignment {
                assigned_id,
                role,
                source,
                items,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        a: AppliedExternalIdentificationAssignment,
    ) -> Result<u64, WriteError> {
        let role_step = buf.step_id(a.role);
        let source_step = buf.step_id(a.source);
        let mut item_refs = Vec::with_capacity(a.items.len());
        for item in a.items {
            let step_id = match item {
                IdentificationItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT",
            vec![
                Attribute::String(a.assigned_id),
                Attribute::EntityRef(role_step),
                Attribute::EntityRef(source_step),
                Attribute::List(item_refs),
            ],
        ))
    }
}
