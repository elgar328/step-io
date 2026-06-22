//! `ORGANIZATION` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Organization;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OrganizationHandler;

#[step_entity(name = "ORGANIZATION")]
impl SimpleEntityHandler for OrganizationHandler {
    type WriteInput = Organization;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_organization(entity_id, attrs)?;
        lower::lower_organization(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, o: Organization) -> Result<u64, WriteError> {
        let early = lift::lift_organization(o);
        Ok(serialize::serialize_organization(buf, &early))
    }
}
