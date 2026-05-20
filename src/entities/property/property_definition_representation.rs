//! `PROPERTY_DEFINITION_REPRESENTATION` handler — Pass 8-3.
//!
//! Reader walks the bound `REPRESENTATION` directly off the graph because
//! `REPRESENTATION` is a generic entity name shared with MDGPR / SR — a
//! per-pass map would conflate them. Writer emits the two-attr form
//! binding a `PROPERTY_DEFINITION` to a `REPRESENTATION`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{Property, PropertyItem, PropertyPool};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PropertyDefinitionRepresentationWriteInput {
    pub(crate) pd: u64,
    pub(crate) repr: u64,
}

pub(crate) struct PropertyDefinitionRepresentationHandler;

#[step_entity(name = "PROPERTY_DEFINITION_REPRESENTATION", pass = Pass8Pdr)]
impl SimpleEntityHandler for PropertyDefinitionRepresentationHandler {
    type WriteInput = PropertyDefinitionRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PROPERTY_DEFINITION_REPRESENTATION")?;
        let pd_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let repr_ref = read_entity_ref(attrs, 1, entity_id, "used_representation")?;

        let Some((pd_name, pd_desc, pd_target)) = ctx.property_def_map.get(&pd_ref).cloned() else {
            return Ok(()); // PD silently skipped (Pattern B / unresolved target)
        };

        // Walk the graph for the bound REPRESENTATION. Direct read — REPR
        // is shared with MDGPR / SR so a generic map would conflate them.
        let Some(RawEntity::Simple {
            name: repr_name_step,
            attributes: repr_attrs,
            ..
        }) = graph.get(repr_ref)
        else {
            return Ok(());
        };
        if repr_name_step != "REPRESENTATION" {
            return Ok(());
        }
        let representation_name = read_string_or_unset(repr_attrs, 0, repr_ref, "name")?.to_owned();
        let item_refs = read_entity_ref_list(repr_attrs, 1, repr_ref, "items")?;
        let ctx_ref = read_entity_ref(repr_attrs, 2, repr_ref, "context_of_items")?;
        let context = ctx.context_id_map.get(&ctx_ref).copied();

        let items: Vec<PropertyItem> = item_refs
            .into_iter()
            .filter_map(|r| {
                if let Some(m) = ctx.measure_item_map.get(&r) {
                    Some(PropertyItem::Measure(m.clone()))
                } else {
                    ctx.descriptive_item_map
                        .get(&r)
                        .cloned()
                        .map(PropertyItem::Descriptive)
                }
            })
            .collect();

        let prop_id = ctx
            .properties
            .get_or_insert_with(PropertyPool::default)
            .properties
            .push(Property {
                name: pd_name,
                description: pd_desc,
                target: pd_target,
                representation_name,
                context,
                items,
            });
        // Record PD `#N → PropertyId` so the GPA reader can resolve a
        // `derived_definition` pointing at this PROPERTY_DEFINITION.
        ctx.property_step_to_id.insert(pd_ref, prop_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        PropertyDefinitionRepresentationWriteInput { pd, repr }: PropertyDefinitionRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PROPERTY_DEFINITION_REPRESENTATION",
            vec![Attribute::EntityRef(pd), Attribute::EntityRef(repr)],
        ))
    }
}
