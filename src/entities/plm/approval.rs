//! `APPROVAL` handler plm. Depends on the approval-leaf handlers
//! for the status ref.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{Approval, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "APPROVAL")?;
        let status_ref = read_entity_ref(attrs, 0, entity_id, "status")?;
        let level = read_string_or_unset(attrs, 1, entity_id, "level")?.to_owned();
        let Some(status) = ctx.id_cache.get::<crate::ir::ApprovalStatusId>(status_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.approvals.push(Approval { status, level });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: Approval) -> Result<u64, WriteError> {
        let status_step = buf.step_id(a.status);
        Ok(buf.push_simple(
            "APPROVAL",
            vec![
                Attribute::EntityRef(status_step),
                Attribute::String(a.level),
            ],
        ))
    }
}
