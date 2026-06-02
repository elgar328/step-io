//! `DATE_TIME_ROLE` handler — Pass 9-1 plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{DateTimeRole, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DateTimeRoleHandler;

#[step_entity(name = "DATE_TIME_ROLE")]
impl SimpleEntityHandler for DateTimeRoleHandler {
    type WriteInput = DateTimeRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DATE_TIME_ROLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.date_time_roles.push(DateTimeRole { name });
        ctx.plm_date_time_role_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, role: DateTimeRole) -> Result<u64, WriteError> {
        Ok(buf.push_simple("DATE_TIME_ROLE", vec![Attribute::String(role.name)]))
    }
}
