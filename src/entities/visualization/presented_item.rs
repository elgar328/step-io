//! `PRESENTED_ITEM_REPRESENTATION` + `APPLIED_PRESENTED_ITEM` handlers
//! — phase pr-item.
//!
//! `presented_item` SELECT has 9 members; step-io models only
//! `product_definition` (`ProductId` via the product chain). Other 8 PLM
//! members are silently skipped on read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    AppliedPresentedItem, PresentationReprSelect, PresentedItem, PresentedItemRepresentation,
    VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentedItemRepresentationHandler;

#[step_entity(name = "PRESENTED_ITEM_REPRESENTATION")]
impl SimpleEntityHandler for PresentedItemRepresentationHandler {
    type WriteInput = PresentedItemRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRESENTED_ITEM_REPRESENTATION")?;
        let pres_ref = read_entity_ref(attrs, 0, entity_id, "presentation")?;
        let presentation = if let Some(&id) = ctx.presentation_representation_id_map.get(&pres_ref)
        {
            PresentationReprSelect::Representation(id)
        } else if let Some(&id) = ctx.presentation_set_id_map.get(&pres_ref) {
            PresentationReprSelect::Set(id)
        } else {
            return Ok(());
        };
        let item_ref = read_entity_ref(attrs, 1, entity_id, "item")?;
        // `item` is a `presented_item` (abstract; concrete subtype
        // `applied_presented_item`), so it resolves through the
        // applied-presented-item arena, not a `presented_item_select` member.
        let Some(&item) = ctx.applied_presented_item_id_map.get(&item_ref) else {
            return Ok(());
        };
        let _id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .presented_item_representations
            .push(PresentedItemRepresentation { presentation, item });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pir: PresentedItemRepresentation) -> Result<u64, WriteError> {
        let pres_step = match pir.presentation {
            PresentationReprSelect::Representation(id) => buf.step_id(id),
            PresentationReprSelect::Set(id) => buf.step_id(id),
        };
        let item_step = buf.step_id(pir.item);
        Ok(buf.push_simple(
            "PRESENTED_ITEM_REPRESENTATION",
            vec![
                Attribute::EntityRef(pres_step),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}

pub(crate) struct AppliedPresentedItemHandler;

#[step_entity(name = "APPLIED_PRESENTED_ITEM")]
impl SimpleEntityHandler for AppliedPresentedItemHandler {
    type WriteInput = AppliedPresentedItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "APPLIED_PRESENTED_ITEM")?;
        let refs = read_entity_ref_list(attrs, 0, entity_id, "items")?;
        let mut items = Vec::with_capacity(refs.len());
        for r in refs {
            if let Some(item) = resolve_presented_item(ctx, r) {
                items.push(item);
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .applied_presented_items
            .push(AppliedPresentedItem { items });
        ctx.applied_presented_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, api: AppliedPresentedItem) -> Result<u64, WriteError> {
        let items = api
            .items
            .into_iter()
            .map(|it| Attribute::EntityRef(emit_presented_item(buf, it)))
            .collect();
        Ok(buf.push_simple("APPLIED_PRESENTED_ITEM", vec![Attribute::List(items)]))
    }
}

fn resolve_presented_item(ctx: &ReaderContext, step_id: u64) -> Option<PresentedItem> {
    // `presented_item` SELECT — only `product_definition` is modelled.
    // Other 8 PLM members (action / product_concept / ...) are skipped.
    ctx.resolve_product_by_pdef(0, step_id, "presented_item")
        .ok()
        .map(PresentedItem::ProductDefinition)
}

fn emit_presented_item(buf: &WriteBuffer, item: PresentedItem) -> u64 {
    match item {
        PresentedItem::ProductDefinition(id) => buf.product_def_ids.get(&id).copied().unwrap_or(0),
    }
}
