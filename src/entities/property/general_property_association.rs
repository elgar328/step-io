//! `GENERAL_PROPERTY_ASSOCIATION` handler — Pass 8-5.
//!
//! Links a `GENERAL_PROPERTY` (`base_definition`) to the property
//! occurrence it annotates (`derived_definition`, a `PROPERTY_DEFINITION`).
//! An association whose `base_definition` GP or `derived_definition` PD did
//! not resolve is dropped at read time with a warning.

use crate::entities::SimpleEntityHandler;
use crate::ir::PropertyPool;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{DerivedDefinitionItem, GeneralPropertyAssociation};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GeneralPropertyAssociationWriteInput {
    pub(crate) gpa: GeneralPropertyAssociation,
    pub(crate) base_step: u64,
    pub(crate) derived_step: u64,
}

pub(crate) struct GeneralPropertyAssociationHandler;

#[step_entity(name = "GENERAL_PROPERTY_ASSOCIATION", pass = Pass8Gpa)]
impl SimpleEntityHandler for GeneralPropertyAssociationHandler {
    type WriteInput = GeneralPropertyAssociationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "GENERAL_PROPERTY_ASSOCIATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let desc_str = read_string_or_unset(attrs, 1, entity_id, "description")?;
        let description = if desc_str.is_empty() {
            None
        } else {
            Some(desc_str.to_owned())
        };
        let base_ref = read_entity_ref(attrs, 2, entity_id, "base_definition")?;
        let derived_ref = read_entity_ref(attrs, 3, entity_id, "derived_definition")?;

        let Some(&base_definition) = ctx.general_property_id_map.get(&base_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "GENERAL_PROPERTY_ASSOCIATION.base_definition #{base_ref} is not a GENERAL_PROPERTY"
                ),
            });
            return Ok(());
        };
        // `derived_definition` is a SELECT; step-io covers the
        // `property_definition` member, resolved through the property arena.
        let Some(&prop_id) = ctx.property_step_to_id.get(&derived_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "GENERAL_PROPERTY_ASSOCIATION.derived_definition #{derived_ref} did not resolve to a PROPERTY_DEFINITION"
                ),
            });
            return Ok(());
        };

        ctx.properties
            .get_or_insert_with(PropertyPool::default)
            .general_property_associations
            .push(GeneralPropertyAssociation {
                name,
                description,
                base_definition,
                derived_definition: DerivedDefinitionItem::PropertyDefinition(prop_id),
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        GeneralPropertyAssociationWriteInput {
            gpa,
            base_step,
            derived_step,
        }: GeneralPropertyAssociationWriteInput,
    ) -> Result<u64, WriteError> {
        let desc_attr = match gpa.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "GENERAL_PROPERTY_ASSOCIATION",
            vec![
                Attribute::String(gpa.name),
                desc_attr,
                Attribute::EntityRef(base_step),
                Attribute::EntityRef(derived_step),
            ],
        ))
    }
}
