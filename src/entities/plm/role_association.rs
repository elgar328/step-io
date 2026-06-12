//! `ROLE_ASSOCIATION` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::RoleAssociation;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RoleAssociationHandler;

#[step_entity(name = "ROLE_ASSOCIATION")]
impl SimpleEntityHandler for RoleAssociationHandler {
    type WriteInput = RoleAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_role_association(entity_id, attrs)?;
        lower::lower_role_association(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ra: RoleAssociation) -> Result<u64, WriteError> {
        let role_step = buf.step_id(ra.role);
        let item_step = ra.item_with_role.emit_select(buf);
        let early = lift::lift_role_association(role_step, item_step);
        Ok(serialize::serialize_role_association(buf, &early))
    }
}
