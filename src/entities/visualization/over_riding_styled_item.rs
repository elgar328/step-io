//! `OVER_RIDING_STYLED_ITEM` handler.
//!
//! Subtype of `STYLED_ITEM` whose `over_ridden_style` ref points at
//! another styled item whose styles this entity replaces. Topo order
//! processes the referenced `STYLED_ITEM` first, so the over-ridden ref
//! resolves to an existing `StyledItemId` in `VisualizationPool::styled_items`. Targets that
//! don't resolve, missing over-ridden refs, or unsupported
//! `representation_item` variants are silently dropped to preserve
//! round-trip equality on the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_entity_ref, read_entity_ref_list, read_optional_entity_ref,
    read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{OverRidingStyledItem, StyledItem, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::styled_item::resolve_representation_item_ref;
use step_io_macros::step_entity;

pub(crate) struct OverRidingStyledItemHandler;

#[step_entity(name = "OVER_RIDING_STYLED_ITEM")]
impl SimpleEntityHandler for OverRidingStyledItemHandler {
    type WriteInput = OverRidingStyledItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "OVER_RIDING_STYLED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        // [NS-orsi-over-ridden-unset] `over_ridden_style` is mandatory
        // (EXPRESS `over_riding_styled_item.over_ridden_style : styled_item`);
        // some exporters emit `$`. Without an over-ridden target the entity
        // carries no information → drop as a normalization, not a defect.
        let Some(over_ridden_ref) =
            read_optional_entity_ref(attrs, 3, entity_id, "over_ridden_style")?
        else {
            // field must be the exact type name so reference-check's
            // count-aware effective-missing deduction matches the dropped type.
            ctx.record_nonstandard(
                "OVER_RIDING_STYLED_ITEM".into(),
                "dropped (over_ridden_style Unset — no over-ridden style)",
            );
            return Ok(());
        };

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "OVER_RIDING_STYLED_ITEM target #{item_ref} did not resolve to a modelled \
                     representation_item kind (likely cascade from a dropped dependency)"
                ),
            });
            return Ok(());
        };
        let Some(&over_ridden_style) = ctx.viz_styled_item_id_map.get(&over_ridden_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "OVER_RIDING_STYLED_ITEM over_ridden_style #{over_ridden_ref} did not resolve \
                     to a previously-loaded STYLED_ITEM"
                ),
            });
            return Ok(());
        };

        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .styled_items
            .push(StyledItem::OverRiding(OverRidingStyledItem {
                name,
                styles,
                item,
                over_ridden_style,
            }));
        ctx.viz_styled_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, osi: OverRidingStyledItem) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(osi.item)?;
        let mut style_refs = Vec::with_capacity(osi.styles.len());
        for psa_id in osi.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
        }
        let over_ridden_step_id = buf.styled_item_step_ids[osi.over_ridden_style.0 as usize];
        Ok(buf.push_simple(
            "OVER_RIDING_STYLED_ITEM",
            vec![
                Attribute::String(osi.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::EntityRef(over_ridden_step_id),
            ],
        ))
    }
}
