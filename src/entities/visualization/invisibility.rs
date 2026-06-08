//! `INVISIBILITY` handler — phase invisibility (delayed emit).
//!
//! Reads the SET of `invisible_item` refs and classifies each as one of the
//! four schema members: `StyledItem` / `Representation` / `DraughtingCallout`
//! / `PresentationLayerAssignment`. Refs to dropped target instances are
//! skipped per-item.
//!
//! Emit is scheduled in `emit_pools` after
//! `emit_draughting_callouts` / `emit_draughting_models`, so every item
//! ref cache is populated before this handler indexes them. Inline emit
//! inside `emit_visualization_if_set` would panic on corpora whose
//! invisibility entries reference a `DRAUGHTING_CALLOUT` (the cache is
//! filled later in the pipeline).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Invisibility, InvisibleItem, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct InvisibilityHandler;

#[step_entity(name = "INVISIBILITY")]
impl SimpleEntityHandler for InvisibilityHandler {
    type WriteInput = Invisibility;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "INVISIBILITY")?;
        let refs = read_entity_ref_list(attrs, 0, entity_id, "invisible_items")?;
        // [NS-empty-invisibility] grabcad: invisible_items is SET[1:?] but an
        // empty `()` violates the cardinality (and hides nothing) → drop as a
        // normalization (leaf, no cascade). See reader::nonstandard.
        if refs.is_empty() {
            ctx.warnings.push(ConvertError::NonStandardInput {
                field: "INVISIBILITY".into(),
                count: 1,
                normalized_to: "dropped (empty invisible_items, non-standard SET[1:?])".into(),
            });
            return Ok(());
        }
        let mut invisible_items = Vec::with_capacity(refs.len());
        for r in refs {
            if let Some(id) = ctx.id_cache.get::<crate::ir::id::StyledItemId>(r) {
                invisible_items.push(InvisibleItem::StyledItem(id));
            } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::AnnotationOccurrenceId>(r) {
                // annotation_occurrence is an EXPRESS styled_item subtype but
                // lives in a separate step-io arena — probe it alongside
                // viz_styled_item.
                invisible_items.push(InvisibleItem::AnnotationOccurrence(id));
            } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::RepresentationId>(r) {
                invisible_items.push(InvisibleItem::Representation(id));
            } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::DraughtingCalloutId>(r) {
                invisible_items.push(InvisibleItem::DraughtingCallout(id));
            } else if let Some(id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationLayerAssignmentId>(r)
            {
                invisible_items.push(InvisibleItem::PresentationLayerAssignment(id));
            }
            // Dropped item instances are skipped per-item — the entity survives
            // on its resolved items.
        }
        if invisible_items.is_empty() {
            // The set was non-empty but nothing resolved (all unmodelled SELECT
            // members, or every target instance was dropped). Surface the gap
            // instead of dropping silently — these entities already count as
            // missing, so this adds visibility, not loss.
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from(
                    "INVISIBILITY invisible_items did not resolve to any modelled \
                     styled_item / annotation_occurrence / representation / \
                     draughting_callout / presentation_layer_assignment — dropping",
                ),
            });
            return Ok(());
        }
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .invisibilities
            .push(Invisibility {
                invisible_items,
                presentation_context: None,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, inv: Invisibility) -> Result<u64, WriteError> {
        let items = inv
            .invisible_items
            .into_iter()
            .map(|item| match item {
                InvisibleItem::AnnotationOccurrence(id) => Attribute::EntityRef(buf.step_id(id)),
                InvisibleItem::StyledItem(id) => Attribute::EntityRef(buf.step_id(id)),
                InvisibleItem::Representation(id) => Attribute::EntityRef(buf.step_id(id)),
                InvisibleItem::DraughtingCallout(id) => Attribute::EntityRef(buf.step_id(id)),
                InvisibleItem::PresentationLayerAssignment(id) => {
                    Attribute::EntityRef(buf.step_id(id))
                }
            })
            .collect();
        Ok(buf.push_simple("INVISIBILITY", vec![Attribute::List(items)]))
    }
}
