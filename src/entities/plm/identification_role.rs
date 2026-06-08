//! `IDENTIFICATION_ROLE` handler plm Identification leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_optional_string, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{IdentificationRole, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct IdentificationRoleHandler;

#[step_entity(name = "IDENTIFICATION_ROLE")]
impl SimpleEntityHandler for IdentificationRoleHandler {
    type WriteInput = IdentificationRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "IDENTIFICATION_ROLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_optional_string(attrs, 1, entity_id, "description")?;
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .identification_roles
            .push(IdentificationRole { name, description });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: IdentificationRole) -> Result<u64, WriteError> {
        let desc_attr = match r.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "IDENTIFICATION_ROLE",
            vec![Attribute::String(r.name), desc_attr],
        ))
    }
}
