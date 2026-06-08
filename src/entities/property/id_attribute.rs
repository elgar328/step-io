//! `ID_ATTRIBUTE` handler.
//!
//! `(attribute_value, identified_item)` — SELECT target. Coverage: the full
//! `shape_aspect` family via `resolve_shape_aspect_ref` (plain `shape_aspect`
//! plus every subtype `ShapeAspectRef` models — `datum*`, `datum_system`,
//! `datum_target`, `placed_datum_target_feature`, `tolerance_zone`,
//! `general_datum_reference`/`datum_reference_compartment`), `property_definition`,
//! and the plm-metadata targets (`Group` / `Address` / `ApplicationContext`).
//! SELECT members outside that set are dropped at read time with a warning.

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::ir::PropertyPool;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::property::{IdAttribute, IdAttributeItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct IdAttributeWriteInput {
    pub(crate) attr: IdAttribute,
    pub(crate) item_step: u64,
}

pub(crate) struct IdAttributeHandler;

#[step_entity(name = "ID_ATTRIBUTE")]
impl SimpleEntityHandler for IdAttributeHandler {
    type WriteInput = IdAttributeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "ID_ATTRIBUTE")?;
        let attribute_value =
            read_string_or_unset(attrs, 0, entity_id, "attribute_value")?.to_owned();
        let item_ref = read_entity_ref(attrs, 1, entity_id, "identified_item")?;

        // `identified_item` is a SELECT; its `shape_aspect` member covers every
        // shape_aspect subtype (plain SHAPE_ASPECT, DATUM*, TOLERANCE_ZONE,
        // GENERAL_DATUM_REFERENCE, ...), unified through `ShapeAspectRef`. Each
        // subtype round-trips through its own arena and is verified by the
        // merkle round-trip gate.
        let identified_item = if let Some(sa_ref) = resolve_shape_aspect_ref(ctx, item_ref) {
            IdAttributeItem::ShapeAspect(sa_ref)
        } else if let Some(&pd_id) = ctx.property_def_step_to_id.get(&item_ref) {
            IdAttributeItem::PropertyDefinition(pd_id)
        } else if let Some(&g_id) = ctx.plm_group_id_map.get(&item_ref) {
            IdAttributeItem::Group(g_id)
        } else if let Some(&a_id) = ctx.plm_address_id_map.get(&item_ref) {
            IdAttributeItem::Address(a_id)
        } else if let Some(&ac_id) = ctx.plm_application_context_id_map.get(&item_ref) {
            IdAttributeItem::ApplicationContext(ac_id)
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("ID_ATTRIBUTE.identified_item #{item_ref} target type unsupported"),
            });
            return Ok(());
        };

        let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
        pool.id_attributes.push(IdAttribute {
            attribute_value,
            identified_item,
        });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        IdAttributeWriteInput { attr, item_step }: IdAttributeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "ID_ATTRIBUTE",
            vec![
                Attribute::String(attr.attribute_value),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}
