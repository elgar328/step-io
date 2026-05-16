//! `PROPERTY_DEFINITION` handler — Pass 8-2.
//!
//! Reader stores `(name, description, ProductId)` in `property_def_map`
//! keyed by STEP entity id. Pattern B (target = `SHAPE_ASPECT`) is dropped
//! at read time so only Product-targeting PDs reach the PDR pass. Writer
//! emits the bare PD line; the surrounding `REPRESENTATION` + PDR are
//! handled in `buffer/property.rs::emit_property` (the orchestrator).

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct PropertyDefinitionWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) pdef_id: u64,
}

pub(crate) struct PropertyDefinitionHandler;

impl SimpleEntityHandler for PropertyDefinitionHandler {
    const NAME: &'static str = "PROPERTY_DEFINITION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass8PropertyDef;
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
        // Resolve target via the assembly pass's pdef_to_product map. PDs
        // whose target doesn't resolve to a Product (SHAPE_ASPECT etc.) are
        // silently dropped — Pattern B per the property module docs.
        let Some(&product_step_id) = ctx.pdef_to_product.get(&target_ref) else {
            return Ok(());
        };
        let Some(&product_id) = ctx.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };
        ctx.property_def_map
            .insert(entity_id, (name, description, product_id));
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PD_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: PropertyDefinitionHandler::NAME,
    pass_level: PropertyDefinitionHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: PropertyDefinitionHandler::read,
    },
};
