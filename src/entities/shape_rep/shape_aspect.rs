//! `SHAPE_ASPECT` handler.
//!
//! Resolves `SHAPE_ASPECT.of_shape` to a `ProductId` through the typed
//! `product_of_pds` probe (recorded by the PDS lower).
//! Future PMI work (Tolerance / Datum / GD&T per ROADMAP Phase 2) hangs
//! additional handlers off the same group.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeAspectWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

pub(crate) struct ShapeAspectHandler;

#[step_entity(name = "SHAPE_ASPECT")]
impl SimpleEntityHandler for ShapeAspectHandler {
    type WriteInput = ShapeAspectWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves of_shape
        // (standard PDS or the non-standard PD form, surfacing NsCase) and
        // registers the `ShapeAspectId` key consumers probe.
        let early = bind::bind_shape_aspect(entity_id, attrs)?;
        lower::lower_shape_aspect(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeAspectWriteInput {
            name,
            description,
            pds_step_id,
            product_definitional,
        }: ShapeAspectWriteInput,
    ) -> Result<u64, WriteError> {
        // 2-layer write path: lift the (pre-resolved) write fields → L1, then
        // serialize. The emit loop already resolved ProductId → pds_step_id.
        let early = lift::lift_shape_aspect(name, description, pds_step_id, product_definitional);
        Ok(serialize::serialize_shape_aspect(buf, &early))
    }
}
