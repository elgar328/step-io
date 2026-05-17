//! Shared writer helpers for the four NURBS handlers
//! (`B_SPLINE_CURVE_WITH_KNOTS`, `RATIONAL_B_SPLINE_CURVE`,
//! `B_SPLINE_SURFACE_WITH_KNOTS`, `RATIONAL_B_SPLINE_SURFACE`).
//!
//! Each handler still owns the simple-vs-complex body assembly; only the
//! attribute building that is identical across the rational/non-rational
//! pair lives here.

use crate::ir::attr::logical_to_step;
use crate::ir::geometry::{NurbsCurve, NurbsKind, NurbsSurface};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Schema form to emit for a unified `NurbsCurve`. Detected from the IR
/// pattern by [`detect_curve_schema_form`].
pub(crate) enum CurveSchemaForm {
    /// `B_SPLINE_CURVE_WITH_KNOTS` — simple, non-rational.
    SimpleWithKnots,
    /// `QUASI_UNIFORM_CURVE` — simple, non-rational, derived knots.
    SimpleQuasiUniform,
    /// Complex `RATIONAL_B_SPLINE_CURVE` with the `B_SPLINE_CURVE_WITH_KNOTS` marker.
    ComplexRationalWithKnots,
    /// Complex `RATIONAL_B_SPLINE_CURVE` with the `QUASI_UNIFORM_CURVE` marker.
    ComplexRationalQuasiUniform,
}

/// STEP `QuasiUniformKnots` and `QuasiUniformKnotsMultiplicities` derives.
/// Returns `(multiplicities, knots)` where the knot vector has
/// `cp_count - degree + 1` entries (`0.0, 1.0, ...`) and the
/// multiplicity vector is `[degree + 1, 1, 1, ..., 1, degree + 1]`.
/// Returns `None` if `cp_count < degree + 1` (non-standard input).
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

/// True iff the multiplicity / knot vectors match the
/// `QUASI_UNIFORM_CURVE` derive pattern for `degree`. Comparison is
/// bit-exact (`k == i as f64`): any float drift in the source knot
/// values forces fallback to `B_SPLINE_CURVE_WITH_KNOTS` emission so
/// the writer never normalizes a non-integer knot down to its rounded
/// ideal mid-round-trip.
fn knots_match_quasi_uniform(degree: u32, mults: &[i64], knots: &[f64]) -> bool {
    let Some((expect_mults, expect_knots)) =
        quasi_uniform_knots(degree, mults_total_to_cp_count(degree, mults))
    else {
        return false;
    };
    if expect_mults != mults {
        return false;
    }
    expect_knots.as_slice() == knots
}

/// Recover the control-point count from a multiplicity vector and
/// degree. Total multiplicity = `cp_count + degree + 1` by STEP rule.
fn mults_total_to_cp_count(degree: u32, mults: &[i64]) -> usize {
    let total: i64 = mults.iter().sum();
    let dp1 = i64::from(degree + 1);
    usize::try_from((total - dp1).max(0)).unwrap_or(0)
}

pub(crate) fn detect_curve_schema_form(nurbs: &NurbsCurve) -> CurveSchemaForm {
    let quasi = knots_match_quasi_uniform(nurbs.degree, &nurbs.knot_multiplicities, &nurbs.knots)
        && mults_total_to_cp_count(nurbs.degree, &nurbs.knot_multiplicities)
            == nurbs.control_points.len();
    match (&nurbs.kind, quasi) {
        (NurbsKind::NonRational, true) => CurveSchemaForm::SimpleQuasiUniform,
        (NurbsKind::NonRational, false) => CurveSchemaForm::SimpleWithKnots,
        (NurbsKind::Rational { .. }, true) => CurveSchemaForm::ComplexRationalQuasiUniform,
        (NurbsKind::Rational { .. }, false) => CurveSchemaForm::ComplexRationalWithKnots,
    }
}

pub(super) struct CurveCommonAttrs {
    pub degree: Attribute,
    pub cps: Attribute,
    pub form: Attribute,
    pub closed: Attribute,
    pub self_intersect: Attribute,
    pub mults: Attribute,
    pub knots: Attribute,
    pub knot_spec: Attribute,
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
        closed: Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() }),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        mults: Attribute::List(
            nurbs
                .knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        knots: Attribute::List(nurbs.knots.iter().copied().map(Attribute::Real).collect()),
        knot_spec: Attribute::Enum("UNSPECIFIED".into()),
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
    pub u_mults: Attribute,
    pub v_mults: Attribute,
    pub u_knots: Attribute,
    pub v_knots: Attribute,
    pub knot_spec: Attribute,
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
        u_closed: Attribute::Enum(if nurbs.u_closed {
            "T".into()
        } else {
            "F".into()
        }),
        v_closed: Attribute::Enum(if nurbs.v_closed {
            "T".into()
        } else {
            "F".into()
        }),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        u_mults: Attribute::List(
            nurbs
                .u_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        v_mults: Attribute::List(
            nurbs
                .v_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        u_knots: Attribute::List(nurbs.u_knots.iter().copied().map(Attribute::Real).collect()),
        v_knots: Attribute::List(nurbs.v_knots.iter().copied().map(Attribute::Real).collect()),
        knot_spec: Attribute::Enum("UNSPECIFIED".into()),
    })
}
