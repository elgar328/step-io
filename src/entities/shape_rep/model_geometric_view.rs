//! `MODEL_GEOMETRIC_VIEW` handler — phase model-geometric-view.
//!
//! `SUBTYPE OF (characterized_item_within_representation)` narrowing
//! `item` to a `CAMERA_MODEL` and `rep` to a `DRAUGHTING_MODEL`
//! (`MODEL_GEOMETRIC_VIEW(name, description, item, rep)` — a saved view).
//! Mirrors the sibling `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` handler;
//! either ref unresolved drops the occurrence.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{CharacterizedObject, CharacterizedObjectData, ModelGeometricView};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ModelGeometricViewHandler;

#[step_entity(name = "MODEL_GEOMETRIC_VIEW")]
impl SimpleEntityHandler for ModelGeometricViewHandler {
    type WriteInput = ModelGeometricView;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "MODEL_GEOMETRIC_VIEW")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = match attrs.get(1) {
            Some(Attribute::String(s)) => Some(s.clone()),
            _ => None,
        };
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let rep_ref = read_entity_ref(attrs, 3, entity_id, "rep")?;
        let Some(&item) = ctx.viz_camera_model_id_map.get(&item_ref) else {
            return Ok(());
        };
        let Some(&rep) = ctx.repr_id_map.get(&rep_ref) else {
            return Ok(());
        };
        let co_id = ctx
            .characterized_objects
            .push(CharacterizedObject::ModelGeometricView(
                ModelGeometricView {
                    inherited: CharacterizedObjectData { name, description },
                    item,
                    rep,
                },
            ));
        // Let PROPERTY_DEFINITION.definition resolve an MGV target (mirrors CIWR).
        ctx.characterized_object_id_map.insert(entity_id, co_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mgv: ModelGeometricView) -> Result<u64, WriteError> {
        // Standalone arena emit goes through `emit_characterized_objects` (under a
        // reserved id); this trait method mirrors that body for completeness.
        let item_step = buf.step_id(mgv.item);
        let rep_step = buf.step_id(mgv.rep);
        let desc_attr = match mgv.inherited.description {
            Some(d) => Attribute::String(d),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "MODEL_GEOMETRIC_VIEW",
            vec![
                Attribute::String(mgv.inherited.name),
                desc_attr,
                Attribute::EntityRef(item_step),
                Attribute::EntityRef(rep_step),
            ],
        ))
    }
}
