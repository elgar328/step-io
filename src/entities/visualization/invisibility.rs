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

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::Invisibility;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` does the 5-bucket
        // multi-probe and the empty-SET / all-unresolved normalizations.
        let early = bind::bind_invisibility(entity_id, attrs)?;
        lower::lower_invisibility(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, inv: Invisibility) -> Result<u64, WriteError> {
        let early = lift::lift_invisibility(buf, &inv);
        Ok(serialize::serialize_invisibility(buf, &early))
    }
}
