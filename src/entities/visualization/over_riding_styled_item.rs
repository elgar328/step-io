//! `OVER_RIDING_STYLED_ITEM` handler.
//!
//! Subtype of `STYLED_ITEM` whose `over_ridden_style` ref points at
//! another styled item whose styles this entity replaces. Topo order
//! processes the referenced `STYLED_ITEM` first, so the over-ridden ref
//! resolves to an existing `StyledItemId` in `VisualizationPool::styled_items`. Targets that
//! don't resolve, missing over-ridden refs, or unsupported
//! `representation_item` variants are silently dropped to preserve
//! round-trip equality on the supported subset.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::visualization::OverRidingStyledItem;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

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
        // NsCase::OrsiOverRiddenUnset: `over_ridden_style` is mandatory in EXPRESS
        // but some exporters emit `$`. The generated bind is strict (errors on a
        // `$` required ref), so normalize that case here (NsCase-recorded for
        // NORM) by dropping before bind — matches the previous read_optional path.
        if matches!(attrs.get(3), Some(Attribute::Unset | Attribute::Derived)) {
            ctx.ns_record(
                crate::reader::NsCase::OrsiOverRiddenUnset,
                "OVER_RIDING_STYLED_ITEM".into(),
                "dropped (over_ridden_style Unset — no over-ridden style)",
            );
            return Ok(());
        }
        let early = bind::bind_over_riding_styled_item(entity_id, attrs)?;
        lower::lower_over_riding_styled_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, osi: OverRidingStyledItem) -> Result<u64, WriteError> {
        let early = lift::lift_over_riding_styled_item(buf, &osi)?;
        Ok(serialize::serialize_over_riding_styled_item(buf, &early))
    }
}
