//! `RATIONAL_B_SPLINE_CURVE` handler — 2D sister (pcurve subtree), rational
//! → `Curve2d::Nurbs` (2-layer path; reuses the 3D generated complex
//! bind/serialize).
//!
//! The `is_2d` flag routes this handler only to entities inside a PCURVE
//! `DEFINITIONAL_REPRESENTATION` subtree, so it never sees a 3D rational entity;
//! the 3D sister `rational_bspline_curve.rs` is skipped on the pcurve subtree.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{NurbsCurve2d, NurbsKind2d};
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct RationalBsplineCurve2dHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_CURVE",
    is_2d,
    cases = [[
        "BOUNDED_CURVE", "B_SPLINE_CURVE", "B_SPLINE_CURVE_WITH_KNOTS", "CURVE",
        "GEOMETRIC_REPRESENTATION_ITEM", "RATIONAL_B_SPLINE_CURVE", "REPRESENTATION_ITEM"
    ]]
)]
impl ComplexEntityHandler for RationalBsplineCurve2dHandler {
    type WriteInput = NurbsCurve2d;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rational_b_spline_curve(entity_id, parts)?;
        lower::lower_rational_bspline_curve_2d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve2d) -> Result<u64, WriteError> {
        let NurbsKind2d::Rational { weights } = &nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineCurve2dHandler::write requires weights".into(),
            });
        };
        let weights = weights.clone();
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            control_points.push(CartesianPoint2dHandler::write(buf, pid)?);
        }
        let early = lift::lift_rational_bspline_curve_2d(&nurbs, control_points, weights);
        Ok(serialize::serialize_rational_b_spline_curve(buf, &early))
    }
}
