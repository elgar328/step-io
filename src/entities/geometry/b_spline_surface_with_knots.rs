//! `B_SPLINE_SURFACE_WITH_KNOTS` handler — leaf NURBS surface (non-rational;
//! 2-layer path). Rational form lives in `rational_bspline_surface.rs`. Surface
//! control points are always 3D, so (unlike the curve) there is no 2D sister.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsSurface;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BSplineSurfaceWithKnotsHandler;

#[step_entity(name = "B_SPLINE_SURFACE_WITH_KNOTS")]
impl SimpleEntityHandler for BSplineSurfaceWithKnotsHandler {
    type WriteInput = NurbsSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_b_spline_surface_with_knots(entity_id, attrs)?;
        lower::lower_b_spline_surface_with_knots(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        debug_assert!(
            nurbs.weights().is_none(),
            "BSplineSurfaceWithKnotsHandler::write expects a non-rational surface"
        );
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for row in &nurbs.control_points {
            let mut pt_row = Vec::with_capacity(row.len());
            for &pid in row {
                pt_row.push(buf.emit_point(pid)?);
            }
            control_points.push(pt_row);
        }
        let early = lift::lift_b_spline_surface_with_knots(&nurbs, control_points);
        Ok(serialize::serialize_b_spline_surface_with_knots(
            buf, &early,
        ))
    }
}
