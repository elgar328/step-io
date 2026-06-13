//! `PRESENTED_ITEM_REPRESENTATION` / `APPLIED_PRESENTED_ITEM` handlers —
//! visualization (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    AppliedPresentedItem, PresentationReprSelect, PresentedItem, PresentedItemRepresentation,
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
        let early = bind::bind_presented_item_representation(entity_id, attrs)?;
        lower::lower_presented_item_representation(ctx, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pir: PresentedItemRepresentation) -> Result<u64, WriteError> {
        let pres_step = match pir.presentation {
            PresentationReprSelect::Representation(id) => buf.step_id(id),
            PresentationReprSelect::Set(id) => buf.step_id(id),
        };
        let item_step = buf.step_id(pir.item);
        let early = lift::lift_presented_item_representation(pres_step, item_step);
        Ok(serialize::serialize_presented_item_representation(
            buf, &early,
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
        let early = bind::bind_applied_presented_item(entity_id, attrs)?;
        lower::lower_applied_presented_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, api: AppliedPresentedItem) -> Result<u64, WriteError> {
        let items: Vec<u64> = api
            .items
            .into_iter()
            .map(|it| emit_presented_item(buf, it))
            .collect();
        let early = lift::lift_applied_presented_item(items);
        Ok(serialize::serialize_applied_presented_item(buf, &early))
    }
}

fn emit_presented_item(buf: &WriteBuffer, item: PresentedItem) -> u64 {
    match item {
        PresentedItem::ProductDefinition(id) => buf.product_def_ids.get(&id).copied().unwrap_or(0),
    }
}
