//! `PERSON` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Person;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonHandler;

#[step_entity(name = "PERSON")]
impl SimpleEntityHandler for PersonHandler {
    type WriteInput = Person;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_person(entity_id, attrs)?;
        lower::lower_person(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: Person) -> Result<u64, WriteError> {
        let early = lift::lift_person(p);
        Ok(serialize::serialize_person(buf, &early))
    }
}
