//! `GENERAL_PROPERTY` handler — Pass 8-4.
//!
//! AP242 user-defined attribute definition `(id, name, description)`.
//! Pushed into the `general_properties` arena; the GPA handler resolves
//! its `base_definition` against `general_property_id_map`.

use crate::entities::SimpleEntityHandler;
use crate::ir::PropertyPool;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::GeneralProperty;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GeneralPropertyHandler;

#[step_entity(name = "GENERAL_PROPERTY", pass = Pass8GeneralProperty)]
impl SimpleEntityHandler for GeneralPropertyHandler {
    type WriteInput = GeneralProperty;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "GENERAL_PROPERTY")?;
        // `id` / `name` are mandatory strings; read leniently (Unset → "")
        // for consistency with the PROPERTY_DEFINITION handler.
        let id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        let desc_str = read_string_or_unset(attrs, 2, entity_id, "description")?;
        let description = if desc_str.is_empty() {
            None
        } else {
            Some(desc_str.to_owned())
        };

        let gp_id = ctx
            .properties
            .get_or_insert_with(PropertyPool::default)
            .general_properties
            .push(GeneralProperty {
                id,
                name,
                description,
            });
        ctx.general_property_id_map.insert(entity_id, gp_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gp: GeneralProperty) -> Result<u64, WriteError> {
        let desc_attr = match gp.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "GENERAL_PROPERTY",
            vec![
                Attribute::String(gp.id),
                Attribute::String(gp.name),
                desc_attr,
            ],
        ))
    }
}
