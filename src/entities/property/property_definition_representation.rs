//! `PROPERTY_DEFINITION_REPRESENTATION` handler.
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

#[step_entity(name = "PROPERTY_DEFINITION_REPRESENTATION")]
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

        let Some((pd_name, pd_desc)) = ctx.property_def_map.get(&pd_ref).cloned() else {
            return Ok(()); // PD silently skipped (unresolved / unsupported target)
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
            // `used_representation` is a modelled representation subtype (e.g.
            // SHAPE_REPRESENTATION) rather than a generic descriptive-property
            // REPRESENTATION. Record a PD↔representation link for the writer to
            // reference the existing representation; resolve_pdr_links filters
            // by `property_def_step_to_id` / `repr_id_map`. (PD already gated
            // resolved above.)
            if ctx.repr_id_map.contains_key(&repr_ref) {
                ctx.pdr_link_refs.push((pd_ref, repr_ref));
            }
            return Ok(());
        }
        let representation_name = read_string_or_unset(repr_attrs, 0, repr_ref, "name")?.to_owned();
        let item_refs = read_entity_ref_list(repr_attrs, 1, repr_ref, "items")?;
        let ctx_ref = read_entity_ref(repr_attrs, 2, repr_ref, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);

        let items: Vec<PropertyItem> = item_refs
            .into_iter()
            .filter_map(|r| {
                // MEASURE_REPRESENTATION_ITEM lives in the representation_item
                // arena — reference it so the writer emits it once. Guard on the
                // variant: repr_item_id_map also holds QRI / VRI.
                if let Some(&id) = ctx.repr_item_id_map.get(&r) {
                    if matches!(
                        ctx.representation_items[id],
                        crate::ir::representation_item::RepresentationItem::MeasureRepresentationItem(_)
                    ) {
                        return Some(PropertyItem::MeasureItem(id));
                    }
                }
                ctx.descriptive_item_map
                    .get(&r)
                    .cloned()
                    .map(PropertyItem::Descriptive)
            })
            .collect();

        // Resolve the source PD step ref to the new PropertyDefinition
        // arena id so the writer can fetch the cached PD step id during
        // PDR emit (no longer re-emits PD inline). PD handler always
        // pushes when its target resolves to a Product, matching the
        // gate above, so this lookup never misses in practice — but stay
        // defensive on kernel-built IR.
        let Some(&definition) = ctx.property_def_step_to_id.get(&pd_ref) else {
            return Ok(());
        };
        let prop_id = ctx
            .properties
            .get_or_insert_with(PropertyPool::default)
            .properties
            .push(Property {
                name: pd_name,
                description: pd_desc,
                definition,
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
