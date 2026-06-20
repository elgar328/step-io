//! Rational `QUASI_UNIFORM_CURVE` complex (`ComplexEntityHandler`, 2-layer path).
//! A synthetic-name complex (no single early.toml entity): the parts drop
//! `B_SPLINE_CURVE_WITH_KNOTS` and add `QUASI_UNIFORM_CURVE`; knots are derived
//! in `lower`. `read_only` — `emit_nurbs_curve` normalizes every rational
//! `Curve::Nurbs` to the `RATIONAL_B_SPLINE_CURVE`-with-knots form, so this
//! handler's `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsCurve;
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct RationalQuasiUniformCurveHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_CURVE",
    cases = [[
        "BOUNDED_CURVE", "B_SPLINE_CURVE", "CURVE", "GEOMETRIC_REPRESENTATION_ITEM",
        "QUASI_UNIFORM_CURVE", "RATIONAL_B_SPLINE_CURVE", "REPRESENTATION_ITEM"
    ]]
)]
impl ComplexEntityHandler for RationalQuasiUniformCurveHandler {
    type WriteInput = NurbsCurve;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rational_quasi_uniform_curve(entity_id, parts)?;
        lower::lower_rational_quasi_uniform_curve(ctx, entity_id, &early)
    }

    fn write(_buf: &mut WriteBuffer, _nurbs: NurbsCurve) -> Result<u64, WriteError> {
        // read_only: emit_nurbs_curve normalizes rational Curve::Nurbs to the
        // RATIONAL_B_SPLINE_CURVE-with-knots form, so this is never reached.
        unreachable!(
            "rational QUASI_UNIFORM_CURVE is never emitted; emit_nurbs_curve normalizes to RATIONAL_B_SPLINE_CURVE"
        )
    }
}
