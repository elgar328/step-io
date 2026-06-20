//! `QUASI_UNIFORM_CURVE` handler — simple, non-rational B-spline with knots
//! derived from `(degree, cp_count)` (STEP `QuasiUniformKnots` rule). 2-layer
//! read path (`bind`→`lower`, the derivation lives in `lower`). The form is
//! `read_only`: `emit_nurbs_curve` normalizes every `Curve::Nurbs` to
//! `B_SPLINE_CURVE_WITH_KNOTS`, so this handler's `write` is never reached.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::NurbsCurve;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct QuasiUniformCurveHandler;

#[step_entity(name = "QUASI_UNIFORM_CURVE")]
impl SimpleEntityHandler for QuasiUniformCurveHandler {
    type WriteInput = NurbsCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_quasi_uniform_curve(entity_id, attrs)?;
        lower::lower_quasi_uniform_curve(ctx, entity_id, &early)
    }

    fn write(_buf: &mut WriteBuffer, _nurbs: NurbsCurve) -> Result<u64, WriteError> {
        // `read_only`: `emit_nurbs_curve` always normalizes `Curve::Nurbs` to
        // `B_SPLINE_CURVE_WITH_KNOTS`, so QUASI_UNIFORM_CURVE is never emitted.
        unreachable!(
            "QUASI_UNIFORM_CURVE is never emitted; emit_nurbs_curve normalizes to B_SPLINE_CURVE_WITH_KNOTS"
        )
    }
}
