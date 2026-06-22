//! `GROUP` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Group;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GroupHandler;

#[step_entity(name = "GROUP")]
impl SimpleEntityHandler for GroupHandler {
    type WriteInput = Group;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_group(entity_id, attrs)?;
        lower::lower_group(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: Group) -> Result<u64, WriteError> {
        let early = lift::lift_group(v);
        Ok(serialize::serialize_group(buf, &early))
    }
}
