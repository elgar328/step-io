//! `RATIONAL_B_SPLINE_CURVE` handler (`ComplexEntityHandler`, 2-layer path).
//!
//! `emit_nurbs_curve` (`src/writer/buffer/geometry.rs`) routes every rational
//! `Curve::Nurbs` here (non-rational stays the simple `B_SPLINE_CURVE_WITH_KNOTS`
//! form). The 2D sister `rational_bspline_curve_2d` shares the same `cases`,
//! disambiguated by the pcurve partition. The generated complex `bind`/`serialize`
//! (gen-early `[complex.rational_b_spline_curve]`) handle the multi-part wire form.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{NurbsCurve, NurbsKind};
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct RationalBsplineCurveHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_CURVE",
    cases = [[
        "BOUNDED_CURVE", "B_SPLINE_CURVE", "B_SPLINE_CURVE_WITH_KNOTS", "CURVE",
        "GEOMETRIC_REPRESENTATION_ITEM", "RATIONAL_B_SPLINE_CURVE", "REPRESENTATION_ITEM"
    ]]
)]
impl ComplexEntityHandler for RationalBsplineCurveHandler {
    type WriteInput = NurbsCurve;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rational_b_spline_curve(entity_id, parts)?;
        lower::lower_rational_bspline_curve(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        let NurbsKind::Rational { weights } = &nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineCurveHandler::write requires weights".into(),
            });
        };
        let weights = weights.clone();
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            control_points.push(buf.emit_point(pid)?);
        }
        let early = lift::lift_rational_bspline_curve(&nurbs, control_points, weights);
        Ok(serialize::serialize_rational_b_spline_curve(buf, &early))
    }
}
