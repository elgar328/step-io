//! `DATE_TIME_ROLE` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DateTimeRole;
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
        let early = bind::bind_date_time_role(entity_id, attrs)?;
        lower::lower_date_time_role(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: DateTimeRole) -> Result<u64, WriteError> {
        let early = lift::lift_date_time_role(v);
        Ok(serialize::serialize_date_time_role(buf, &early))
    }
}
