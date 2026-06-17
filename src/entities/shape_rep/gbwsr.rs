//! `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` handler — 2-layer path.
//!
//! Hosts the shared writer helper that the GBSSR sister imports. Both wrappers
//! share the same `(name, items, context)` shape: the items list contains an
//! axis placement (often omitted by CATIA in the SURFACE flavour) plus one or
//! more `GEOMETRIC_(CURVE_)SET`s. `read` = generated `bind` + the hand
//! `lower_geometrically_bounded_{wireframe,surface}_shape_representation`
//! (collapses the curve sets into a single `WireframeContent`, stamps the
//! `repr_kind`, dual-writes the arena). `write` resolves the axis + inner curve
//! set via `wireframe_write_items`, then each handler lifts + serializes its own
//! entity (so the original wrapper name round-trips).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{Product, WireframeContent};
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::geometry::geometric_curve_set::{
    CurveSetWriteInput, GeometricCurveSetHandler,
};
use crate::entities::geometry::geometric_set::GeometricSetHandler;
use step_io_macros::step_entity;

pub(crate) struct WireframeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) wireframe: WireframeContent,
    pub(crate) unit_ctx: u64,
}

/// Shared writer helper for GBWSR and GBSSR: emit the per-product axis
/// placement and the inner curve set (dispatched through
/// `GeometricCurveSetHandler` / `GeometricSetHandler` based on whether loose
/// points coexist with curves), returning the resolved `items` step ids plus
/// the unit context. Each handler lifts + serializes its own entity name.
pub(crate) fn wireframe_write_items(
    buf: &mut WriteBuffer,
    input: WireframeRepresentationWriteInput,
) -> Result<(Vec<u64>, u64), WriteError> {
    let WireframeRepresentationWriteInput {
        product,
        wireframe,
        unit_ctx,
    } = input;
    let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
    let set_input = CurveSetWriteInput {
        curves: wireframe.curves.clone(),
        points: wireframe.points.clone(),
    };
    let set_ref = if wireframe.points.is_empty() {
        GeometricCurveSetHandler::write(buf, set_input)?
    } else {
        GeometricSetHandler::write(buf, set_input)?
    };
    Ok((vec![axis_ref, set_ref], unit_ctx))
}

pub(crate) struct GbwsrHandler;

#[step_entity(name = "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for GbwsrHandler {
    type WriteInput = WireframeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION",
        )?;
        let early =
            bind::bind_geometrically_bounded_wireframe_shape_representation(entity_id, attrs)?;
        lower::lower_geometrically_bounded_wireframe_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: WireframeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let (items, unit_ctx) = wireframe_write_items(buf, input)?;
        let early = lift::lift_geometrically_bounded_wireframe_shape_representation(
            String::new(),
            items,
            unit_ctx,
        );
        Ok(serialize::serialize_geometrically_bounded_wireframe_shape_representation(buf, &early))
    }
}
