//! `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` handler — 2-layer path.
//!
//! `read` = generated `bind` + hand `lower_manifold_surface_shape_representation`
//! (flattens each item's SBSM shells into `mssr_shells_map`, keeps the SBSM GRI
//! ids for arena routing, and captures the first `AXIS2_PLACEMENT_3D` frame;
//! dual-writes the unified `representations` arena). `write` (product-driven)
//! emits a per-product axis placement + an SBSM wrapping all shells, then lift +
//! generated serialize; the arena-driven `ManifoldSurface` arm lives in
//! `writer::buffer::assembly`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;
use step_io_macros::step_entity;

pub(crate) struct ManifoldSurfaceShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) shells: Vec<ShellId>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ManifoldSurfaceShapeRepresentationHandler;

#[step_entity(name = "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for ManifoldSurfaceShapeRepresentationHandler {
    type WriteInput = ManifoldSurfaceShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")?;
        let early = bind::bind_manifold_surface_shape_representation(entity_id, attrs)?;
        lower::lower_manifold_surface_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ManifoldSurfaceShapeRepresentationWriteInput {
            product,
            shells,
            unit_ctx,
        }: ManifoldSurfaceShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let sbsm_ref = ShellBasedSurfaceModelHandler::write(buf, shells)?;
        let early = lift::lift_manifold_surface_shape_representation(
            String::new(),
            vec![axis_ref, sbsm_ref],
            unit_ctx,
        );
        Ok(serialize::serialize_manifold_surface_shape_representation(
            buf, &early,
        ))
    }
}
