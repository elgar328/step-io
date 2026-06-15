//! `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM` (CDOSI) handler.
//!
//! Extends [`super::over_riding_styled_item::OverRidingStyledItemHandler`]
//! with the additional `style_context` SELECT list. The list is resolved in
//! the [`crate::reader::ReaderContext::resolve_cdorsi_style_contexts`] post-pass
//! (assembly RR-complex targets only gain an id after the placements are
//! materialised): a `representation_item` becomes a `RepresentationItem`, an
//! assembly placement complex a `RepresentationRelationship`, and other SELECT
//! members (`shape` / `shape_aspect`) are dropped with a warning.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::ContextDependentOverRidingStyledItem;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct ContextDependentOverRidingStyledItemHandler;

#[step_entity(name = "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM")]
impl SimpleEntityHandler for ContextDependentOverRidingStyledItemHandler {
    type WriteInput = ContextDependentOverRidingStyledItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves styles /
        // item / over_ridden and defers `style_context` to the
        // `resolve_cdorsi_style_contexts` post-pass via `pending_cdorsi`.
        let early = bind::bind_context_dependent_over_riding_styled_item(entity_id, attrs)?;
        lower::lower_context_dependent_over_riding_styled_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        cd: ContextDependentOverRidingStyledItem,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_context_dependent_over_riding_styled_item(buf, &cd)?;
        Ok(serialize::serialize_context_dependent_over_riding_styled_item(buf, &early))
    }
}
