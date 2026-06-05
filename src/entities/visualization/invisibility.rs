//! `INVISIBILITY` handler — phase invisibility (delayed emit).
//!
//! Reads the SET of `invisible_item` refs and classifies each as
//! `StyledItem` / `Representation` / `DraughtingCallout`. Source refs to
//! other SELECT members (e.g. `presentation_layer_assignment`) are
//! silently skipped — step-io has no `id_map` for that target today.
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
            if let Some(&id) = ctx.viz_styled_item_id_map.get(&r) {
                invisible_items.push(InvisibleItem::StyledItem(id));
            } else if let Some(&id) = ctx.repr_id_map.get(&r) {
                invisible_items.push(InvisibleItem::Representation(id));
            } else if let Some(&id) = ctx.draughting_callout_id_map.get(&r) {
                invisible_items.push(InvisibleItem::DraughtingCallout(id));
            }
            // Other SELECT members (e.g. presentation_layer_assignment) and
            // dropped item instances: skipped per-item — the entity survives on
            // its resolved items.
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
                     styled_item / representation / draughting_callout — dropping",
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
        ctx.invisibility_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, inv: Invisibility) -> Result<u64, WriteError> {
        let items = inv
            .invisible_items
            .into_iter()
            .map(|item| match item {
                InvisibleItem::StyledItem(id) => {
                    Attribute::EntityRef(buf.styled_item_step_ids[id.0 as usize])
                }
                InvisibleItem::Representation(id) => {
                    Attribute::EntityRef(buf.representation_step_ids[id.0 as usize])
                }
                InvisibleItem::DraughtingCallout(id) => {
                    Attribute::EntityRef(buf.draughting_callout_step_ids[id.0 as usize])
                }
            })
            .collect();
        Ok(buf.push_simple("INVISIBILITY", vec![Attribute::List(items)]))
    }
}
