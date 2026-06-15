//! Shared writer helpers for the four NURBS handlers
//! (`B_SPLINE_CURVE_WITH_KNOTS`, `RATIONAL_B_SPLINE_CURVE`,
//! `B_SPLINE_SURFACE_WITH_KNOTS`, `RATIONAL_B_SPLINE_SURFACE`).
//!
//! Each handler still owns the simple-vs-complex body assembly; only the
//! attribute building that is identical across the rational/non-rational
//! pair lives here.

use crate::ir::attr::logical_to_step;
use crate::ir::geometry::{NurbsCurve, NurbsSurface};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// STEP `QuasiUniformKnots` and `QuasiUniformKnotsMultiplicities` derives.
/// Returns `(multiplicities, knots)` where the knot vector has
/// `cp_count - degree + 1` entries (`0.0, 1.0, ...`) and the
/// multiplicity vector is `[degree + 1, 1, 1, ..., 1, degree + 1]`.
/// Returns `None` if `cp_count < degree + 1` (non-standard input).
///
/// Reader-only helper: used by the `QUASI_UNIFORM_CURVE` /
/// `QUASI_UNIFORM_SURFACE` handlers (simple and rational complex) to
/// reconstruct the implicit knot vector when reading those forms.
/// Writer always emits `B_SPLINE_CURVE_WITH_KNOTS` / `*_SURFACE_*` per
/// the compatibility-first form policy, so no writer-side caller exists.
///
/// Examples (`degree = 3`):
/// - `cp = 4` → mults `[4, 4]`, knots `[0.0, 1.0]`.
/// - `cp = 7` → mults `[4, 1, 1, 1, 4]`, knots `[0.0, 1.0, 2.0, 3.0, 4.0]`.
#[allow(clippy::cast_precision_loss)] // knot index fits in f64 mantissa for realistic NURBS
pub(crate) fn quasi_uniform_knots(degree: u32, cp_count: usize) -> Option<(Vec<i64>, Vec<f64>)> {
    let d = degree as usize;
    if cp_count < d + 1 {
        return None;
    }
    let n = cp_count - d + 1;
    let knots: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let dp1 = i64::from(degree + 1);
    let mults = if n == 2 {
        vec![dp1, dp1]
    } else {
        let mut m = Vec::with_capacity(n);
        m.push(dp1);
        m.extend(std::iter::repeat_n(1, n - 2));
        m.push(dp1);
        m
    };
    Some((mults, knots))
}

pub(super) struct CurveCommonAttrs {
    pub degree: Attribute,
    pub cps: Attribute,
    pub form: Attribute,
    pub closed: Attribute,
    pub self_intersect: Attribute,
}

pub(super) fn build_curve_common(
    buf: &mut WriteBuffer,
    nurbs: &NurbsCurve,
) -> Result<CurveCommonAttrs, WriteError> {
    let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
    for &pid in &nurbs.control_points {
        cp_refs.push(buf.emit_point(pid)?);
    }
    let degree = Attribute::Integer(i64::from(nurbs.degree));
    Ok(CurveCommonAttrs {
        degree,
        cps: Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect()),
        form: Attribute::Enum(nurbs.form.as_step_enum().into()),
        closed: Attribute::Enum(logical_to_step(nurbs.closed).into()),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
    })
}

pub(super) struct SurfaceCommonAttrs {
    pub u_degree: Attribute,
    pub v_degree: Attribute,
    pub cps: Attribute,
    pub form: Attribute,
    pub u_closed: Attribute,
    pub v_closed: Attribute,
    pub self_intersect: Attribute,
}

pub(super) fn build_surface_common(
    buf: &mut WriteBuffer,
    nurbs: &NurbsSurface,
) -> Result<SurfaceCommonAttrs, WriteError> {
    let mut cp_rows: Vec<Attribute> = Vec::with_capacity(nurbs.control_points.len());
    for row in &nurbs.control_points {
        let mut refs = Vec::with_capacity(row.len());
        for &pid in row {
            refs.push(Attribute::EntityRef(buf.emit_point(pid)?));
        }
        cp_rows.push(Attribute::List(refs));
    }
    let u_degree = Attribute::Integer(i64::from(nurbs.u_degree));
    let v_degree = Attribute::Integer(i64::from(nurbs.v_degree));
    Ok(SurfaceCommonAttrs {
        u_degree,
        v_degree,
        cps: Attribute::List(cp_rows),
        form: Attribute::Enum(nurbs.form.as_step_enum().into()),
        u_closed: Attribute::Enum(logical_to_step(nurbs.u_closed).into()),
        v_closed: Attribute::Enum(logical_to_step(nurbs.v_closed).into()),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
    })
}
