//! Hand-rolled `PROPERTY_DEFINITION_REPRESENTATION` converter (Pass 8-3).
//!
//! Plan 7 stage C5 migrated `MEASURE_REPRESENTATION_ITEM` and
//! `PROPERTY_DEFINITION` into `entities/property/`. PDR stays here
//! because its read body needs `&EntityGraph` to walk the bound
//! `REPRESENTATION` (a generic entity name shared with MDGPR / SR — a
//! per-pass map would conflate them). The handler trait's reader
//! signature does not carry `&graph`, so PDR keeps its hand-rolled loop
//! in `passes.rs` and routes through this method.
//!
//! See the `DOMAIN_TBD` marker on the call site for the Plan 7+ IR
//! Roadmap follow-up: CDSR + PDR share the graph-traversal trait limit
//! and should be unblocked together.

use super::ReaderContext;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{Property, PropertyMeasure, PropertyPool};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};

impl ReaderContext {
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
