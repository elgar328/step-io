//! `POINT_STYLE` handler — phase point-style.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PointStyle;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PointStyleHandler;

#[step_entity(name = "POINT_STYLE")]
impl SimpleEntityHandler for PointStyleHandler {
    type WriteInput = PointStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves
        // marker/size/colour refs via the id_cache and registers the typed
        // `EarlyPointStyleId` key.
        if let Some(early) = bind::bind_point_style(entity_id, attrs)? {
            lower::lower_point_style(ctx, entity_id, early);
        } else {
            // NsCase::NonStandardEnumValue — bind returned None because a present
            // marker/size SELECT member carried a non-standard value (e.g. a
            // marker token outside the EXPRESS enum). Rejecting it is correct →
            // drop + NORM, not a silent loss.
            ctx.ns_push(
                crate::reader::NsCase::NonStandardEnumValue,
                "POINT_STYLE".into(),
                1,
                "dropped (non-standard marker/size value)".into(),
            );
            ctx.nonstandard_dropped_refs.insert(entity_id);
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ps: PointStyle) -> Result<u64, WriteError> {
        // 2-layer write path: lift L2 → L1, then serialize L1 → Part21 text.
        let early = lift::lift_point_style(buf, &ps);
        Ok(serialize::serialize_point_style(buf, &early))
    }
}
