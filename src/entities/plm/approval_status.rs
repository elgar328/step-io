//! `APPROVAL_STATUS` handler — Pass 9-8 plm Approval leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalStatus, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "APPROVAL_STATUS")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.approval_statuses.push(ApprovalStatus { name });
        ctx.plm_approval_status_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: ApprovalStatus) -> Result<u64, WriteError> {
        Ok(buf.push_simple("APPROVAL_STATUS", vec![Attribute::String(s.name)]))
    }
}
