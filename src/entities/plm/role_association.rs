//! `ROLE_ASSOCIATION` handler plm. STEP positional shape
//! `(role, item_with_role)` per `AP214e3` schema lines 9773–9776.
//! `item_with_role` is the AP214 `role_select` SELECT — step-io
//! currently scopes to `APPLIED_DOCUMENT_REFERENCE` only.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PlmPool, RoleAssociation, RoleSelect};
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
        check_count(attrs, 2, entity_id, "ROLE_ASSOCIATION")?;
        let role_ref = read_entity_ref(attrs, 0, entity_id, "role")?;
        let item_ref = read_entity_ref(attrs, 1, entity_id, "item_with_role")?;
        let Some(&role) = ctx.plm_object_role_id_map.get(&role_ref) else {
            return Ok(());
        };
        let item_with_role = if let Some(&adr_id) = ctx.plm_document_reference_id_map.get(&item_ref)
        {
            RoleSelect::DocumentReference(adr_id)
        } else {
            return Ok(()); // unsupported SELECT variant (Approval/DTA/etc) — drop
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.role_associations.push(RoleAssociation {
            role,
            item_with_role,
        });
        ctx.plm_role_association_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ra: RoleAssociation) -> Result<u64, WriteError> {
        let role_step = buf.step_id(ra.role);
        let item_step = match ra.item_with_role {
            RoleSelect::DocumentReference(id) => buf.step_id(id),
        };
        Ok(buf.push_simple(
            "ROLE_ASSOCIATION",
            vec![
                Attribute::EntityRef(role_step),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}
