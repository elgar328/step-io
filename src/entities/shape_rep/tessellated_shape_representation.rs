//! `TESSELLATED_SHAPE_REPRESENTATION` handler — phase tsr.
//!
//! `shape_representation` SUBTYPE whose `items` SET is narrowed to
//! `tessellated_item`. EXPRESS WHERE rule requires `context_of_items`
//! to be a `GLOBAL_UNIT_ASSIGNED_CONTEXT`; non-unitful contexts drop
//! the carrier (symmetric on re-read). Items unresolved by
//! `resolve_tessellated_item_ref` are skipped from the set; an empty
//! resolved set drops the carrier.
//!
//! Emit is delayed (`Mdgpr` / `DraughtingModel` pattern) —
//! `emit_representations_pre_pass` skips this variant, and
//! `emit_tessellated_shape_representations` runs after
//! `emit_tessellation` so the `tessellated_*_step_ids` caches are
//! fully populated before `items` refs resolve.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::TessellatedShapeRepresentation;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TessellatedShapeRepresentationHandler;

#[step_entity(name = "TESSELLATED_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for TessellatedShapeRepresentationHandler {
    type WriteInput = TessellatedShapeRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tessellated_shape_representation(entity_id, attrs)?;
        lower::lower_tessellated_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        tsr: TessellatedShapeRepresentation,
    ) -> Result<u64, WriteError> {
        Ok(serialize::serialize_tessellated_shape_representation(
            buf,
            &lift::lift_tessellated_shape_representation(buf, tsr),
        ))
    }
}
