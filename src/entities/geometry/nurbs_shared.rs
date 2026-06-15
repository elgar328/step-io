//! Shared NURBS reader helper. The writer-side `build_{curve,surface}_common`
//! attribute builders were removed once every NURBS handler moved to the 2-layer
//! (gen-early serialize) path; only the quasi-uniform knot derivation remains, as
//! it has no L1 representation (the knots are implicit on the wire).

/// STEP `QuasiUniformKnots` and `QuasiUniformKnotsMultiplicities` derives.
/// Returns `(multiplicities, knots)` where the knot vector has
/// `cp_count - degree + 1` entries (`0.0, 1.0, ...`) and the
/// multiplicity vector is `[degree + 1, 1, 1, ..., 1, degree + 1]`.
/// Returns `None` if `cp_count < degree + 1` (non-standard input).
///
/// Reader-only helper: used by the `QUASI_UNIFORM_CURVE` /
/// `QUASI_UNIFORM_SURFACE` lowers (simple and rational complex) to reconstruct
/// the implicit knot vector when reading those forms. The writer always emits
/// `B_SPLINE_CURVE_WITH_KNOTS` / `*_SURFACE_*` per the compatibility-first form
/// policy, so no writer-side caller exists.
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
