//! `ORGANIZATION` handler — Pass 9-5 plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_optional_string, read_string};
use crate::ir::error::ConvertError;
use crate::ir::plm::{Organization, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OrganizationHandler;

#[step_entity(name = "ORGANIZATION", pass = Pass9PlmPoLeaves)]
impl SimpleEntityHandler for OrganizationHandler {
    type WriteInput = Organization;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ORGANIZATION")?;
        let id = read_optional_string(attrs, 0, entity_id, "id")?;
        let name = read_string(attrs, 1, entity_id, "name")?.to_owned();
        let description = read_string(attrs, 2, entity_id, "description")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let o_id = pool.organizations.push(Organization {
            id,
            name,
            description,
        });
        ctx.plm_organization_id_map.insert(entity_id, o_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, o: Organization) -> Result<u64, WriteError> {
        let id_attr = match o.id {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "ORGANIZATION",
            vec![
                id_attr,
                Attribute::String(o.name),
                Attribute::String(o.description),
            ],
        ))
    }
}
