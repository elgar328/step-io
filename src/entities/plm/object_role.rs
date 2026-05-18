//! `OBJECT_ROLE` handler — Pass 9-22 plm Role leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_optional_string, read_string};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ObjectRole, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ObjectRoleHandler;

#[step_entity(name = "OBJECT_ROLE", pass = Pass9PlmObjectRole)]
impl SimpleEntityHandler for ObjectRoleHandler {
    type WriteInput = ObjectRole;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "OBJECT_ROLE")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_optional_string(attrs, 1, entity_id, "description")?;
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.object_roles.push(ObjectRole { name, description });
        ctx.plm_object_role_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: ObjectRole) -> Result<u64, WriteError> {
        let desc_attr = match r.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple("OBJECT_ROLE", vec![Attribute::String(r.name), desc_attr]))
    }
}
