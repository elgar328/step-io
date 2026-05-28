//! `PROPERTY_DEFINITION` handler — Pass 8-2.
//!
//! Reader stores `(name, description, ProductId)` in `property_def_map`
//! keyed by STEP entity id. Pattern B (target = `SHAPE_ASPECT`) is dropped
//! at read time so only Product-targeting PDs reach the PDR pass. Writer
//! emits the bare PD line; the surrounding `REPRESENTATION` + PDR are
//! handled in `buffer/property.rs::emit_property` (the orchestrator).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{
    CharacterizedDefinition, PropertyDefinition, PropertyDefinitionData, PropertyPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PropertyDefinitionWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) pdef_id: u64,
}

pub(crate) struct PropertyDefinitionHandler;

#[step_entity(name = "PROPERTY_DEFINITION", pass = Pass8PropertyDef)]
impl SimpleEntityHandler for PropertyDefinitionHandler {
    type WriteInput = PropertyDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
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
        // Two target patterns. Pattern A: PROPERTY_DEFINITION.definition →
        // PRODUCT_DEFINITION (resolved via pdef_to_product). Pattern B:
        // PROPERTY_DEFINITION.definition → SHAPE_ASPECT (resolved via
        // shape_aspect_id_map — populated by Pass8ShapeAspect which runs
        // before this pass). Both store an entry; PDR / GPA reuse the
        // shared property_def_map.
        let (definition, product_id) =
            if let Some(&product_step_id) = ctx.pdef_to_product.get(&target_ref) {
                let Some(&pid) = ctx.product_arena_map.get(&product_step_id) else {
                    return Ok(());
                };
                (CharacterizedDefinition::ProductDefinition(pid), pid)
            } else if let Some(&sa_id) = ctx.shape_aspect_id_map.get(&target_ref) {
                let pid = ctx.shape_aspects[sa_id].target;
                (CharacterizedDefinition::ShapeAspect(sa_id), pid)
            } else {
                eprintln!(
                    "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
                     resolves to neither PRODUCT_DEFINITION nor SHAPE_ASPECT — skipping"
                );
                return Ok(());
            };
        ctx.property_def_map
            .insert(entity_id, (name.clone(), description.clone(), product_id));
        // Schema-faithful `property_definitions` arena push (the writer's
        // sole PD emit source). `description` flattens Option → empty
        // string so the carrier struct uses raw `String`.
        let arena_description = description.unwrap_or_default();
        let pd_id = ctx
            .properties
            .get_or_insert_with(PropertyPool::default)
            .property_definitions
            .push(PropertyDefinition::Itself(PropertyDefinitionData {
                name,
                description: arena_description,
                definition,
            }));
        ctx.property_def_step_to_id.insert(entity_id, pd_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        PropertyDefinitionWriteInput {
            name,
            description,
            pdef_id,
        }: PropertyDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        let desc_attr = match description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "PROPERTY_DEFINITION",
            vec![
                Attribute::String(name),
                desc_attr,
                Attribute::EntityRef(pdef_id),
            ],
        ))
    }
}
