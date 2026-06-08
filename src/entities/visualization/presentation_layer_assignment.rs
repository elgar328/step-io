//! `PRESENTATION_LAYER_ASSIGNMENT` handler. Top-level
//! grouping of `STYLED_ITEM` entries into a named display layer
//! ("VISIBLE", "HIDDEN", numeric ids, ...). Topo order processes the
//! referenced styled items first, so the `assigned_items` SELECT refs
//! resolve to existing `StyledItemId` arena entries. Other `layered_item` SELECT variants
//! (direct geometry refs, etc.) are silently dropped on read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationLayerAssignment, PresentationLayerAssignmentItem, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationLayerAssignmentHandler;

#[step_entity(name = "PRESENTATION_LAYER_ASSIGNMENT")]
impl SimpleEntityHandler for PresentationLayerAssignmentHandler {
    type WriteInput = PresentationLayerAssignment;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PRESENTATION_LAYER_ASSIGNMENT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 2, entity_id, "assigned_items")?;
        let mut assigned_items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(sid) = ctx.id_cache.get::<crate::ir::id::StyledItemId>(r) {
                assigned_items.push(PresentationLayerAssignmentItem::StyledItem(sid));
            }
        }
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .presentation_layer_assignments
            .push(PresentationLayerAssignment {
                name,
                description,
                assigned_items,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pla: PresentationLayerAssignment) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(pla.assigned_items.len());
        for item in pla.assigned_items {
            let step_id = match item {
                PresentationLayerAssignmentItem::StyledItem(sid) => buf.step_id(sid),
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "PRESENTATION_LAYER_ASSIGNMENT",
            vec![
                Attribute::String(pla.name),
                Attribute::String(pla.description),
                Attribute::List(item_refs),
            ],
        ))
    }
}
