//! Property entity converters (Pass 8).
//!
//! Three sub-passes process the user-defined-attribute chain:
//! 8-1 `MEASURE_REPRESENTATION_ITEM`, 8-3 `PROPERTY_DEFINITION`,
//! 8-4 `PROPERTY_DEFINITION_REPRESENTATION`. The PDR converter walks the
//! graph directly to read the bound `REPRESENTATION` entity (no dedicated
//! sub-pass — `REPRESENTATION` entities are shared with `MDGPR`/`SR` so a
//! per-pass map would conflate them).
//!
//! Pattern A only — PDs targeting `PRODUCT_DEFINITION`. Pattern B
//! (geometric validation, target = `SHAPE_ASPECT`) is dropped at read time
//! pending ROADMAP item 4 (PMI scaffolding).

use super::ReaderContext;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{MeasureKind, Property, PropertyMeasure, PropertyPool};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 8-1: MEASURE_REPRESENTATION_ITEM
    // ------------------------------------------------------------------

    pub(super) fn convert_measure_representation_item(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MEASURE_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        // attrs[1] is a typed value (LENGTH_MEASURE / PLANE_ANGLE_MEASURE / ...).
        // Skip silently if the kind is outside the supported set — symmetric
        // ignorance keeps round-trip equality intact.
        let Some(Attribute::Typed { type_name, value }) = attrs.get(1) else {
            return Ok(());
        };
        let Some(kind) = match_measure_kind(type_name) else {
            return Ok(());
        };
        let Attribute::Real(measure_value) = value.as_ref() else {
            return Ok(());
        };
        // attrs[2] = unit_component — ignored. The bound REPRESENTATION's
        // `context_of_items` field (or the parent Property's `context`) is
        // the authoritative unit reference; the writer reproduces it from
        // there.
        self.measure_item_map.insert(
            entity_id,
            PropertyMeasure {
                name,
                kind,
                value: *measure_value,
            },
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 8-3: PROPERTY_DEFINITION
    // ------------------------------------------------------------------

    pub(super) fn convert_property_definition(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PROPERTY_DEFINITION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let desc_str = read_string_or_unset(attrs, 1, entity_id, "description")?;
        let description = if desc_str.is_empty() {
            None
        } else {
            Some(desc_str.to_owned())
        };
        let target_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
        // Resolve target via the assembly pass's pdef_to_product map. PDs
        // whose target doesn't resolve to a Product (SHAPE_ASPECT etc.) are
        // silently dropped — Pattern B per the module docs.
        let Some(&product_step_id) = self.pdef_to_product.get(&target_ref) else {
            return Ok(());
        };
        let Some(&product_id) = self.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };
        self.property_def_map
            .insert(entity_id, (name, description, product_id));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 8-4: PROPERTY_DEFINITION_REPRESENTATION (terminal — pushes
    // assembled `Property` records into `self.properties`).
    // ------------------------------------------------------------------

    pub(super) fn convert_property_definition_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PROPERTY_DEFINITION_REPRESENTATION")?;
        let pd_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let repr_ref = read_entity_ref(attrs, 1, entity_id, "used_representation")?;

        let Some((pd_name, pd_desc, pd_target)) = self.property_def_map.get(&pd_ref).cloned()
        else {
            return Ok(()); // PD silently skipped (Pattern B / unresolved target)
        };

        // Walk the graph for the bound REPRESENTATION. Direct read — no
        // dedicated sub-pass because REPR entities are shared with MDGPR /
        // SR and a generic map would conflate them.
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
        let context = self.context_id_map.get(&ctx_ref).copied();

        let items: Vec<PropertyMeasure> = item_refs
            .into_iter()
            .filter_map(|r| self.measure_item_map.get(&r).cloned())
            .collect();

        self.properties
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
        Ok(())
    }
}

fn match_measure_kind(type_name: &str) -> Option<MeasureKind> {
    match type_name {
        "LENGTH_MEASURE" => Some(MeasureKind::Length),
        "PLANE_ANGLE_MEASURE" => Some(MeasureKind::PlaneAngle),
        "SOLID_ANGLE_MEASURE" => Some(MeasureKind::SolidAngle),
        _ => None,
    }
}
