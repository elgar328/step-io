//! `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` handler — phase
//! characterized-object-ciwr.
//!
//! Pairs a `representation_item` with the `representation` that contains
//! it. Either ref unresolved drops the occurrence.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    CharacterizedItemWithinRepresentation, CharacterizedObject, CharacterizedObjectData,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CharacterizedItemWithinRepresentationHandler;

#[step_entity(
    name = "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
    pass = Pass8CharacterizedItemWithinRepresentation
)]
impl SimpleEntityHandler for CharacterizedItemWithinRepresentationHandler {
    type WriteInput = CharacterizedItemWithinRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            4,
            entity_id,
            "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
        )?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = match attrs.get(1) {
            Some(Attribute::String(s)) => Some(s.clone()),
            _ => None,
        };
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let rep_ref = read_entity_ref(attrs, 3, entity_id, "rep")?;
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        let Some(&rep) = ctx.repr_id_map.get(&rep_ref) else {
            return Ok(());
        };
        let co_id = ctx.characterized_objects.push(
            CharacterizedObject::CharacterizedItemWithinRepresentation(
                CharacterizedItemWithinRepresentation {
                    inherited: CharacterizedObjectData { name, description },
                    item,
                    rep,
                },
            ),
        );
        // Let PROPERTY_DEFINITION.definition resolve a CIWR target.
        ctx.characterized_object_id_map.insert(entity_id, co_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ciwr: CharacterizedItemWithinRepresentation,
    ) -> Result<u64, WriteError> {
        let item_step = buf.emit_representation_item_ref(ciwr.item)?;
        let rep_step = buf.representation_step_ids[ciwr.rep.0 as usize];
        let desc_attr = match ciwr.inherited.description {
            Some(d) => Attribute::String(d),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
            vec![
                Attribute::String(ciwr.inherited.name),
                desc_attr,
                Attribute::EntityRef(item_step),
                Attribute::EntityRef(rep_step),
            ],
        ))
    }
}
