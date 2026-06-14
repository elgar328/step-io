//! `B_SPLINE_CURVE_WITH_KNOTS` handler — leaf NURBS curve (non-rational; 2-layer
//! path). Rational form lives in `rational_bspline_curve.rs`; the 2D sister in
//! `b_spline_curve_2d_with_knots.rs`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsCurve;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BSplineCurveWithKnotsHandler;

#[step_entity(name = "B_SPLINE_CURVE_WITH_KNOTS")]
impl SimpleEntityHandler for BSplineCurveWithKnotsHandler {
    type WriteInput = NurbsCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_b_spline_curve_with_knots(entity_id, attrs)?;
        lower::lower_b_spline_curve_with_knots(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        debug_assert!(
            nurbs.weights().is_none(),
            "BSplineCurveWithKnotsHandler::write expects a non-rational curve"
        );
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            control_points.push(buf.emit_point(pid)?);
        }
        let early = lift::lift_b_spline_curve_with_knots(&nurbs, control_points);
        Ok(serialize::serialize_b_spline_curve_with_knots(buf, &early))
    }
}
