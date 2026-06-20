//! `ADVANCED_BREP_SHAPE_REPRESENTATION` handler — 2-layer path.
//!
//! `read` = generated `bind` + hand `lower_advanced_brep_shape_representation`
//! (resolves every `items` ref into a typed `RepresentationItemRef` — solids +
//! an axis frame, or `MAPPED_ITEM`s for an assembly ABSR — preserving source
//! order, and derives the legacy `absr_solid_map` / `absr_ref_frame_map` side
//! maps; dual-writes the unified `representations` arena). `write`
//! (product-driven) resolves the frame + solid refs then lift + generated
//! serialize; the arena-driven `AdvancedBrep` arm and the deferred assembly ABSR
//! (reserve-then-fill) live in `writer::buffer::assembly`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AdvancedBrepShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) solid_refs: Vec<u64>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct AdvancedBrepShapeRepresentationHandler;

#[step_entity(name = "ADVANCED_BREP_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for AdvancedBrepShapeRepresentationHandler {
    type WriteInput = AdvancedBrepShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        let early = bind::bind_advanced_brep_shape_representation(entity_id, attrs)?;
        lower::lower_advanced_brep_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        AdvancedBrepShapeRepresentationWriteInput {
            product,
            solid_refs,
            unit_ctx,
        }: AdvancedBrepShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let mut items = Vec::with_capacity(1 + solid_refs.len());
        items.push(axis_ref);
        items.extend(solid_refs);
        let early = lift::lift_advanced_brep_shape_representation(String::new(), items, unit_ctx);
        Ok(serialize::serialize_advanced_brep_shape_representation(
            buf, &early,
        ))
    }
}
