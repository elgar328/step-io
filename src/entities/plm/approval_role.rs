//! `APPROVAL_ROLE` handler — Pass 9-8 plm Approval leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalRole, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalRoleHandler;

#[step_entity(name = "APPROVAL_ROLE", pass = Pass9PlmApprovalLeaves)]
impl SimpleEntityHandler for ApprovalRoleHandler {
    type WriteInput = ApprovalRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "APPROVAL_ROLE")?;
        let role = read_string_or_unset(attrs, 0, entity_id, "role")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.approval_roles.push(ApprovalRole { role });
        ctx.plm_approval_role_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: ApprovalRole) -> Result<u64, WriteError> {
        Ok(buf.push_simple("APPROVAL_ROLE", vec![Attribute::String(r.role)]))
    }
}
