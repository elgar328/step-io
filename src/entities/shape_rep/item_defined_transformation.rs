//! `ITEM_DEFINED_TRANSFORMATION` handler (2-layer path).
//!
//! Reader resolves source / target placements and stores the resulting
//! `Transform3d` keyed by entity id in `transform_map`; assembly consumers
//! (NAUO / CDSR) fetch it there. Writer emits an IDT line with the per-instance
//! source / target axis placements (`name` / `description` are not modelled by
//! `Transform3d`, so they re-emit as `''`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Transform3d;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ItemDefinedTransformationHandler;

#[step_entity(name = "ITEM_DEFINED_TRANSFORMATION")]
impl SimpleEntityHandler for ItemDefinedTransformationHandler {
    type WriteInput = Transform3d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_item_defined_transformation(entity_id, attrs)?;
        lower::lower_item_defined_transformation(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, transform: Transform3d) -> Result<u64, WriteError> {
        let source = buf.emit_axis2_placement_3d(transform.source)?;
        let target = buf.emit_axis2_placement_3d(transform.target)?;
        let early = lift::lift_item_defined_transformation(source, target);
        Ok(serialize::serialize_item_defined_transformation(
            buf, &early,
        ))
    }
}
