//! `B_SPLINE_CURVE_WITH_KNOTS` handler — 2D sister (pcurve subtree), non-rational
//! → `Curve2d::Nurbs` (2-layer path; reuses the 3D generated bind/serialize).
//!
//! The rational form lives in `rational_bspline_curve_2d.rs`; `write` asserts
//! the non-rational invariant since `emit_nurbs_curve_2d` routes rational forms
//! to the 2D rational handler.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsCurve2d;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BSplineCurve2dWithKnotsHandler;

#[step_entity(name = "B_SPLINE_CURVE_WITH_KNOTS", is_2d)]
impl SimpleEntityHandler for BSplineCurve2dWithKnotsHandler {
    type WriteInput = NurbsCurve2d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_b_spline_curve_with_knots(entity_id, attrs)?;
        lower::lower_b_spline_curve_2d_with_knots(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve2d) -> Result<u64, WriteError> {
        debug_assert!(
            nurbs.weights().is_none(),
            "BSplineCurve2dWithKnotsHandler::write expects a non-rational curve; \
             dispatch in emit_nurbs_curve_2d routes rational forms to the 2D rational handler",
        );
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            control_points.push(CartesianPoint2dHandler::write(buf, pid)?);
        }
        let early = lift::lift_b_spline_curve_2d_with_knots(&nurbs, control_points);
        Ok(serialize::serialize_b_spline_curve_with_knots(buf, &early))
    }
}
