//! Plain `SHAPE_REPRESENTATION` handler — 2-layer path.
//!
//! Catches the bare `SHAPE_REPRESENTATION` form used by Group products and
//! by the outer wrapper of Fusion 360 / CATIA indirect-SR chains. The dispatch
//! registry exact-matches entity names, so ABSR / MSSR never reach this handler.
//!
//! `read` = generated `bind` + hand `lower_shape_representation` (captures the
//! first `AXIS2_PLACEMENT_3D` from `items` for the SDR indirection pass and
//! dual-writes the unified `representations` arena). `write` (product-driven,
//! Group SR) resolves the coordinate frame then lift + generated serialize; the
//! arena-driven Plain arm and the indirect outer SR live in
//! `writer::buffer::assembly`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ShapeRepresentationHandler;

#[step_entity(name = "SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for ShapeRepresentationHandler {
    type WriteInput = ShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_REPRESENTATION")?;
        let early = bind::bind_shape_representation(entity_id, attrs)?;
        lower::lower_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeRepresentationWriteInput { product, unit_ctx }: ShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let early = lift::lift_shape_representation(String::new(), vec![axis_ref], unit_ctx);
        Ok(serialize::serialize_shape_representation(buf, &early))
    }
}
