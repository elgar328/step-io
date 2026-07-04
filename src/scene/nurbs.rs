//! Opt-in NURBS (rational B-spline) views over analytic STEP curves.
//!
//! The faithful model keeps curves in their original form (see
//! [`Curve::kind`](crate::scene::geometry::Curve::kind)); `to_nurbs()` derives a
//! single rational-B-spline representation on demand — e.g. for a geometry
//! kernel that wants every curve as a NURBS. This is a *derived view*: it is
//! computed from the original entity and never written back.

// Vector math reads best with the conventional short axis/scale names (x, y, z,
// a, b) rather than spelled-out identifiers.
#![allow(clippy::many_single_char_names)]

use crate::generated::model as m;
use crate::scene::Ctx;

/// A rational B-spline curve — the one NURBS form every analytic curve converts to.
///
/// The knot vector is fully expanded (each knot repeated by its multiplicity).
/// Invariants: `weights.len() == control_points.len()` and
/// `knots.len() == control_points.len() + degree + 1`.
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsCurve {
    /// Polynomial degree (order − 1).
    pub degree: usize,
    /// Control points in world coordinates.
    pub control_points: Vec<[f64; 3]>,
    /// One weight per control point (all `1.0` for a non-rational curve).
    pub weights: Vec<f64>,
    /// The expanded knot vector.
    pub knots: Vec<f64>,
}

/// A rational B-spline surface — the one NURBS form every analytic surface converts to.
///
/// `control_points[i][j]` / `weights[i][j]` are indexed `[u][v]`. Invariants:
/// the grid is `n_u × n_v`, `knots_u.len() == n_u + degree_u + 1`, and
/// `knots_v.len() == n_v + degree_v + 1`.
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsSurface {
    /// Polynomial degree in u.
    pub degree_u: usize,
    /// Polynomial degree in v.
    pub degree_v: usize,
    /// Control points in world coordinates, indexed `[u][v]`.
    pub control_points: Vec<Vec<[f64; 3]>>,
    /// One weight per control point, indexed `[u][v]`.
    pub weights: Vec<Vec<f64>>,
    /// The expanded u knot vector.
    pub knots_u: Vec<f64>,
    /// The expanded v knot vector.
    pub knots_v: Vec<f64>,
}

// --- small vector helpers ---

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
/// Unit vector, or `None` for a (near-)zero-length input.
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = dot(a, a).sqrt();
    (n >= 1e-12).then(|| scale(a, 1.0 / n))
}

// --- reference resolution ---

fn point_coords(cx: Ctx<'_>, r: &m::CartesianPointRef) -> Option<[f64; 3]> {
    let m::CartesianPointRef::CartesianPoint(id) = r else {
        return None;
    };
    let c = &cx.model.cartesian_point_arena.get(id.0).coordinates;
    Some([
        c.first().copied().unwrap_or(0.0),
        c.get(1).copied().unwrap_or(0.0),
        c.get(2).copied().unwrap_or(0.0),
    ])
}

fn dir_ratios(cx: Ctx<'_>, r: &m::DirectionRef) -> Option<[f64; 3]> {
    let m::DirectionRef::Direction(id) = r else {
        return None;
    };
    let d = &cx.model.direction_arena.get(id.0).direction_ratios;
    Some([
        d.first().copied().unwrap_or(0.0),
        d.get(1).copied().unwrap_or(0.0),
        d.get(2).copied().unwrap_or(0.0),
    ])
}

/// Unit local control points of the 4×90° rational-quadratic full circle.
const CIRCLE_U: [(f64, f64); 9] = [
    (1.0, 0.0),
    (1.0, 1.0),
    (0.0, 1.0),
    (-1.0, 1.0),
    (-1.0, 0.0),
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (1.0, 0.0),
];
/// The clamped knot vector for a 9-control-point degree-2 full circle.
const CIRCLE_KNOTS: [f64; 12] = [0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.];

/// The circle control-point weights (corner points carry √2/2).
fn circle_weights() -> Vec<f64> {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    vec![1.0, w, 1.0, w, 1.0, w, 1.0, w, 1.0]
}

/// An orthonormal local frame: `(centre, x, y, z)`.
type Frame = ([f64; 3], [f64; 3], [f64; 3], [f64; 3]);

/// A borrowed knot vector: `(knot values, multiplicities)`.
type KnotsRef<'a> = (&'a [f64], &'a [i64]);

/// The orthonormal local frame of an `AXIS2_PLACEMENT_3D` (z is the axis /
/// revolution direction). `None` for a degenerate frame.
fn frame_of(cx: Ctx<'_>, id: m::Axis2Placement3dId) -> Option<Frame> {
    let p = cx.model.axis2_placement3d_arena.get(id.0);
    let center = point_coords(cx, &p.location)?;
    let z = match &p.axis {
        Some(a) => normalize(dir_ratios(cx, a)?)?,
        None => [0.0, 0.0, 1.0],
    };
    let x0 = match &p.ref_direction {
        Some(d) => dir_ratios(cx, d)?,
        None => [1.0, 0.0, 0.0],
    };
    // STEP derives x by projecting ref_direction orthogonal to the axis.
    let x = normalize(sub(x0, scale(z, dot(x0, z))))?;
    let y = cross(z, x);
    Some((center, x, y, z))
}

/// The frame of a curve's placement (2D parameter-space placements return
/// `None`).
fn placement_frame_3d(cx: Ctx<'_>, r: &m::Axis2PlacementRef) -> Option<Frame> {
    let m::Axis2PlacementRef::Axis2Placement3d(id) = r else {
        return None;
    };
    frame_of(cx, *id)
}

/// The frame of a surface's placement (always 3D).
fn frame_of_3dref(cx: Ctx<'_>, r: &m::Axis2Placement3dRef) -> Option<Frame> {
    let m::Axis2Placement3dRef::Axis2Placement3d(id) = r;
    frame_of(cx, *id)
}

/// The frame of an `AXIS1_PLACEMENT` (z is the axis). An `AXIS1_PLACEMENT` has
/// no reference direction, so x/y are an arbitrary perpendicular pair — a full
/// revolution makes the u-origin choice immaterial.
pub(crate) fn axis1_frame(cx: Ctx<'_>, r: &m::Axis1PlacementRef) -> Option<Frame> {
    let m::Axis1PlacementRef::Axis1Placement(id) = r;
    let p = cx.model.axis1_placement_arena.get(id.0);
    let center = point_coords(cx, &p.location)?;
    let z = match &p.axis {
        Some(a) => normalize(dir_ratios(cx, a)?)?,
        None => [0.0, 0.0, 1.0],
    };
    let x0 = if z[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let x = normalize(sub(x0, scale(z, dot(x0, z))))?;
    Some((center, x, cross(z, x), z))
}

/// A full circle/ellipse as a degree-2 rational B-spline (four 90° arcs, nine
/// control points). `a`/`b` scale the local x/y axes (radius for a circle;
/// semi-axes for an ellipse).
fn conic_nurbs(center: [f64; 3], x: [f64; 3], y: [f64; 3], a: f64, b: f64) -> NurbsCurve {
    let control_points = CIRCLE_U
        .iter()
        .map(|&(ux, uy)| add(center, add(scale(x, ux * a), scale(y, uy * b))))
        .collect();
    NurbsCurve {
        degree: 2,
        control_points,
        weights: circle_weights(),
        knots: CIRCLE_KNOTS.to_vec(),
    }
}

pub(crate) fn circle_to_nurbs(cx: Ctx<'_>, c: &m::Circle) -> Option<NurbsCurve> {
    if let Some((center, x, y, _)) = placement_frame_3d(cx, &c.position) {
        Some(conic_nurbs(center, x, y, c.radius, c.radius))
    } else {
        cx.warn("CIRCLE.to_nurbs: unsupported (2D or degenerate) placement".to_owned());
        None
    }
}

pub(crate) fn ellipse_to_nurbs(cx: Ctx<'_>, e: &m::Ellipse) -> Option<NurbsCurve> {
    if let Some((center, x, y, _)) = placement_frame_3d(cx, &e.position) {
        Some(conic_nurbs(center, x, y, e.semi_axis_1, e.semi_axis_2))
    } else {
        cx.warn("ELLIPSE.to_nurbs: unsupported (2D or degenerate) placement".to_owned());
        None
    }
}

/// Expand a `(distinct knots, multiplicities)` pair into a full knot vector.
fn expand_knots(knots: &[f64], mults: &[i64]) -> Vec<f64> {
    let mut out = Vec::new();
    for (k, mult) in knots.iter().zip(mults) {
        for _ in 0..(*mult).max(0) {
            out.push(*k);
        }
    }
    out
}

/// Assemble a B-spline curve from its raw pieces — shared by the standalone
/// `B_SPLINE_CURVE_WITH_KNOTS` and the complex (rational) form. `weights` is
/// `None` for a non-rational curve (all `1.0`).
fn nurbs_curve_from(
    cx: Ctx<'_>,
    degree: i64,
    cp: &[m::CartesianPointRef],
    knots: &[f64],
    mults: &[i64],
    weights: Option<&[f64]>,
) -> Option<NurbsCurve> {
    let degree = usize::try_from(degree).ok()?;
    let control_points: Vec<[f64; 3]> = cp
        .iter()
        .map(|r| point_coords(cx, r))
        .collect::<Option<_>>()?;
    let n = control_points.len();
    let weights = weights.map_or_else(|| vec![1.0; n], <[f64]>::to_vec);
    let knots = expand_knots(knots, mults);
    if weights.len() != n || knots.len() != n + degree + 1 {
        cx.warn("B_SPLINE_CURVE.to_nurbs: inconsistent weight/knot count".to_owned());
        return None;
    }
    Some(NurbsCurve {
        degree,
        control_points,
        weights,
        knots,
    })
}

/// Pass a standalone (non-rational) B-spline with explicit knots through.
pub(crate) fn bspline_wk_to_nurbs(cx: Ctx<'_>, b: &m::BSplineCurveWithKnots) -> Option<NurbsCurve> {
    nurbs_curve_from(
        cx,
        b.degree,
        &b.control_points_list,
        &b.knots,
        &b.knot_multiplicities,
        None,
    )
}

// --- uniform-knot B-spline subtypes (knots derived, not stored) ---

/// The B-spline subtypes whose knot vector is *derived* by an ISO 10303-42
/// rule rather than stored: `UNIFORM_*`, `QUASI_UNIFORM_*` and `BEZIER_*`.
#[derive(Clone, Copy)]
pub(crate) enum KnotFamily {
    Uniform,
    QuasiUniform,
    Bezier,
}

/// The standard knot vector of a knot family as `(knot values, multiplicities)`
/// — the same OCCT synthesizes. Uniform: integers `0..n+d`, all multiplicity 1
/// (unclamped). Quasi-uniform: integers `0..n−d`, clamped ends (`d+1`).
/// Bézier: piecewise, interior multiplicity `d` — requires `n = k·d + 1`
/// control points (`None` otherwise, as for too few control points).
#[allow(clippy::cast_precision_loss)] // knot indices are far below 2^52
fn family_knots(family: KnotFamily, degree: i64, n_cp: usize) -> Option<(Vec<f64>, Vec<i64>)> {
    let d = usize::try_from(degree).ok()?;
    if d == 0 || n_cp < d + 1 {
        return None;
    }
    let d_i = i64::try_from(d).ok()?;
    Some(match family {
        KnotFamily::Uniform => {
            let count = n_cp + d + 1;
            ((0..count).map(|j| j as f64).collect(), vec![1; count])
        }
        KnotFamily::QuasiUniform => {
            let m = n_cp - d;
            let mut mults = vec![1; m + 1];
            mults[0] = d_i + 1;
            mults[m] = d_i + 1;
            ((0..=m).map(|j| j as f64).collect(), mults)
        }
        KnotFamily::Bezier => {
            if (n_cp - 1) % d != 0 {
                return None;
            }
            let k = (n_cp - 1) / d;
            let mut mults = vec![d_i; k + 1];
            mults[0] = d_i + 1;
            mults[k] = d_i + 1;
            ((0..=k).map(|j| j as f64).collect(), mults)
        }
    })
}

/// Convert a uniform-family B-spline curve (its knots derived, everything else
/// as in `B_SPLINE_CURVE_WITH_KNOTS`).
pub(crate) fn uniform_family_curve_to_nurbs(
    cx: Ctx<'_>,
    degree: i64,
    cp: &[m::CartesianPointRef],
    family: KnotFamily,
) -> Option<NurbsCurve> {
    let Some((knots, mults)) = family_knots(family, degree, cp.len()) else {
        cx.warn(
            "B_SPLINE_CURVE.to_nurbs: control-point count invalid for its knot family".to_owned(),
        );
        return None;
    };
    nurbs_curve_from(cx, degree, cp, &knots, &mults, None)
}

/// Convert a uniform-family B-spline surface (u and v knots both derived).
pub(crate) fn uniform_family_surface_to_nurbs(
    cx: Ctx<'_>,
    degrees: (i64, i64),
    cp: &[Vec<m::CartesianPointRef>],
    family: KnotFamily,
) -> Option<NurbsSurface> {
    let n_v = cp.first().map_or(0, Vec::len);
    let u = family_knots(family, degrees.0, cp.len());
    let v = family_knots(family, degrees.1, n_v);
    let (Some((uk, um)), Some((vk, vm))) = (u, v) else {
        cx.warn(
            "B_SPLINE_SURFACE.to_nurbs: control-grid size invalid for its knot family".to_owned(),
        );
        return None;
    };
    nurbs_surface_from(cx, degrees, cp, (&uk, &um), (&vk, &vm), None)
}

/// A polyline as an exact degree-1 B-spline: the points are the control points
/// and the knots are the quasi-uniform (clamped) degree-1 vector.
pub(crate) fn polyline_to_nurbs(cx: Ctx<'_>, p: &m::Polyline) -> Option<NurbsCurve> {
    if p.points.len() < 2 {
        cx.warn("POLYLINE.to_nurbs: fewer than two points".to_owned());
        return None;
    }
    let (knots, mults) = family_knots(KnotFamily::QuasiUniform, 1, p.points.len())?;
    nurbs_curve_from(cx, 1, &p.points, &knots, &mults, None)
}

/// Assemble a rational (or plain) B-spline curve from a complex instance's
/// parts: `B_SPLINE_CURVE` (degree + control points), knots from a
/// `B_SPLINE_CURVE_WITH_KNOTS` part or derived from a uniform-family marker
/// part, and an optional `RATIONAL_B_SPLINE_CURVE` (weights). `None` if the
/// complex is neither.
pub(crate) fn complex_bspline_curve_to_nurbs(
    cx: Ctx<'_>,
    parts: &[m::UnitPart],
) -> Option<NurbsCurve> {
    let (degree, cp) = parts.iter().find_map(|p| match p {
        m::UnitPart::BSplineCurve {
            degree,
            control_points_list,
            ..
        } => Some((*degree, control_points_list)),
        _ => None,
    })?;
    let explicit = parts.iter().find_map(|p| match p {
        m::UnitPart::BSplineCurveWithKnots {
            knots,
            knot_multiplicities,
            ..
        } => Some((knots, knot_multiplicities)),
        _ => None,
    });
    let derived;
    let (knots, mults): (&[f64], &[i64]) = if let Some((k, m)) = explicit {
        (k, m)
    } else {
        let family = parts.iter().find_map(|p| match p {
            m::UnitPart::UniformCurve => Some(KnotFamily::Uniform),
            m::UnitPart::QuasiUniformCurve => Some(KnotFamily::QuasiUniform),
            m::UnitPart::BezierCurve => Some(KnotFamily::Bezier),
            _ => None,
        })?;
        derived = family_knots(family, degree, cp.len())?;
        (&derived.0, &derived.1)
    };
    let weights = parts.iter().find_map(|p| match p {
        m::UnitPart::RationalBSplineCurve { weights_data } => Some(weights_data.as_slice()),
        _ => None,
    });
    nurbs_curve_from(cx, degree, cp, knots, mults, weights)
}

// --- trimmed curves (bounded arcs / segments) ---

/// The direction vector (orientation × magnitude) of a `VECTOR`.
fn vector_dir(cx: Ctx<'_>, r: &m::VectorRef) -> Option<[f64; 3]> {
    let m::VectorRef::Vector(id) = r else {
        return None;
    };
    let v = cx.model.vector_arena.get(id.0);
    Some(scale(dir_ratios(cx, &v.orientation)?, v.magnitude))
}

/// The world point of a line trim: a `CARTESIAN_POINT` or a `PARAMETER_VALUE`
/// `t` (giving `base + t·dir`). The line's vector already carries the scale, so
/// this is unit-independent.
fn line_trim_point(
    cx: Ctx<'_>,
    trim: &[m::TrimmingSelectRef],
    base: [f64; 3],
    dir: [f64; 3],
) -> Option<[f64; 3]> {
    trim.iter().find_map(|s| match s {
        m::TrimmingSelectRef::CartesianPoint(id) => {
            let c = &cx.model.cartesian_point_arena.get(id.0).coordinates;
            Some([
                c.first().copied().unwrap_or(0.0),
                c.get(1).copied().unwrap_or(0.0),
                c.get(2).copied().unwrap_or(0.0),
            ])
        }
        m::TrimmingSelectRef::ParameterValue(t) => Some(add(base, scale(dir, *t))),
        _ => None,
    })
}

/// The angle of a conic trim, derived geometrically from a `CARTESIAN_POINT` on
/// the curve (unit-independent). `ParameterValue` trims are angle-unit dependent
/// and unsupported here (→ `None`).
fn conic_trim_angle(
    cx: Ctx<'_>,
    trim: &[m::TrimmingSelectRef],
    frame: ([f64; 3], [f64; 3], [f64; 3]),
    ab: (f64, f64),
    factor: f64,
    master: m::TrimmingPreference,
) -> Option<f64> {
    let (center, x, y) = frame;
    let (a, b) = ab;
    // From a CartesianPoint on the curve — geometric, unit-independent.
    let cartesian = || {
        let p = trim.iter().find_map(|s| match s {
            m::TrimmingSelectRef::CartesianPoint(id) => {
                let c = &cx.model.cartesian_point_arena.get(id.0).coordinates;
                Some([
                    c.first().copied().unwrap_or(0.0),
                    c.get(1).copied().unwrap_or(0.0),
                    c.get(2).copied().unwrap_or(0.0),
                ])
            }
            _ => None,
        })?;
        let d = sub(p, center);
        Some((dot(d, y) / b).atan2(dot(d, x) / a))
    };
    // From a ParameterValue (an angle in the model's plane-angle unit).
    let parameter = || {
        trim.iter().find_map(|s| match s {
            m::TrimmingSelectRef::ParameterValue(v) => Some(*v * factor),
            _ => None,
        })
    };
    match master {
        m::TrimmingPreference::Parameter => parameter().or_else(cartesian),
        _ => cartesian().or_else(parameter),
    }
}

/// A rational-quadratic arc from `theta_s` sweeping `sweep` radians (signed) in
/// the local frame, scaled by `a`/`b` (radius for a circle; semi-axes for an
/// ellipse — an ellipse arc is the affine image of the circle arc).
// Segment counts convert to/from f64 for the knot vector; the values are tiny.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn arc_nurbs(
    center: [f64; 3],
    x: [f64; 3],
    y: [f64; 3],
    a: f64,
    b: f64,
    theta_s: f64,
    sweep: f64,
) -> NurbsCurve {
    let narcs = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let dtheta = sweep / narcs as f64;
    let w1 = (dtheta.abs() / 2.0).cos();
    let world = |lx: f64, ly: f64| add(center, add(scale(x, lx * a), scale(y, ly * b)));

    let mut control_points = vec![world(theta_s.cos(), theta_s.sin())];
    let mut weights = vec![1.0];
    for k in 0..narcs {
        let mid = theta_s + dtheta * (k as f64 + 0.5);
        control_points.push(world(mid.cos() / w1, mid.sin() / w1));
        weights.push(w1);
        let end = theta_s + dtheta * (k as f64 + 1.0);
        control_points.push(world(end.cos(), end.sin()));
        weights.push(1.0);
    }
    let mut knots = vec![0.0, 0.0, 0.0];
    for i in 1..narcs {
        knots.push(i as f64);
        knots.push(i as f64);
    }
    let n = narcs as f64;
    knots.extend([n, n, n]);

    NurbsCurve {
        degree: 2,
        control_points,
        weights,
        knots,
    }
}

fn arc_from_trims(
    cx: Ctx<'_>,
    t: &m::TrimmedCurve,
    frame: ([f64; 3], [f64; 3], [f64; 3]),
    ab: (f64, f64),
) -> Option<NurbsCurve> {
    use std::f64::consts::TAU;
    // ParameterValue trims are angles in the model's plane-angle unit.
    let factor = crate::scene::units::units_of(cx.model)
        .angle
        .map_or(1.0, |u| u.to_si);
    let master = t.master_representation;
    let theta1 = conic_trim_angle(cx, &t.trim_1, frame, ab, factor, master)?;
    let mut theta2 = conic_trim_angle(cx, &t.trim_2, frame, ab, factor, master)?;
    if t.sense_agreement {
        while theta2 <= theta1 {
            theta2 += TAU;
        }
    } else {
        while theta2 >= theta1 {
            theta2 -= TAU;
        }
    }
    let (center, x, y) = frame;
    let (a, b) = ab;
    Some(arc_nurbs(center, x, y, a, b, theta1, theta2 - theta1))
}

/// A trimmed line (→ segment), trimmed circle/ellipse (→ arc), or trimmed
/// B-spline-family basis (→ the exact knot-inserted subcurve). Other basis
/// curves are not yet supported (→ `None`).
pub(crate) fn trimmed_to_nurbs(cx: Ctx<'_>, t: &m::TrimmedCurve) -> Option<NurbsCurve> {
    match &t.basis_curve {
        m::CurveRef::Line(id) => {
            let line = cx.model.line_arena.get(id.0);
            let base = point_coords(cx, &line.pnt)?;
            let dir = vector_dir(cx, &line.dir)?;
            let p0 = line_trim_point(cx, &t.trim_1, base, dir)?;
            let p1 = line_trim_point(cx, &t.trim_2, base, dir)?;
            Some(NurbsCurve {
                degree: 1,
                control_points: vec![p0, p1],
                weights: vec![1.0, 1.0],
                knots: vec![0.0, 0.0, 1.0, 1.0],
            })
        }
        m::CurveRef::Circle(id) => {
            let c = cx.model.circle_arena.get(id.0);
            let (center, x, y, _) = placement_frame_3d(cx, &c.position)?;
            arc_from_trims(cx, t, (center, x, y), (c.radius, c.radius))
        }
        m::CurveRef::Ellipse(id) => {
            let e = cx.model.ellipse_arena.get(id.0);
            let (center, x, y, _) = placement_frame_3d(cx, &e.position)?;
            arc_from_trims(cx, t, (center, x, y), (e.semi_axis_1, e.semi_axis_2))
        }
        _ => {
            let basis = trimmed_basis_nurbs(cx, &t.basis_curve)?;
            trimmed_bspline_to_nurbs(cx, t, &basis)
        }
    }
}

// --- bspline segment extraction (trimmed B-spline basis) ---

/// Convert a B-spline-family basis of a trimmed curve. The kinds are matched
/// here directly (not via the generic curve dispatcher) so a malformed
/// self-referencing `TRIMMED_CURVE` basis cannot recurse.
fn trimmed_basis_nurbs(cx: Ctx<'_>, r: &m::CurveRef) -> Option<NurbsCurve> {
    match r {
        m::CurveRef::BSplineCurveWithKnots(id) => {
            bspline_wk_to_nurbs(cx, cx.model.b_spline_curve_with_knots_arena.get(id.0))
        }
        m::CurveRef::Complex(id) => {
            complex_bspline_curve_to_nurbs(cx, &cx.model.complex_unit_arena.get(id.0).parts)
        }
        m::CurveRef::UniformCurve(id) => {
            let c = cx.model.uniform_curve_arena.get(id.0);
            uniform_family_curve_to_nurbs(cx, c.degree, &c.control_points_list, KnotFamily::Uniform)
        }
        m::CurveRef::QuasiUniformCurve(id) => {
            let c = cx.model.quasi_uniform_curve_arena.get(id.0);
            uniform_family_curve_to_nurbs(
                cx,
                c.degree,
                &c.control_points_list,
                KnotFamily::QuasiUniform,
            )
        }
        m::CurveRef::BezierCurve(id) => {
            let c = cx.model.bezier_curve_arena.get(id.0);
            uniform_family_curve_to_nurbs(cx, c.degree, &c.control_points_list, KnotFamily::Bezier)
        }
        m::CurveRef::Polyline(id) => polyline_to_nurbs(cx, cx.model.polyline_arena.get(id.0)),
        _ => None,
    }
}

/// A NURBS curve in homogeneous form `(w·x, w·y, w·z, w)`, with its first
/// derivative curve precomputed — the working representation for evaluation,
/// point inversion and knot insertion.
struct HomCurve {
    degree: usize,
    knots: Vec<f64>,
    cps: Vec<[f64; 4]>,
    dknots: Vec<f64>,
    dcps: Vec<[f64; 4]>,
}

fn hom_lerp(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// The knot span containing `t`: the largest non-empty span index `k` with
/// `knots[k] <= t`, clamped to the domain (in particular `t = domain end` maps
/// to the last span — the standard de Boor boundary case).
fn find_span(knots: &[f64], degree: usize, t: f64) -> usize {
    let n = knots.len() - degree - 1; // number of control points
    if t >= knots[n] {
        return n - 1;
    }
    let mut k = degree;
    while k + 1 < n && knots[k + 1] <= t {
        k += 1;
    }
    k
}

/// De Boor evaluation of a (homogeneous) B-spline at `t`.
fn de_boor(cps: &[[f64; 4]], knots: &[f64], degree: usize, t: f64) -> [f64; 4] {
    let k = find_span(knots, degree, t);
    let mut d: Vec<[f64; 4]> = (0..=degree).map(|j| cps[k - degree + j]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = k - degree + j;
            let denom = knots[i + degree + 1 - r] - knots[i];
            let a = if denom.abs() < 1e-300 {
                0.0
            } else {
                (t - knots[i]) / denom
            };
            d[j] = hom_lerp(d[j - 1], d[j], a);
        }
    }
    d[degree]
}

impl HomCurve {
    fn new(c: &NurbsCurve) -> Self {
        let p = c.degree;
        let cps: Vec<[f64; 4]> = c
            .control_points
            .iter()
            .zip(&c.weights)
            .map(|(&q, &w)| [q[0] * w, q[1] * w, q[2] * w, w])
            .collect();
        // The derivative curve: degree p−1 over the knot vector with the two
        // outermost knots dropped.
        #[allow(clippy::cast_precision_loss)] // the degree is tiny
        let dcps: Vec<[f64; 4]> = cps
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let denom = c.knots[i + p + 1] - c.knots[i + 1];
                let s = if denom.abs() < 1e-300 {
                    0.0
                } else {
                    p as f64 / denom
                };
                [
                    (w[1][0] - w[0][0]) * s,
                    (w[1][1] - w[0][1]) * s,
                    (w[1][2] - w[0][2]) * s,
                    (w[1][3] - w[0][3]) * s,
                ]
            })
            .collect();
        HomCurve {
            degree: p,
            knots: c.knots.clone(),
            cps,
            dknots: c.knots[1..c.knots.len() - 1].to_vec(),
            dcps,
        }
    }

    fn domain(&self) -> (f64, f64) {
        let n = self.knots.len() - self.degree - 1;
        (self.knots[self.degree], self.knots[n])
    }

    /// The rational point and first derivative at `t` (quotient rule).
    fn point_deriv(&self, t: f64) -> ([f64; 3], [f64; 3]) {
        let h = de_boor(&self.cps, &self.knots, self.degree, t);
        let hd = de_boor(&self.dcps, &self.dknots, self.degree - 1, t);
        let w = h[3];
        let point = [h[0] / w, h[1] / w, h[2] / w];
        let deriv = [
            (hd[0] - point[0] * hd[3]) / w,
            (hd[1] - point[1] * hd[3]) / w,
            (hd[2] - point[2] * hd[3]) / w,
        ];
        (point, deriv)
    }

    /// The curve parameter of a point (assumed on or near the curve): a coarse
    /// per-span sampling picks the nearest start, then a point-projection
    /// iteration refines it to machine precision.
    fn invert_point(&self, target: [f64; 3]) -> Option<f64> {
        let (lo, hi) = self.domain();
        let span = hi - lo;
        if !span.is_finite() || span <= 0.0 {
            return None;
        }
        let mut best = (f64::MAX, lo);
        for w in self.knots.windows(2) {
            if w[1] - w[0] < 1e-300 || w[1] <= lo || w[0] >= hi {
                continue;
            }
            for s in 0..=8_i32 {
                let t = w[0] + (w[1] - w[0]) * (f64::from(s) / 8.0);
                let (p, _) = self.point_deriv(t);
                let d = sub(p, target);
                let d2 = dot(d, d);
                if d2 < best.0 {
                    best = (d2, t);
                }
            }
        }
        let mut t = best.1;
        for _ in 0..30 {
            let (p, dv) = self.point_deriv(t);
            let dd = dot(dv, dv);
            if dd < 1e-300 {
                break;
            }
            let dt = dot(sub(target, p), dv) / dd;
            t = (t + dt).clamp(lo, hi);
            if dt.abs() < 1e-13 * span {
                break;
            }
        }
        Some(t)
    }
}

/// Insert `t` once (Boehm), updating the homogeneous control points and the
/// expanded knot vector in place.
fn insert_knot_once(cps: &mut Vec<[f64; 4]>, knots: &mut Vec<f64>, degree: usize, t: f64) {
    let k = find_span(knots, degree, t);
    let q: Vec<[f64; 4]> = ((k - degree + 1)..=k)
        .map(|i| {
            let denom = knots[i + degree] - knots[i];
            let a = if denom.abs() < 1e-300 {
                0.0
            } else {
                (t - knots[i]) / denom
            };
            hom_lerp(cps[i - 1], cps[i], a)
        })
        .collect();
    let tail = cps[k..].to_vec();
    cps.truncate(k - degree + 1);
    cps.extend(q);
    cps.extend(tail);
    knots.insert(k + 1, t);
}

/// The exact subcurve of a NURBS over `[t0, t1]` (both inside the domain,
/// `t0 < t1`): each end is knot-inserted to full multiplicity and the clamped
/// slice between them is taken.
fn bspline_segment(c: &NurbsCurve, t0: f64, t1: f64) -> Option<NurbsCurve> {
    let p = c.degree;
    let mut knots = c.knots.clone();
    let mut cps: Vec<[f64; 4]> = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(&q, &w)| [q[0] * w, q[1] * w, q[2] * w, w])
        .collect();
    let n = knots.len() - p - 1;
    let eps = 1e-9 * (knots[n] - knots[p]).abs();
    if t1 - t0 <= eps {
        return None;
    }
    for &t in &[t0, t1] {
        let mult = knots.iter().filter(|k| (**k - t).abs() <= eps).count();
        for _ in mult..p {
            insert_knot_once(&mut cps, &mut knots, p, t);
        }
    }
    let n_le = knots.iter().filter(|k| **k <= t0 + eps).count();
    let cp_start = n_le.checked_sub(p + 1)?;
    let interior: Vec<f64> = knots
        .iter()
        .copied()
        .filter(|k| *k > t0 + eps && *k < t1 - eps)
        .collect();
    let mut seg_knots = vec![t0; p + 1];
    seg_knots.extend(interior);
    seg_knots.extend(vec![t1; p + 1]);
    let n_seg = seg_knots.len() - p - 1;
    if cp_start + n_seg > cps.len() {
        return None;
    }
    let (control_points, weights) = cps[cp_start..cp_start + n_seg]
        .iter()
        .map(|h| ([h[0] / h[3], h[1] / h[3], h[2] / h[3]], h[3]))
        .unzip();
    Some(NurbsCurve {
        degree: p,
        control_points,
        weights,
        knots: seg_knots,
    })
}

/// Reverse a curve's direction (mirrored knots over the domain).
fn reverse_curve(c: &NurbsCurve) -> NurbsCurve {
    let (a, b) = (c.knots[0], c.knots[c.knots.len() - 1]);
    NurbsCurve {
        degree: c.degree,
        control_points: c.control_points.iter().rev().copied().collect(),
        weights: c.weights.iter().rev().copied().collect(),
        knots: c.knots.iter().rev().map(|k| a + b - k).collect(),
    }
}

/// Resolve one trim of a B-spline basis to a parameter: a `PARAMETER_VALUE` is
/// already in the basis's own knot space (unit-free); a `CARTESIAN_POINT` is
/// inverted by point projection. Ordered by `master_representation`.
fn bspline_trim_param(
    cx: Ctx<'_>,
    trim: &[m::TrimmingSelectRef],
    hc: &HomCurve,
    master: m::TrimmingPreference,
) -> Option<f64> {
    let parameter = || {
        trim.iter().find_map(|s| match s {
            m::TrimmingSelectRef::ParameterValue(v) => Some(*v),
            _ => None,
        })
    };
    let cartesian = || {
        let p = trim.iter().find_map(|s| match s {
            m::TrimmingSelectRef::CartesianPoint(id) => {
                let c = &cx.model.cartesian_point_arena.get(id.0).coordinates;
                Some([
                    c.first().copied().unwrap_or(0.0),
                    c.get(1).copied().unwrap_or(0.0),
                    c.get(2).copied().unwrap_or(0.0),
                ])
            }
            _ => None,
        })?;
        hc.invert_point(p)
    };
    let t = match master {
        m::TrimmingPreference::Cartesian => cartesian().or_else(parameter),
        _ => parameter().or_else(cartesian),
    }?;
    Some(snap_param(hc, t))
}

/// Snap a parameter to an existing knot so an inversion residual cannot
/// create a sliver span on insertion.
fn snap_param(hc: &HomCurve, t: f64) -> f64 {
    let (lo, hi) = hc.domain();
    let snap = 1e-7 * (hi - lo);
    hc.knots
        .iter()
        .copied()
        .find(|k| (k - t).abs() <= snap)
        .unwrap_or(t)
}

/// Extract the segment from `t0` to `t1` — the `trim_1` → `trim_2` (or edge
/// start → end) direction; `sense` says that direction has increasing
/// parameter. A closed basis has a double solution at its seam, normalized by
/// the direction; a genuine mid-curve wrap-around is unsupported. `what` names
/// the calling entity in warnings.
fn segment_between_params(
    cx: Ctx<'_>,
    basis: &NurbsCurve,
    hc: &HomCurve,
    (mut t0, mut t1): (f64, f64),
    sense: bool,
    what: &str,
) -> Option<NurbsCurve> {
    let (lo, hi) = hc.domain();
    let eps = 1e-7 * (hi - lo);
    let closed = {
        let (a, _) = hc.point_deriv(lo);
        let (b, _) = hc.point_deriv(hi);
        let d = sub(a, b);
        let size = 1.0 + dot(a, a).sqrt().max(dot(b, b).sqrt());
        dot(d, d).sqrt() <= 1e-9 * size
    };
    // The seam of a closed basis is both `lo` and `hi`; pick the side that
    // makes the direction consistent.
    if sense {
        if t0 > t1 && closed {
            if (t0 - hi).abs() <= eps {
                t0 = lo;
            } else if (t1 - lo).abs() <= eps {
                t1 = hi;
            }
        }
        if t0 >= t1 {
            cx.warn(format!(
                "{what}.to_nurbs: wrap-around trim on a B-spline basis"
            ));
            return None;
        }
        bspline_segment(basis, t0, t1)
    } else {
        if t0 < t1 && closed {
            if (t0 - lo).abs() <= eps {
                t0 = hi;
            } else if (t1 - hi).abs() <= eps {
                t1 = lo;
            }
        }
        if t1 >= t0 {
            cx.warn(format!(
                "{what}.to_nurbs: wrap-around trim on a B-spline basis"
            ));
            return None;
        }
        Some(reverse_curve(&bspline_segment(basis, t1, t0)?))
    }
}

/// A trimmed curve over a B-spline-family basis: resolve both trims to
/// parameters and extract the exact segment.
fn trimmed_bspline_to_nurbs(
    cx: Ctx<'_>,
    t: &m::TrimmedCurve,
    basis: &NurbsCurve,
) -> Option<NurbsCurve> {
    let hc = HomCurve::new(basis);
    let master = t.master_representation;
    let (Some(t0), Some(t1)) = (
        bspline_trim_param(cx, &t.trim_1, &hc, master),
        bspline_trim_param(cx, &t.trim_2, &hc, master),
    ) else {
        cx.warn("TRIMMED_CURVE.to_nurbs: no usable trim on a B-spline basis".to_owned());
        return None;
    };
    segment_between_params(cx, basis, &hc, (t0, t1), t.sense_agreement, "TRIMMED_CURVE")
}

// --- surface curves (3D-curve + on-surface association containers) ---

/// Resolve through surface-curve containers to the underlying 3D curve.
/// `SURFACE_CURVE` and its subtypes pair a `curve_3d` with the same curve's
/// on-surface (pcurve) representations; the 3D curve carries the geometry.
/// The unwrap is an iterative hop (never recursion), so malformed nested or
/// self-referencing containers terminate; the limit is far above the real
/// nesting depth of one.
fn resolve_curve_3d<'m>(cx: Ctx<'m>, r: &'m m::CurveRef) -> Option<&'m m::CurveRef> {
    let mut cur = r;
    for _ in 0..16 {
        cur = match cur {
            m::CurveRef::SurfaceCurve(id) => &cx.model.surface_curve_arena.get(id.0).curve_3d,
            m::CurveRef::SeamCurve(id) => &cx.model.seam_curve_arena.get(id.0).curve_3d,
            m::CurveRef::BoundedSurfaceCurve(id) => {
                &cx.model.bounded_surface_curve_arena.get(id.0).curve_3d
            }
            m::CurveRef::IntersectionCurve(id) => {
                &cx.model.intersection_curve_arena.get(id.0).curve_3d
            }
            other => return Some(other),
        };
    }
    cx.warn("SURFACE_CURVE.to_nurbs: curve_3d container nesting too deep".to_owned());
    None
}

/// The terminal whole-curve dispatch for an already-resolved (non-container)
/// curve: trimmed / full conics / composite / the bounded B-spline family.
fn resolved_curve_to_nurbs(cx: Ctx<'_>, r: &m::CurveRef) -> Option<NurbsCurve> {
    match r {
        m::CurveRef::TrimmedCurve(id) => {
            trimmed_to_nurbs(cx, cx.model.trimmed_curve_arena.get(id.0))
        }
        m::CurveRef::Circle(id) => circle_to_nurbs(cx, cx.model.circle_arena.get(id.0)),
        m::CurveRef::Ellipse(id) => ellipse_to_nurbs(cx, cx.model.ellipse_arena.get(id.0)),
        m::CurveRef::CompositeCurve(id) => {
            composite_to_nurbs(cx, cx.model.composite_curve_arena.get(id.0))
        }
        other => trimmed_basis_nurbs(cx, other),
    }
}

/// Convert a surface-curve container by delegating to its 3D curve.
pub(crate) fn surface_curve_3d_to_nurbs(cx: Ctx<'_>, r: &m::CurveRef) -> Option<NurbsCurve> {
    resolved_curve_to_nurbs(cx, resolve_curve_3d(cx, r)?)
}

/// The bounded NURBS of a b-rep edge: its geometry between the two vertex
/// points `p0`/`p1`, in the edge's own direction (start → end). `same_sense`
/// says the edge direction agrees with the curve's parameter direction.
/// Contrast with `Curve::to_nurbs`, which converts the whole curve — here an
/// unbounded line becomes the finite segment and a partial arc/span is cut
/// out; an edge whose two vertices coincide is a closed edge and yields the
/// whole (closed) curve.
pub(crate) fn edge_to_nurbs(
    cx: Ctx<'_>,
    geometry: &m::CurveRef,
    p0: [f64; 3],
    p1: [f64; 3],
    same_sense: bool,
) -> Option<NurbsCurve> {
    let r = resolve_curve_3d(cx, geometry)?;
    let size = 1.0
        + p0.iter()
            .chain(p1.iter())
            .fold(0.0_f64, |m, v| m.max(v.abs()));
    let d = sub(p1, p0);
    if dot(d, d).sqrt() <= 1e-9 * size {
        // A closed edge: the whole (closed) curve.
        if matches!(r, m::CurveRef::Line(_)) {
            cx.warn("EDGE_CURVE.to_nurbs: closed edge on a line".to_owned());
            return None;
        }
        return resolved_curve_to_nurbs(cx, r);
    }
    match r {
        m::CurveRef::Line(_) => Some(NurbsCurve {
            degree: 1,
            control_points: vec![p0, p1],
            weights: vec![1.0, 1.0],
            knots: vec![0.0, 0.0, 1.0, 1.0],
        }),
        m::CurveRef::Circle(id) => {
            let c = cx.model.circle_arena.get(id.0);
            let frame = placement_frame_3d(cx, &c.position)?;
            Some(conic_edge_arc(
                frame,
                (c.radius, c.radius),
                p0,
                p1,
                same_sense,
            ))
        }
        m::CurveRef::Ellipse(id) => {
            let e = cx.model.ellipse_arena.get(id.0);
            let frame = placement_frame_3d(cx, &e.position)?;
            Some(conic_edge_arc(
                frame,
                (e.semi_axis_1, e.semi_axis_2),
                p0,
                p1,
                same_sense,
            ))
        }
        other => {
            let whole = resolved_curve_to_nurbs(cx, other)?;
            let hc = HomCurve::new(&whole);
            let (t0, t1) = (
                snap_param(&hc, hc.invert_point(p0)?),
                snap_param(&hc, hc.invert_point(p1)?),
            );
            segment_between_params(cx, &whole, &hc, (t0, t1), same_sense, "EDGE_CURVE")
        }
    }
}

/// The arc of a circle/ellipse between two on-curve points, sweeping in the
/// `sense` parameter direction (which of the two arcs between the points).
fn conic_edge_arc(
    frame: Frame,
    ab: (f64, f64),
    p0: [f64; 3],
    p1: [f64; 3],
    sense: bool,
) -> NurbsCurve {
    use std::f64::consts::TAU;
    let (center, x, y, _) = frame;
    let angle = |p: [f64; 3]| {
        let d = sub(p, center);
        (dot(d, y) / ab.1).atan2(dot(d, x) / ab.0)
    };
    let theta1 = angle(p0);
    let mut theta2 = angle(p1);
    if sense {
        while theta2 <= theta1 {
            theta2 += TAU;
        }
    } else {
        while theta2 >= theta1 {
            theta2 -= TAU;
        }
    }
    arc_nurbs(center, x, y, ab.0, ab.1, theta1, theta2 - theta1)
}

// --- composite curves (segment chains joined into one NURBS) ---

/// Convert one composite-curve segment's parent curve. A trimmed curve is the
/// dominant case; the bounded B-spline family comes via the same direct table
/// as trimmed bases (again avoiding the generic dispatcher, so a malformed
/// self-referencing chain cannot recurse).
fn segment_parent_nurbs(cx: Ctx<'_>, r: &m::CurveRef) -> Option<NurbsCurve> {
    match r {
        m::CurveRef::TrimmedCurve(id) => {
            trimmed_to_nurbs(cx, cx.model.trimmed_curve_arena.get(id.0))
        }
        _ => trimmed_basis_nurbs(cx, r),
    }
}

/// Raise a clamped curve's degree by one — exactly. The curve is split into
/// Bézier pieces (every interior knot inserted to full multiplicity), each
/// piece is elevated by the closed form `C_i = a·B_{i−1} + (1−a)·B_i` with
/// `a = i/(p+1)` (in homogeneous coordinates), and the pieces are reassembled.
/// No knot removal afterwards: the representation is non-minimal (interior
/// multiplicity p+1) but the geometry is identical.
#[allow(clippy::cast_precision_loss)] // degrees are tiny
fn elevate_once(c: &NurbsCurve) -> Option<NurbsCurve> {
    let p = c.degree;
    let mut knots = c.knots.clone();
    let mut cps: Vec<[f64; 4]> = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(&q, &w)| [q[0] * w, q[1] * w, q[2] * w, w])
        .collect();
    let n = knots.len() - p - 1;
    let (lo, hi) = (knots[p], knots[n]);
    let eps = 1e-9 * (hi - lo).abs();
    let mut distinct: Vec<f64> = Vec::new();
    for &k in &c.knots {
        if k > lo + eps && k < hi - eps && !distinct.iter().any(|d| (d - k).abs() <= eps) {
            distinct.push(k);
        }
    }
    for &t in &distinct {
        let mult = knots.iter().filter(|k| (**k - t).abs() <= eps).count();
        for _ in mult..p {
            insert_knot_once(&mut cps, &mut knots, p, t);
        }
    }
    // Clamped + fully split: pieces are consecutive windows of p+1 control
    // points stepping by p.
    let m = distinct.len() + 1;
    if cps.len() != m * p + 1 {
        return None;
    }
    let mut out = Vec::with_capacity(m * (p + 1) + 1);
    for j in 0..m {
        let b = &cps[j * p..=j * p + p];
        let start = usize::from(j != 0); // share the boundary control point
        for i in start..=(p + 1) {
            out.push(if i == 0 {
                b[0]
            } else if i == p + 1 {
                b[p]
            } else {
                hom_lerp(b[i], b[i - 1], i as f64 / (p + 1) as f64)
            });
        }
    }
    let mut out_knots = vec![lo; p + 2];
    for &d in &distinct {
        out_knots.extend(std::iter::repeat_n(d, p + 1));
    }
    out_knots.extend(std::iter::repeat_n(hi, p + 2));
    let (control_points, weights) = out
        .iter()
        .map(|h| ([h[0] / h[3], h[1] / h[3], h[2] / h[3]], h[3]))
        .unzip();
    Some(NurbsCurve {
        degree: p + 1,
        control_points,
        weights,
        knots: out_knots,
    })
}

/// Join consecutive segment curves into one NURBS: clamp-normalize each (an
/// unclamped B-spline's end control points are not its end points), elevate
/// all to the maximum degree, verify each junction actually meets, rescale
/// weights across junctions (a projective no-op) and chain the knot vectors
/// over consecutive unit intervals.
#[allow(clippy::cast_precision_loss)] // segment indices are tiny
fn join_segments(cx: Ctx<'_>, parts: &[NurbsCurve]) -> Option<NurbsCurve> {
    let clamped: Vec<NurbsCurve> = parts
        .iter()
        .map(|c| {
            let p = c.degree;
            let n = c.knots.len() - p - 1;
            bspline_segment(c, c.knots[p], c.knots[n])
        })
        .collect::<Option<_>>()?;
    let target = clamped.iter().map(|c| c.degree).max()?;
    let elevated: Vec<NurbsCurve> = clamped
        .into_iter()
        .map(|mut c| {
            while c.degree < target {
                c = elevate_once(&c)?;
            }
            Some(c)
        })
        .collect::<Option<_>>()?;
    let size = elevated
        .iter()
        .flat_map(|c| c.control_points.iter())
        .flat_map(|q| q.iter())
        .fold(0.0_f64, |m, v| m.max(v.abs()));
    let eps = 1e-9 * (1.0 + size);
    let p = target;
    let (mut cps, mut ws, mut knots) = (Vec::new(), Vec::new(), Vec::<f64>::new());
    for (i, seg) in elevated.iter().enumerate() {
        let (lo, hi) = (seg.knots[p], seg.knots[seg.knots.len() - p - 1]);
        if !(hi - lo).is_finite() || hi - lo <= 0.0 {
            return None;
        }
        let remap = |k: f64| i as f64 + (k - lo) / (hi - lo);
        if i == 0 {
            knots.extend(seg.knots.iter().map(|&k| remap(k)));
            cps.extend(seg.control_points.iter().copied());
            ws.extend(seg.weights.iter().copied());
        } else {
            let prev: [f64; 3] = *cps.last()?;
            let d = sub(prev, seg.control_points[0]);
            if dot(d, d).sqrt() > eps {
                cx.warn(
                    "COMPOSITE_CURVE.to_nurbs: segments do not join (discontinuous)".to_owned(),
                );
                return None;
            }
            // The junction knot keeps multiplicity p (C0): drop one clamp
            // knot from the accumulated end and the p+1 leading clamps of
            // the incoming segment; the shared control point is kept once.
            knots.pop();
            knots.extend(seg.knots.iter().skip(p + 1).map(|&k| remap(k)));
            let w_scale = *ws.last()? / seg.weights[0];
            cps.extend(seg.control_points.iter().skip(1).copied());
            ws.extend(seg.weights.iter().skip(1).map(|w| w * w_scale));
        }
    }
    debug_assert_eq!(knots.len(), cps.len() + p + 1);
    Some(NurbsCurve {
        degree: p,
        control_points: cps,
        weights: ws,
        knots,
    })
}

/// A composite curve as one exact NURBS: each segment converts on its own
/// (reversed when `same_sense` is false) and the chain is joined. The last
/// segment's transition code only marks the curve open/closed, so joinability
/// is decided by the junctions themselves.
pub(crate) fn composite_to_nurbs(cx: Ctx<'_>, cc: &m::CompositeCurve) -> Option<NurbsCurve> {
    let mut parts = Vec::with_capacity(cc.segments.len());
    for s in &cc.segments {
        let m::CompositeCurveSegmentRef::CompositeCurveSegment(id) = s else {
            cx.warn("COMPOSITE_CURVE.to_nurbs: unsupported complex segment".to_owned());
            return None;
        };
        let seg = cx.model.composite_curve_segment_arena.get(id.0);
        let Some(mut c) = segment_parent_nurbs(cx, &seg.parent_curve) else {
            cx.warn("COMPOSITE_CURVE.to_nurbs: unsupported segment parent curve".to_owned());
            return None;
        };
        if !seg.same_sense {
            c = reverse_curve(&c);
        }
        parts.push(c);
    }
    if parts.is_empty() {
        cx.warn("COMPOSITE_CURVE.to_nurbs: no segments".to_owned());
        return None;
    }
    join_segments(cx, &parts)
}

// --- surfaces of revolution ---

/// Revolve profile control points, given in the axis frame as
/// `((a, b), h, weight)` — perpendicular components a/b along x/y and height h
/// along z — a full turn to build a rational surface. u is the full
/// 9-control-point circle; v is the profile. Each `CIRCLE_U` entry acts on
/// (a, b) as a complex multiplication: the even entries rotate by 0/90/180/
/// 270°, the odd `(±1, ±1)` entries rotate by the mid-angle and scale the
/// perpendicular part by √2 (the tangent-intersection corner; h unscaled).
/// Exact — a rotation is linear in (cos v, sin v).
fn revolve_frame_cps(
    frame: Frame,
    cps: &[((f64, f64), f64, f64)],
    degree_v: usize,
    knots_v: Vec<f64>,
) -> NurbsSurface {
    let (center, x, y, z) = frame;
    let w_u = circle_weights();
    let control_points = CIRCLE_U
        .iter()
        .map(|&(ux, uy)| {
            cps.iter()
                .map(|&((a, b), h, _)| {
                    let (ra, rb) = (ux * a - uy * b, ux * b + uy * a);
                    add(center, add(add(scale(x, ra), scale(y, rb)), scale(z, h)))
                })
                .collect()
        })
        .collect();
    let weights = w_u
        .iter()
        .map(|&wu| cps.iter().map(|&(_, _, wv)| wu * wv).collect())
        .collect();
    NurbsSurface {
        degree_u: 2,
        degree_v,
        control_points,
        weights,
        knots_u: CIRCLE_KNOTS.to_vec(),
        knots_v,
    }
}

/// Revolve a meridian profile (`(rho, h, weight)` control points, where `rho`
/// is the radial distance from the axis and `h` the height along it) a full
/// turn — the in-meridian-plane (b = 0) case of [`revolve_frame_cps`].
fn revolve_nurbs(
    center: [f64; 3],
    x: [f64; 3],
    y: [f64; 3],
    z: [f64; 3],
    profile: &[(f64, f64, f64)],
    knots_v: Vec<f64>,
) -> NurbsSurface {
    let cps: Vec<((f64, f64), f64, f64)> = profile
        .iter()
        .map(|&(rho, h, w)| ((rho, 0.0), h, w))
        .collect();
    revolve_frame_cps((center, x, y, z), &cps, 2, knots_v)
}

/// Revolve an arbitrary NURBS profile curve a full turn around the frame's z
/// axis. Each profile control point is decomposed into the axis frame and
/// swept by the rational circle structure; v keeps the profile's degree and
/// knots. Exact for rational profiles too (the standard tensor construction).
pub(crate) fn revolve_curve(profile: &NurbsCurve, frame: Frame) -> NurbsSurface {
    let (center, x, y, z) = frame;
    let cps: Vec<((f64, f64), f64, f64)> = profile
        .control_points
        .iter()
        .zip(&profile.weights)
        .map(|(&p, &w)| {
            let d = sub(p, center);
            ((dot(d, x), dot(d, y)), dot(d, z), w)
        })
        .collect();
    revolve_frame_cps(frame, &cps, profile.degree, profile.knots.clone())
}

pub(crate) fn sphere_to_nurbs(cx: Ctx<'_>, s: &m::SphericalSurface) -> Option<NurbsSurface> {
    let Some((center, x, y, z)) = frame_of_3dref(cx, &s.position) else {
        cx.warn("SPHERICAL_SURFACE.to_nurbs: degenerate placement".to_owned());
        return None;
    };
    let r = s.radius;
    let w = std::f64::consts::FRAC_1_SQRT_2;
    // A 180° meridian (south pole → +x equator → north pole), two 90° arcs.
    let profile = [
        (0.0, -r, 1.0),
        (r, -r, w),
        (r, 0.0, 1.0),
        (r, r, w),
        (0.0, r, 1.0),
    ];
    let knots_v = vec![0., 0., 0., 1., 1., 2., 2., 2.];
    Some(revolve_nurbs(center, x, y, z, &profile, knots_v))
}

pub(crate) fn torus_to_nurbs(cx: Ctx<'_>, t: &m::ToroidalSurface) -> Option<NurbsSurface> {
    let Some((center, x, y, z)) = frame_of_3dref(cx, &t.position) else {
        cx.warn("TOROIDAL_SURFACE.to_nurbs: degenerate placement".to_owned());
        return None;
    };
    let big = t.major_radius;
    let r = t.minor_radius;
    let w = std::f64::consts::FRAC_1_SQRT_2;
    // The full tube circle in the (rho, h) half-plane, centred at (major, 0).
    let profile = [
        (big + r, 0.0, 1.0),
        (big + r, r, w),
        (big, r, 1.0),
        (big - r, r, w),
        (big - r, 0.0, 1.0),
        (big - r, -r, w),
        (big, -r, 1.0),
        (big + r, -r, w),
        (big + r, 0.0, 1.0),
    ];
    Some(revolve_nurbs(
        center,
        x,
        y,
        z,
        &profile,
        CIRCLE_KNOTS.to_vec(),
    ))
}

/// Pass a B-spline surface with an explicit knot vector straight through,
/// expanding both knot vectors and defaulting weights to `1.0`.
/// Assemble a B-spline surface from its raw pieces — shared by the standalone
/// `B_SPLINE_SURFACE_WITH_KNOTS` and the complex (rational) form. `weights` is
/// `None` for a non-rational surface (all `1.0`). `u`/`v` are each
/// `(knots, multiplicities)`.
fn nurbs_surface_from(
    cx: Ctx<'_>,
    degrees: (i64, i64),
    cp: &[Vec<m::CartesianPointRef>],
    u: (&[f64], &[i64]),
    v: (&[f64], &[i64]),
    weights: Option<&[Vec<f64>]>,
) -> Option<NurbsSurface> {
    let degree_u = usize::try_from(degrees.0).ok()?;
    let degree_v = usize::try_from(degrees.1).ok()?;
    let control_points: Vec<Vec<[f64; 3]>> = cp
        .iter()
        .map(|row| {
            row.iter()
                .map(|r| point_coords(cx, r))
                .collect::<Option<_>>()
        })
        .collect::<Option<_>>()?;
    let n_u = control_points.len();
    let n_v = control_points.first().map_or(0, Vec::len);
    if n_u == 0 || n_v == 0 || control_points.iter().any(|row| row.len() != n_v) {
        cx.warn("B_SPLINE_SURFACE.to_nurbs: empty or ragged control grid".to_owned());
        return None;
    }
    let weights = weights.map_or_else(|| vec![vec![1.0; n_v]; n_u], <[Vec<f64>]>::to_vec);
    let knots_u = expand_knots(u.0, u.1);
    let knots_v = expand_knots(v.0, v.1);
    if weights.len() != n_u
        || weights.iter().any(|row| row.len() != n_v)
        || knots_u.len() != n_u + degree_u + 1
        || knots_v.len() != n_v + degree_v + 1
    {
        cx.warn("B_SPLINE_SURFACE.to_nurbs: inconsistent weight/knot count".to_owned());
        return None;
    }
    Some(NurbsSurface {
        degree_u,
        degree_v,
        control_points,
        weights,
        knots_u,
        knots_v,
    })
}

/// Pass a standalone (non-rational) B-spline surface with explicit knots through.
pub(crate) fn bspline_surf_wk_to_nurbs(
    cx: Ctx<'_>,
    b: &m::BSplineSurfaceWithKnots,
) -> Option<NurbsSurface> {
    nurbs_surface_from(
        cx,
        (b.u_degree, b.v_degree),
        &b.control_points_list,
        (&b.u_knots, &b.u_multiplicities),
        (&b.v_knots, &b.v_multiplicities),
        None,
    )
}

/// Assemble a rational (or plain) B-spline surface from a complex instance's
/// parts — knots from a `B_SPLINE_SURFACE_WITH_KNOTS` part or derived from a
/// uniform-family marker part. `None` if the complex is neither.
pub(crate) fn complex_bspline_surface_to_nurbs(
    cx: Ctx<'_>,
    parts: &[m::UnitPart],
) -> Option<NurbsSurface> {
    let (u_degree, v_degree, cp) = parts.iter().find_map(|p| match p {
        m::UnitPart::BSplineSurface {
            u_degree,
            v_degree,
            control_points_list,
            ..
        } => Some((*u_degree, *v_degree, control_points_list)),
        _ => None,
    })?;
    let explicit = parts.iter().find_map(|p| match p {
        m::UnitPart::BSplineSurfaceWithKnots {
            u_knots,
            u_multiplicities,
            v_knots,
            v_multiplicities,
            ..
        } => Some((u_knots, u_multiplicities, v_knots, v_multiplicities)),
        _ => None,
    });
    let derived;
    let (u, v): (KnotsRef<'_>, KnotsRef<'_>) = if let Some((uk, um, vk, vm)) = explicit {
        ((uk, um), (vk, vm))
    } else {
        let family = parts.iter().find_map(|p| match p {
            m::UnitPart::UniformSurface => Some(KnotFamily::Uniform),
            m::UnitPart::QuasiUniformSurface => Some(KnotFamily::QuasiUniform),
            m::UnitPart::BezierSurface => Some(KnotFamily::Bezier),
            _ => None,
        })?;
        let n_v = cp.first().map_or(0, Vec::len);
        derived = (
            family_knots(family, u_degree, cp.len())?,
            family_knots(family, v_degree, n_v)?,
        );
        ((&derived.0.0, &derived.0.1), (&derived.1.0, &derived.1.1))
    };
    let weights = parts.iter().find_map(|p| match p {
        m::UnitPart::RationalBSplineSurface { weights_data } => Some(weights_data.as_slice()),
        _ => None,
    });
    nurbs_surface_from(cx, (u_degree, v_degree), cp, u, v, weights)
}

// --- bounded patches from a face ---

/// A finite bilinear patch of a `PLANE`, sized to the parameter-space bounding
/// box of the face's edge vertices. The result is the *untrimmed* base surface
/// over that box — the face's edge loops trim it separately. `points` are the
/// face's edge-vertex world coordinates; assumes straight edges (the box is
/// taken from vertices, so a curved edge could under-cover).
pub(crate) fn plane_patch(
    cx: Ctx<'_>,
    plane: &m::Plane,
    points: &[[f64; 3]],
) -> Option<NurbsSurface> {
    let (origin, x, y, _) = frame_of_3dref(cx, &plane.position)?;
    if points.is_empty() {
        return None;
    }
    let (mut u_lo, mut u_hi, mut v_lo, mut v_hi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in points {
        let d = sub(*p, origin);
        let (u, v) = (dot(d, x), dot(d, y));
        u_lo = u_lo.min(u);
        u_hi = u_hi.max(u);
        v_lo = v_lo.min(v);
        v_hi = v_hi.max(v);
    }
    if u_hi - u_lo < 1e-12 || v_hi - v_lo < 1e-12 {
        cx.warn("PLANE face.to_nurbs: degenerate (collinear) parameter extent".to_owned());
        return None;
    }
    let corner = |u: f64, v: f64| add(origin, add(scale(x, u), scale(y, v)));
    Some(NurbsSurface {
        degree_u: 1,
        degree_v: 1,
        control_points: vec![
            vec![corner(u_lo, v_lo), corner(u_lo, v_hi)],
            vec![corner(u_hi, v_lo), corner(u_hi, v_hi)],
        ],
        weights: vec![vec![1.0; 2]; 2],
        knots_u: vec![0.0, 0.0, 1.0, 1.0],
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
    })
}

/// A cylinder or cone bounded by a face into a finite ruled patch: the full
/// circle (rational quadratic, u) swept linearly (v) between two axial levels,
/// taken from the axial span of the face's edge vertices. `semi_angle` is `0.0`
/// for a cylinder; for a cone the radius grows `radius + v·tan(semi_angle)`.
///
/// Exact within the axial span (both surfaces are ruled). The full circle
/// over-covers a partial face — the edge loops trim it. `None` if the axial
/// span is degenerate.
pub(crate) fn cylinder_cone_patch(
    cx: Ctx<'_>,
    position: &m::Axis2Placement3dRef,
    radius: f64,
    semi_angle: f64,
    points: &[[f64; 3]],
) -> Option<NurbsSurface> {
    let (center, x, y, z) = frame_of_3dref(cx, position)?;
    if points.is_empty() {
        return None;
    }
    let (mut v_lo, mut v_hi) = (f64::MAX, f64::MIN);
    for p in points {
        let v = dot(sub(*p, center), z);
        v_lo = v_lo.min(v);
        v_hi = v_hi.max(v);
    }
    if v_hi - v_lo < 1e-12 {
        cx.warn("CYLINDRICAL/CONICAL face.to_nurbs: degenerate axial extent".to_owned());
        return None;
    }
    let r_lo = radius + v_lo * semi_angle.tan();
    let r_hi = radius + v_hi * semi_angle.tan();
    let w_u = circle_weights();
    let control_points = CIRCLE_U
        .iter()
        .map(|&(ux, uy)| {
            let radial = add(scale(x, ux), scale(y, uy));
            vec![
                add(add(center, scale(z, v_lo)), scale(radial, r_lo)),
                add(add(center, scale(z, v_hi)), scale(radial, r_hi)),
            ]
        })
        .collect();
    let weights = w_u.iter().map(|&w| vec![w, w]).collect();
    Some(NurbsSurface {
        degree_u: 2,
        degree_v: 1,
        control_points,
        weights,
        knots_u: CIRCLE_KNOTS.to_vec(),
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
    })
}

/// A `SURFACE_OF_LINEAR_EXTRUSION` bounded by a face into a finite ruled patch:
/// the profile curve (u, already converted to NURBS) swept linearly (v) along
/// the extrusion direction between two levels taken from the span of the face's
/// edge vertices. Only the vector's *direction* is used — its magnitude is a
/// parameter scale, not an extrusion length (the surface is formally unbounded).
///
/// The v-span is widened by the profile's own span along the direction (the
/// control-point hull bounds the curve), so the patch always covers the face —
/// the edge loops trim it. Exact within the span (the surface is ruled).
pub(crate) fn extrude_patch(
    cx: Ctx<'_>,
    profile: &NurbsCurve,
    axis: &m::VectorRef,
    points: &[[f64; 3]],
) -> Option<NurbsSurface> {
    let m::VectorRef::Vector(id) = axis else {
        return None;
    };
    let v = cx.model.vector_arena.get(id.0);
    let Some(d) = normalize(dir_ratios(cx, &v.orientation)?) else {
        cx.warn("SURFACE_OF_LINEAR_EXTRUSION face.to_nurbs: degenerate direction".to_owned());
        return None;
    };
    if points.is_empty() {
        return None;
    }
    let span = |ps: &[[f64; 3]]| {
        ps.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            let t = dot(*p, d);
            (lo.min(t), hi.max(t))
        })
    };
    let (pc_lo, pc_hi) = span(&profile.control_points);
    let (b_lo, b_hi) = span(points);
    let (v_lo, v_hi) = (b_lo - pc_hi, b_hi - pc_lo);
    if v_hi - v_lo < 1e-12 {
        cx.warn("SURFACE_OF_LINEAR_EXTRUSION face.to_nurbs: degenerate extent".to_owned());
        return None;
    }
    let control_points = profile
        .control_points
        .iter()
        .map(|&cp| vec![add(cp, scale(d, v_lo)), add(cp, scale(d, v_hi))])
        .collect();
    let weights = profile.weights.iter().map(|&w| vec![w, w]).collect();
    Some(NurbsSurface {
        degree_u: profile.degree,
        degree_v: 1,
        control_points,
        weights,
        knots_u: profile.knots.clone(),
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
    })
}

/// A `LINE` profile of a surface of revolution, bounded by the face into a
/// finite degree-1 segment (which [`revolve_curve`] then sweeps). Height and
/// radius are rotation-invariant, so each edge vertex pins the line parameter:
/// via its height along the axis for a slanted line (exact), or via the radius
/// equation for a line (nearly) perpendicular to the axis — all real roots are
/// taken, a safe over-cover. The line vector's magnitude is a parameter scale,
/// not an extent.
pub(crate) fn revolved_line_segment(
    cx: Ctx<'_>,
    line: &m::Line,
    frame: Frame,
    points: &[[f64; 3]],
) -> Option<NurbsCurve> {
    let (center, _, _, z) = frame;
    let base = point_coords(cx, &line.pnt)?;
    let m::VectorRef::Vector(vid) = &line.dir else {
        return None;
    };
    let v = cx.model.vector_arena.get(vid.0);
    let d = normalize(dir_ratios(cx, &v.orientation)?)?;
    if points.is_empty() {
        return None;
    }
    let rel = sub(base, center);
    let h_base = dot(rel, z);
    let dz = dot(d, z);
    let (mut t_lo, mut t_hi) = (f64::MAX, f64::MIN);
    if dz.abs() >= 1e-7 {
        for p in points {
            let t = (dot(sub(*p, center), z) - h_base) / dz;
            t_lo = t_lo.min(t);
            t_hi = t_hi.max(t);
        }
    } else {
        // Perpendicular components of the line's base and direction.
        let w = sub(rel, scale(z, h_base));
        let u = sub(d, scale(z, dz));
        let (uu, wu, ww) = (dot(u, u), dot(w, u), dot(w, w));
        if uu < 1e-12 {
            return None;
        }
        for p in points {
            let pr = sub(*p, center);
            let perp = sub(pr, scale(z, dot(pr, z)));
            // |w + t·u|² = |perp|², a quadratic in t; a vertex off the
            // surface has no real root and is skipped.
            let disc = wu * wu - uu * (ww - dot(perp, perp));
            if disc < 0.0 {
                continue;
            }
            let s = disc.sqrt();
            for t in [(-wu - s) / uu, (-wu + s) / uu] {
                t_lo = t_lo.min(t);
                t_hi = t_hi.max(t);
            }
        }
    }
    if t_hi - t_lo < 1e-12 {
        cx.warn("SURFACE_OF_REVOLUTION face.to_nurbs: degenerate line-profile extent".to_owned());
        return None;
    }
    Some(NurbsCurve {
        degree: 1,
        control_points: vec![add(base, scale(d, t_lo)), add(base, scale(d, t_hi))],
        weights: vec![1.0, 1.0],
        knots: vec![0.0, 0.0, 1.0, 1.0],
    })
}
