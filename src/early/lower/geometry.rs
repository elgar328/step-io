//! Geometry-domain `lower` fns (the W-RECURSIVE pilot: leaf points/directions
//! + vertex). See the [module docs](super) for the lowering contract.
//!
//! The 2D/3D `CARTESIAN_POINT` / `DIRECTION` sister handlers both run on the
//! same entity and self-claim by coordinate count; these lowers reproduce the
//! 3D branch exactly (2-count → the 2D arena claims it, so we no-op).

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement2d, EarlyAxis2Placement3d, EarlyBSplineCurveWithKnots,
    EarlyBSplineSurfaceWithKnots, EarlyBoundedPcurve, EarlyBoundedSurfaceCurve,
    EarlyCartesianPoint, EarlyCircle, EarlyCircularArea, EarlyCompositeCurve,
    EarlyCompositeCurveSegment, EarlyConicalSurface, EarlyCurveBoundedSurface,
    EarlyCylindricalSurface, EarlyDegenerateToroidalSurface, EarlyDirection, EarlyEllipse,
    EarlyHyperbola, EarlyIntersectionCurve, EarlyLine, EarlyOffsetCurve3d, EarlyOffsetSurface,
    EarlyParabola, EarlyPlanarBox, EarlyPlanarExtent, EarlyPlane, EarlyPolyline,
    EarlyQuasiUniformCurve, EarlyQuasiUniformSurface, EarlyRationalBSplineCurve,
    EarlyRationalBSplineSurface, EarlyRationalQuasiUniformCurve, EarlyRationalQuasiUniformSurface,
    EarlyRectangularTrimmedSurface, EarlySphericalSurface, EarlySurfaceOfLinearExtrusion,
    EarlySurfaceOfRevolution, EarlyToroidalSurface, EarlyTrimSelect, EarlyTrimmedCurve,
    EarlyVector, EarlyVertexPoint,
};
use crate::entities::geometry::nurbs_shared::quasi_uniform_knots;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, BoundedPCurve, Circle2, Circle3,
    CircularArea, CircularAreaCentre, CompositeCurve, CompositeSegment, ConicalSurface, Curve,
    Curve2d, CurveBoundedSurface, CylindricalSurface, DegenerateToroidalSurface, Direction2,
    Direction3, Ellipse2, Ellipse3, Hyperbola, Line2, Line3, NurbsCurve, NurbsCurve2d, NurbsKind,
    NurbsKind2d, NurbsSurface, NurbsSurfaceKind, OffsetCurve3d, PCurveOrSurface, Parabola,
    ParameterSpaceCurve, PlanarBox, PlanarBoxPlacement, PlanarExtent, PlanarExtentData, Plane3,
    Point2, Point3, Polyline, Polyline2d, PreferredSurfaceCurveRepresentation,
    RectangularTrimmedSurface, SphericalSurface, Surface, SurfaceCurve, SurfaceCurveData,
    SurfaceOfLinearExtrusion, SurfaceOfOffset, SurfaceOfRevolution, ToroidalSurface, TrimSelect,
    TrimmedCurve, Vertex,
};
use crate::reader::ReaderContext;

/// Lower one `BOUNDED_PCURVE` — resolve `basis_surface` + `reference_to_curve`
/// and push to the `parameter_space_curves` arena. Orphan (no inbound refs) so
/// no `id_cache` entry; a missing ref drops it (matching the legacy handler).
pub(crate) fn lower_bounded_pcurve(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: &EarlyBoundedPcurve,
) {
    let Some(basis_surface) = ctx
        .id_cache
        .get::<crate::ir::id::SurfaceId>(early.basis_surface)
    else {
        return;
    };
    let Some(reference_to_curve) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.reference_to_curve)
    else {
        return;
    };
    ctx.geometry
        .parameter_space_curves
        .push(ParameterSpaceCurve::BoundedPCurve(BoundedPCurve {
            name: early.name.clone(),
            basis_surface,
            reference_to_curve,
        }));
}

/// Lower one `B_SPLINE_CURVE_WITH_KNOTS` (non-rational). Mirrors the legacy
/// handler: `degree` narrows to `u32` (error on overflow) first; then if the
/// first control point is a 2D point this no-ops (the 2D sister
/// `b_spline_curve_2d_with_knots` claims it). `knot_spec` is informational and
/// dropped (the writer always re-emits `UNSPECIFIED`).
pub(crate) fn lower_b_spline_curve_with_knots(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyBSplineCurveWithKnots,
) -> Result<(), ConvertError> {
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if let Some(&first) = early.control_points_list.first()
        && ctx.id_cache.contains::<crate::ir::id::Point2dId>(first)
    {
        return Ok(()); // 2D sister handler claims this instance
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        control_points.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
    }
    let curve = NurbsCurve {
        degree,
        control_points,
        kind: NurbsKind::NonRational,
        knot_multiplicities: early.knot_multiplicities.clone(),
        knots: early.knots.clone(),
        closed: early.closed_curve,
        form: early.curve_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.curves.push(Curve::Nurbs(curve));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `B_SPLINE_SURFACE_WITH_KNOTS` (non-rational). Mirrors the legacy
/// handler: `u_degree`/`v_degree` narrow to `u32` (error on overflow) first, then
/// each control point in the u/v grid resolves through the shared point resolver.
/// There is no 2D sister (surface control points are always 3D), so no self-claim
/// guard. `knot_spec` is informational and dropped (the writer always re-emits
/// `UNSPECIFIED`).
pub(crate) fn lower_b_spline_surface_with_knots(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyBSplineSurfaceWithKnots,
) -> Result<(), ConvertError> {
    let u_degree = u32::try_from(early.u_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "u_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let v_degree = u32::try_from(early.v_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "v_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for row in &early.control_points_list {
        let mut pt_row = Vec::with_capacity(row.len());
        for &r in row {
            pt_row.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
        }
        control_points.push(pt_row);
    }
    let surface = NurbsSurface {
        u_degree,
        v_degree,
        control_points,
        kind: NurbsSurfaceKind::NonRational,
        u_knot_multiplicities: early.u_multiplicities.clone(),
        v_knot_multiplicities: early.v_multiplicities.clone(),
        u_knots: early.u_knots.clone(),
        v_knots: early.v_knots.clone(),
        u_closed: early.u_closed,
        v_closed: early.v_closed,
        form: early.surface_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.surfaces.push(Surface::Nurbs(surface));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `QUASI_UNIFORM_CURVE` (non-rational). Like the `b_spline` curve but
/// knots/multiplicities are absent on the wire and **derived** from
/// `(degree, cp_count)` via `quasi_uniform_knots` (`None` → `DimensionMismatch`,
/// matching the legacy handler). The first-2D-cp guard mirrors the curve. This
/// form is `read_only` (never re-emitted), so there is no `lift`/`serialize`.
pub(crate) fn lower_quasi_uniform_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyQuasiUniformCurve,
) -> Result<(), ConvertError> {
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if let Some(&first) = early.control_points_list.first()
        && ctx.id_cache.contains::<crate::ir::id::Point2dId>(first)
    {
        return Ok(()); // 2D sister instance
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        control_points.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
    }
    let Some((knot_multiplicities, knots)) = quasi_uniform_knots(degree, control_points.len())
    else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: degree as usize + 1,
            actual: control_points.len(),
        });
    };
    let curve = NurbsCurve {
        degree,
        control_points,
        kind: NurbsKind::NonRational,
        knot_multiplicities,
        knots,
        closed: early.closed_curve,
        form: early.curve_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.curves.push(Curve::Nurbs(curve));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `QUASI_UNIFORM_SURFACE` (non-rational). u/v knots are derived per
/// direction via `quasi_uniform_knots` (`None` → `DimensionMismatch`). No 2D
/// sister; `read_only` (no `lift`/`serialize`).
pub(crate) fn lower_quasi_uniform_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyQuasiUniformSurface,
) -> Result<(), ConvertError> {
    let u_degree = u32::try_from(early.u_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "u_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let v_degree = u32::try_from(early.v_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "v_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for row in &early.control_points_list {
        let mut pt_row = Vec::with_capacity(row.len());
        for &r in row {
            pt_row.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
        }
        control_points.push(pt_row);
    }
    let u_cp_count = control_points.len();
    let v_cp_count = control_points.first().map_or(0, Vec::len);
    let Some((u_knot_multiplicities, u_knots)) = quasi_uniform_knots(u_degree, u_cp_count) else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: u_degree as usize + 1,
            actual: u_cp_count,
        });
    };
    let Some((v_knot_multiplicities, v_knots)) = quasi_uniform_knots(v_degree, v_cp_count) else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: v_degree as usize + 1,
            actual: v_cp_count,
        });
    };
    let surface = NurbsSurface {
        u_degree,
        v_degree,
        control_points,
        kind: NurbsSurfaceKind::NonRational,
        u_knot_multiplicities,
        v_knot_multiplicities,
        u_knots,
        v_knots,
        u_closed: early.u_closed,
        v_closed: early.v_closed,
        form: early.surface_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.surfaces.push(Surface::Nurbs(surface));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `RATIONAL_B_SPLINE_CURVE` (complex, rational). Mirrors the legacy
/// complex handler: degree narrows to `u32`; `weights_data` length must equal the
/// control-point count (`DimensionMismatch` otherwise). No 2D guard — the 3D and
/// 2D rational handlers share `cases`, disambiguated by the pcurve partition.
/// `knot_spec` is informational and dropped.
pub(crate) fn lower_rational_bspline_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRationalBSplineCurve,
) -> Result<(), ConvertError> {
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if early.weights_data.len() != early.control_points_list.len() {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "weights_data",
            expected: early.control_points_list.len(),
            actual: early.weights_data.len(),
        });
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        control_points.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
    }
    let curve = NurbsCurve {
        degree,
        control_points,
        kind: NurbsKind::Rational {
            weights: early.weights_data.clone(),
        },
        knot_multiplicities: early.knot_multiplicities.clone(),
        knots: early.knots.clone(),
        closed: early.closed_curve,
        form: early.curve_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.curves.push(Curve::Nurbs(curve));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower the 2D-arena variant of `RATIONAL_B_SPLINE_CURVE` (complex, rational;
/// sister of [`lower_rational_bspline_curve`]). The first control point's arena
/// discriminates **first** (a 3D point falls through to the 3D sister), then the
/// `weights`/control-point count is checked and `degree` narrows to `u32` — the
/// legacy handler's check order. `self_intersect`/`knot_spec` are not modelled by
/// `NurbsCurve2d` (the writer re-emits `.F.` / `UNSPECIFIED`).
pub(crate) fn lower_rational_bspline_curve_2d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRationalBSplineCurve,
) -> Result<(), ConvertError> {
    if let Some(&first) = early.control_points_list.first()
        && !ctx.id_cache.contains::<crate::ir::id::Point2dId>(first)
    {
        return Ok(()); // 3D sister handler claims this instance
    }
    if early.weights_data.len() != early.control_points_list.len() {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "weights_data",
            expected: early.control_points_list.len(),
            actual: early.weights_data.len(),
        });
    }
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        let pt = ctx.id_cache.get::<crate::ir::id::Point2dId>(r).ok_or(
            ConvertError::MissingReference {
                from: entity_id,
                to: r,
                field_name: "control_points_list",
            },
        )?;
        control_points.push(pt);
    }
    let id = ctx.geometry.curves_2d.push(Curve2d::Nurbs(NurbsCurve2d {
        degree,
        control_points,
        kind: NurbsKind2d::Rational {
            weights: early.weights_data.clone(),
        },
        knot_multiplicities: early.knot_multiplicities.clone(),
        knots: early.knots.clone(),
        closed: early.closed_curve,
        form: early.curve_form,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `RATIONAL_B_SPLINE_SURFACE` (complex, rational). u/v degrees narrow
/// to `u32`; the `weights_data` 2D grid must match the control-point grid shape
/// (row count and each row's length — `DimensionMismatch` otherwise). No 2D guard.
/// `knot_spec` is informational and dropped.
pub(crate) fn lower_rational_bspline_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRationalBSplineSurface,
) -> Result<(), ConvertError> {
    let u_degree = u32::try_from(early.u_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "u_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let v_degree = u32::try_from(early.v_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "v_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if early.weights_data.len() != early.control_points_list.len() {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "weights_data",
            expected: early.control_points_list.len(),
            actual: early.weights_data.len(),
        });
    }
    for (w_row, cp_row) in early
        .weights_data
        .iter()
        .zip(early.control_points_list.iter())
    {
        if w_row.len() != cp_row.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_row.len(),
                actual: w_row.len(),
            });
        }
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for row in &early.control_points_list {
        let mut pt_row = Vec::with_capacity(row.len());
        for &r in row {
            pt_row.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
        }
        control_points.push(pt_row);
    }
    let surface = NurbsSurface {
        u_degree,
        v_degree,
        control_points,
        kind: NurbsSurfaceKind::Rational {
            weights: early.weights_data.clone(),
        },
        u_knot_multiplicities: early.u_multiplicities.clone(),
        v_knot_multiplicities: early.v_multiplicities.clone(),
        u_knots: early.u_knots.clone(),
        v_knots: early.v_knots.clone(),
        u_closed: early.u_closed,
        v_closed: early.v_closed,
        form: early.surface_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.surfaces.push(Surface::Nurbs(surface));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one rational `QUASI_UNIFORM_CURVE` complex (`read_only`). Like
/// `lower_rational_bspline_curve` but knots/multiplicities are derived from
/// `(degree, cp_count)` via `quasi_uniform_knots` (no `B_SPLINE_CURVE_WITH_KNOTS`
/// part). No 2D guard (matching the legacy handler).
pub(crate) fn lower_rational_quasi_uniform_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRationalQuasiUniformCurve,
) -> Result<(), ConvertError> {
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if early.weights_data.len() != early.control_points_list.len() {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "weights_data",
            expected: early.control_points_list.len(),
            actual: early.weights_data.len(),
        });
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        control_points.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
    }
    let Some((knot_multiplicities, knots)) = quasi_uniform_knots(degree, control_points.len())
    else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: degree as usize + 1,
            actual: control_points.len(),
        });
    };
    let curve = NurbsCurve {
        degree,
        control_points,
        kind: NurbsKind::Rational {
            weights: early.weights_data.clone(),
        },
        knot_multiplicities,
        knots,
        closed: early.closed_curve,
        form: early.curve_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.curves.push(Curve::Nurbs(curve));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one rational `QUASI_UNIFORM_SURFACE` complex (`read_only`). u/v knots
/// derived per direction; `weights_data` grid validated against the cp grid.
pub(crate) fn lower_rational_quasi_uniform_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRationalQuasiUniformSurface,
) -> Result<(), ConvertError> {
    let u_degree = u32::try_from(early.u_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "u_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    let v_degree = u32::try_from(early.v_degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "v_degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if early.weights_data.len() != early.control_points_list.len() {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "weights_data",
            expected: early.control_points_list.len(),
            actual: early.weights_data.len(),
        });
    }
    for (w_row, cp_row) in early
        .weights_data
        .iter()
        .zip(early.control_points_list.iter())
    {
        if w_row.len() != cp_row.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_row.len(),
                actual: w_row.len(),
            });
        }
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for row in &early.control_points_list {
        let mut pt_row = Vec::with_capacity(row.len());
        for &r in row {
            pt_row.push(ctx.resolve_point(entity_id, r, "control_points_list")?);
        }
        control_points.push(pt_row);
    }
    let u_cp_count = control_points.len();
    let v_cp_count = control_points.first().map_or(0, Vec::len);
    let Some((u_knot_multiplicities, u_knots)) = quasi_uniform_knots(u_degree, u_cp_count) else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: u_degree as usize + 1,
            actual: u_cp_count,
        });
    };
    let Some((v_knot_multiplicities, v_knots)) = quasi_uniform_knots(v_degree, v_cp_count) else {
        return Err(ConvertError::DimensionMismatch {
            entity_id,
            field_name: "control_points_list",
            expected: v_degree as usize + 1,
            actual: v_cp_count,
        });
    };
    let surface = NurbsSurface {
        u_degree,
        v_degree,
        control_points,
        kind: NurbsSurfaceKind::Rational {
            weights: early.weights_data.clone(),
        },
        u_knot_multiplicities,
        v_knot_multiplicities,
        u_knots,
        v_knots,
        u_closed: early.u_closed,
        v_closed: early.v_closed,
        form: early.surface_form,
        self_intersect: early.self_intersect,
    };
    let id = ctx.geometry.surfaces.push(Surface::Nurbs(surface));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one 3D `CARTESIAN_POINT`. A 2-coordinate point belongs to the 2D
/// sister arena (no-op here); anything but 2 or 3 is malformed (Err, matching
/// the legacy handler).
pub(crate) fn lower_cartesian_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCartesianPoint,
) -> Result<(), ConvertError> {
    let coords = early.coordinates;
    match coords.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("CARTESIAN_POINT must have 2 or 3 coordinates, got {n}"),
            });
        }
    }
    let id = ctx.geometry.points.push(Point3 {
        x: coords[0],
        y: coords[1],
        z: coords[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one 3D `DIRECTION` (same 2D/3D self-claim shape as the point).
pub(crate) fn lower_direction(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDirection,
) -> Result<(), ConvertError> {
    let ratios = early.direction_ratios;
    match ratios.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("DIRECTION must have 2 or 3 direction_ratios, got {n}"),
            });
        }
    }
    let id = ctx.geometry.directions.push(Direction3 {
        x: ratios[0],
        y: ratios[1],
        z: ratios[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower the 2D-arena variant of `CARTESIAN_POINT` (sister of
/// [`lower_cartesian_point`]). Claims exactly 2-coordinate points into
/// `points_2d`; other dimensions are left for the 3D sister (silent skip).
pub(crate) fn lower_cartesian_point_2d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCartesianPoint,
) {
    let coords = early.coordinates;
    if coords.len() != 2 {
        return;
    }
    let id = ctx.geometry.points_2d.push(Point2 {
        x: coords[0],
        y: coords[1],
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the 2D-arena variant of `DIRECTION` (sister of [`lower_direction`]).
/// Claims exactly 2-component directions into `directions_2d`.
pub(crate) fn lower_direction_2d(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDirection) {
    let ratios = early.direction_ratios;
    if ratios.len() != 2 {
        return;
    }
    let id = ctx.geometry.directions_2d.push(Direction2 {
        x: ratios[0],
        y: ratios[1],
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the 2D-arena variant of `VECTOR` (sister of [`lower_vector`]). Claims
/// vectors whose `orientation` is a 2D direction into the `vector_2d_map`
/// side-cache; 3D-oriented vectors are left for the 3D sister (no-op).
pub(crate) fn lower_vector_2d(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyVector) {
    if let Some(dir) = ctx
        .id_cache
        .get::<crate::ir::id::Direction2dId>(early.orientation)
    {
        ctx.vector_2d_map.insert(entity_id, (dir, early.magnitude));
    }
}

/// Lower one `AXIS2_PLACEMENT_2D` (distinct STEP name). Claims a 2D `location`
/// into `placements_2d`; a 3D location is left for the 3D placement sister.
/// `ref_direction` (optional) must resolve to a 2D direction when present
/// (a dangling ref surfaces as `MissingReference`, matching the legacy handler).
pub(crate) fn lower_axis2_placement_2d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAxis2Placement2d,
) -> Result<(), ConvertError> {
    let Some(location) = ctx.id_cache.get::<crate::ir::id::Point2dId>(early.location) else {
        return Ok(());
    };
    let ref_direction = match early.ref_direction {
        Some(r) => Some(ctx.id_cache.get::<crate::ir::id::Direction2dId>(r).ok_or(
            ConvertError::MissingReference {
                from: entity_id,
                to: r,
                field_name: "ref_direction",
            },
        )?),
        None => None,
    };
    let id = ctx.geometry.placements_2d.push(Axis2Placement2d {
        location,
        ref_direction,
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower the 2D-arena variant of `CIRCLE` (sister of [`lower_circle`]). Claims
/// circles whose `position` resolved into `placements_2d`; a 3D placement is
/// left for the 3D sister (no-op).
pub(crate) fn lower_circle_2d(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyCircle) {
    let Some(position) = ctx
        .id_cache
        .get::<crate::ir::id::Placement2dId>(early.position)
    else {
        return;
    };
    let id = ctx.geometry.curves_2d.push(Curve2d::Circle(Circle2 {
        position,
        radius: early.radius,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the 2D-arena variant of `ELLIPSE` (sister of [`lower_ellipse`]). Claims
/// ellipses whose `position` resolved into `placements_2d`; a 3D placement is
/// left for the 3D sister (no-op).
pub(crate) fn lower_ellipse_2d(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyEllipse) {
    let Some(position) = ctx
        .id_cache
        .get::<crate::ir::id::Placement2dId>(early.position)
    else {
        return;
    };
    let id = ctx.geometry.curves_2d.push(Curve2d::Ellipse(Ellipse2 {
        position,
        semi_axis_1: early.semi_axis_1,
        semi_axis_2: early.semi_axis_2,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the 2D-arena variant of `LINE` (sister of [`lower_line`]). Claims lines
/// whose `pnt` resolved into `points_2d`; a 3D point is left for the 3D sister
/// (no-op). The `dir` VECTOR must be in `vector_2d_map` (a dangling ref surfaces
/// as `MissingReference`, matching the legacy handler).
pub(crate) fn lower_line_2d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLine,
) -> Result<(), ConvertError> {
    let Some(point) = ctx.id_cache.get::<crate::ir::id::Point2dId>(early.pnt) else {
        return Ok(());
    };
    let (direction, magnitude) =
        *ctx.vector_2d_map
            .get(&early.dir)
            .ok_or(ConvertError::MissingReference {
                from: entity_id,
                to: early.dir,
                field_name: "dir",
            })?;
    let id = ctx.geometry.curves_2d.push(Curve2d::Line(Line2 {
        point,
        direction,
        magnitude,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower the 2D-arena variant of `POLYLINE` (sister of [`lower_polyline`]). The
/// first point's arena discriminates: 2D points claim here, 3D fall through to
/// the 3D sister. A dangling point ref surfaces as `MissingReference`.
pub(crate) fn lower_polyline_2d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPolyline,
) -> Result<(), ConvertError> {
    let Some(first) = early.points.first() else {
        return Ok(());
    };
    if !ctx.id_cache.contains::<crate::ir::id::Point2dId>(*first) {
        return Ok(());
    }
    let mut points = Vec::with_capacity(early.points.len());
    for r in &early.points {
        let Some(pid) = ctx.id_cache.get::<crate::ir::id::Point2dId>(*r) else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: *r,
                field_name: "points",
            });
        };
        points.push(pid);
    }
    let id = ctx
        .geometry
        .curves_2d
        .push(Curve2d::Polyline(Polyline2d { points }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower the 2D-arena variant of `B_SPLINE_CURVE_WITH_KNOTS` (non-rational;
/// sister of [`lower_b_spline_curve_with_knots`]). `degree` narrows to `u32`
/// first; then the first control point's arena discriminates (a 3D point falls
/// through to the 3D sister). Once 2D is confirmed, a missing successor is an
/// error. `self_intersect`/`knot_spec` are not modelled by `NurbsCurve2d` (the
/// writer re-emits `.F.` / `UNSPECIFIED`).
pub(crate) fn lower_b_spline_curve_2d_with_knots(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyBSplineCurveWithKnots,
) -> Result<(), ConvertError> {
    let degree = u32::try_from(early.degree).map_err(|_| ConvertError::AttributeType {
        entity_id,
        field_name: "degree",
        expected: "non-negative Integer",
        actual: crate::ir::error::AttributeKindTag::Integer,
    })?;
    if let Some(&first) = early.control_points_list.first()
        && !ctx.id_cache.contains::<crate::ir::id::Point2dId>(first)
    {
        return Ok(()); // 3D sister handler claims this instance
    }
    let mut control_points = Vec::with_capacity(early.control_points_list.len());
    for &r in &early.control_points_list {
        let pt = ctx.id_cache.get::<crate::ir::id::Point2dId>(r).ok_or(
            ConvertError::MissingReference {
                from: entity_id,
                to: r,
                field_name: "control_points_list",
            },
        )?;
        control_points.push(pt);
    }
    let id = ctx.geometry.curves_2d.push(Curve2d::Nurbs(NurbsCurve2d {
        degree,
        control_points,
        kind: NurbsKind2d::NonRational,
        knot_multiplicities: early.knot_multiplicities.clone(),
        knots: early.knots.clone(),
        closed: early.closed_curve,
        form: early.curve_form,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `VERTEX_POINT` (resolves `vertex_geometry` through the shared
/// point resolver; a dangling ref surfaces as `MissingReference`). Registers
/// in the `vertex_map` named cache (not `id_cache`), as the legacy handler did.
pub(crate) fn lower_vertex_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVertexPoint,
) -> Result<(), ConvertError> {
    let point = ctx.resolve_point(entity_id, early.vertex_geometry, "vertex_geometry")?;
    let id = ctx.geometry.vertices.push(Vertex { point });
    ctx.vertex_map.insert(entity_id, id);
    Ok(())
}

/// Lower one `VECTOR` (2D sister claims a 2D-direction orientation; a 3D
/// orientation resolves to a `DirectionId`, stored in the `vector_map` named
/// cache as the legacy handler did).
pub(crate) fn lower_vector(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVector,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Direction2dId>(early.orientation)
    {
        return Ok(());
    }
    let dir_id = ctx.resolve_direction(entity_id, early.orientation, "orientation")?;
    ctx.vector_map.insert(entity_id, (dir_id, early.magnitude));
    Ok(())
}

/// Lower one `LINE` (2D sister claims a 2D-point pnt; otherwise resolve the
/// point and the VECTOR ref into `Line3`).
pub(crate) fn lower_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLine,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::Point2dId>(early.pnt) {
        return Ok(());
    }
    let point = ctx.resolve_point(entity_id, early.pnt, "pnt")?;
    let (direction, magnitude) = ctx.resolve_vector(entity_id, early.dir, "dir")?;
    let id = ctx.geometry.curves.push(Curve::Line(Line3 {
        point,
        direction,
        magnitude,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `AXIS1_PLACEMENT` (location + axis direction). The schema types
/// `axis` OPTIONAL, but the L2 `Axis1Placement.axis` is required and every
/// corpus instance carries it; a missing axis is an Err (the legacy required
/// read errored too — no corpus file triggers it).
pub(crate) fn lower_axis1_placement(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAxis1Placement,
) -> Result<(), ConvertError> {
    let location = ctx.resolve_point(entity_id, early.location, "location")?;
    let Some(axis_ref) = early.axis else {
        return Err(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "AXIS1_PLACEMENT.axis is required".into(),
        });
    };
    let axis = ctx.resolve_direction(entity_id, axis_ref, "axis")?;
    let id = ctx
        .geometry
        .placements_1d
        .push(Axis1Placement { location, axis });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `AXIS2_PLACEMENT_3D` (location + optional `axis`/`ref_direction`; 2D
/// sister claims a 2D location). Stored in the `placement_map` named cache.
pub(crate) fn lower_axis2_placement_3d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAxis2Placement3d,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Point2dId>(early.location)
    {
        return Ok(());
    }
    let location = ctx.resolve_point(entity_id, early.location, "location")?;
    let axis = early
        .axis
        .map(|r| ctx.resolve_direction(entity_id, r, "axis"))
        .transpose()?;
    let ref_direction = early
        .ref_direction
        .map(|r| ctx.resolve_direction(entity_id, r, "ref_direction"))
        .transpose()?;
    let id = ctx.geometry.placements.push(Axis2Placement3d {
        location,
        axis,
        ref_direction,
    });
    ctx.placement_map.insert(entity_id, id);
    Ok(())
}

/// Lower one `CIRCLE` (placement + radius; 2D sister claims a 2D placement).
pub(crate) fn lower_circle(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCircle,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Circle(Circle3 {
        position,
        radius: early.radius,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANE` (placement → surface).
pub(crate) fn lower_plane(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlane,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `ELLIPSE` (placement + measures → `Curve::Ellipse`).
pub(crate) fn lower_ellipse(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyEllipse,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Ellipse(Ellipse3 {
        position,
        semi_axis_1: early.semi_axis_1,
        semi_axis_2: early.semi_axis_2,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PARABOLA` (placement + measures → `Curve::Parabola`).
pub(crate) fn lower_parabola(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyParabola,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Parabola(Parabola {
        position,
        focal_dist: early.focal_dist,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `HYPERBOLA` (placement + measures → `Curve::Hyperbola`).
pub(crate) fn lower_hyperbola(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyHyperbola,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Hyperbola(Hyperbola {
        position,
        semi_axis: early.semi_axis,
        semi_imag_axis: early.semi_imag_axis,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CONICAL_SURFACE` (placement + measures → `Surface::Cone`).
pub(crate) fn lower_conical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyConicalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.surfaces.push(Surface::Cone(ConicalSurface {
        position,
        radius: early.radius,
        semi_angle: early.semi_angle,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CYLINDRICAL_SURFACE` (placement + measures → `Surface::Cylinder`).
pub(crate) fn lower_cylindrical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCylindricalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Cylinder(CylindricalSurface {
            position,
            radius: early.radius,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `SPHERICAL_SURFACE` (placement + measures → `Surface::Sphere`).
pub(crate) fn lower_spherical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySphericalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Sphere(SphericalSurface {
            position,
            radius: early.radius,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `TOROIDAL_SURFACE` (placement + measures → `Surface::Torus`).
pub(crate) fn lower_toroidal_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyToroidalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.surfaces.push(Surface::Torus(ToroidalSurface {
        position,
        major_radius: early.major_radius,
        minor_radius: early.minor_radius,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `SURFACE_OF_REVOLUTION` (swept curve + axis1 placement).
pub(crate) fn lower_surface_of_revolution(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceOfRevolution,
) -> Result<(), ConvertError> {
    let swept_curve = ctx.resolve_curve(entity_id, early.swept_curve, "swept_curve")?;
    let axis_placement = ctx.resolve_axis1(entity_id, early.axis_position, "axis_position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Revolution(SurfaceOfRevolution {
            swept_curve,
            axis_placement,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `SURFACE_OF_LINEAR_EXTRUSION` (swept curve + extrusion VECTOR
/// resolved to direction + depth).
pub(crate) fn lower_surface_of_linear_extrusion(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceOfLinearExtrusion,
) -> Result<(), ConvertError> {
    let swept_curve = ctx.resolve_curve(entity_id, early.swept_curve, "swept_curve")?;
    let (extrusion_direction, depth) =
        ctx.resolve_vector(entity_id, early.extrusion_axis, "extrusion_axis")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Extrusion(SurfaceOfLinearExtrusion {
            swept_curve,
            extrusion_direction,
            depth,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `POLYLINE` (point list; 2D sister claims when the first point is
/// 2D). Each point resolves through the shared point resolver.
pub(crate) fn lower_polyline(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPolyline,
) -> Result<(), ConvertError> {
    if let Some(first) = early.points.first()
        && ctx.id_cache.contains::<crate::ir::id::Point2dId>(*first)
    {
        return Ok(());
    }
    let mut points = Vec::with_capacity(early.points.len());
    for r in &early.points {
        points.push(ctx.resolve_point(entity_id, *r, "points")?);
    }
    let id = ctx
        .geometry
        .curves
        .push(Curve::Polyline(Polyline { points }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANAR_EXTENT` (the base form; `PLANAR_BOX` stays on the legacy
/// path). `name` is preserved — the writer re-emits it.
pub(crate) fn lower_planar_extent(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlanarExtent,
) {
    let id = ctx
        .geometry
        .planar_extents
        .push(PlanarExtent::Itself(PlanarExtentData {
            name: early.name.clone(),
            size_in_x: early.size_in_x,
            size_in_y: early.size_in_y,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DEGENERATE_TOROIDAL_SURFACE` (`select_outer` BOOLEAN chooses the
/// outer/inner sheet). `name` is discarded (writer synthesises `''`).
pub(crate) fn lower_degenerate_toroidal_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDegenerateToroidalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::DegenerateToroidal(DegenerateToroidalSurface {
            position,
            major_radius: early.major_radius,
            minor_radius: early.minor_radius,
            select_outer: early.select_outer,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CIRCULAR_AREA` — orphan (no inbound refs, no `id_cache` entry).
/// `centre` probes the local `cartesian_point` then the external `REFERENCE`;
/// unresolved drops the carrier.
pub(crate) fn lower_circular_area(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: &EarlyCircularArea,
) {
    let centre = if let Some(point) = ctx.id_cache.get::<crate::ir::id::PointId>(early.centre) {
        CircularAreaCentre::Point(point)
    } else if let Some(ext) = ctx
        .id_cache
        .get::<crate::ir::id::ExternalRefId>(early.centre)
    {
        CircularAreaCentre::External(ext)
    } else {
        return;
    };
    ctx.geometry.circular_areas.push(CircularArea {
        name: early.name.clone(),
        centre,
        radius: early.radius,
    });
}

/// Lower one `CURVE_BOUNDED_SURFACE` (`bounded_surface` subtype). `boundaries`
/// narrows `boundary_curve` to generic `CurveId`; an unresolved basis or empty
/// boundary set drops the carrier.
pub(crate) fn lower_curve_bounded_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCurveBoundedSurface,
) {
    let Some(basis_surface) = ctx
        .id_cache
        .get::<crate::ir::id::SurfaceId>(early.basis_surface)
    else {
        return;
    };
    let boundaries: Vec<_> = early
        .boundaries
        .iter()
        .filter_map(|r| ctx.id_cache.get::<crate::ir::id::CurveId>(*r))
        .collect();
    if boundaries.is_empty() {
        return;
    }
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::CurveBounded(CurveBoundedSurface {
            name: early.name.clone(),
            basis_surface,
            boundaries,
            implicit_outer: early.implicit_outer,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `OFFSET_SURFACE` (wraps a basis surface; `self_intersect` LOGICAL
/// passes through unchanged — L2 keeps the full `Logical`). The dup-guard skips
/// entities already interned through another resolution path.
pub(crate) fn lower_offset_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyOffsetSurface,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::SurfaceId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_surface(entity_id, early.basis_surface, "basis_surface")?;
    let id = ctx.geometry.surfaces.push(Surface::Offset(SurfaceOfOffset {
        basis,
        distance: early.distance,
        self_intersect: early.self_intersect,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `OFFSET_CURVE_3D` (basis curve offset along `ref_direction`).
pub(crate) fn lower_offset_curve_3d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyOffsetCurve3d,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::CurveId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_curve(entity_id, early.basis_curve, "basis_curve")?;
    let ref_direction = ctx.resolve_direction(entity_id, early.ref_direction, "ref_direction")?;
    let id = ctx
        .geometry
        .curves
        .push(Curve::OffsetCurve3d(OffsetCurve3d {
            basis,
            distance: early.distance,
            self_intersect: early.self_intersect,
            ref_direction,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANAR_BOX` (`PlanarExtent::PlanarBox` variant). `placement` is
/// the `axis2_placement` SELECT — probe the 3D then 2D placement caches; an
/// unresolved placement warns and drops the carrier (verbatim). `name` preserved.
pub(crate) fn lower_planar_box(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyPlanarBox) {
    let placement = if let Some(&id) = ctx.placement_map.get(&early.placement) {
        PlanarBoxPlacement::Placement3d(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::Placement2dId>(early.placement)
    {
        PlanarBoxPlacement::Placement2d(id)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "PLANAR_BOX.placement #{} did not resolve to an AXIS2_PLACEMENT",
                early.placement
            ),
        });
        return;
    };
    let id = ctx
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: early.name.clone(),
            size_in_x: early.size_in_x,
            size_in_y: early.size_in_y,
            placement,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `RECTANGULAR_TRIMMED_SURFACE` (`bounded_surface` subtype:
/// parameter-space rectangle on a basis surface). Fields map by name from L1;
/// EXPRESS positional order is `u1,u2,v1,v2,usense,vsense` (the prior hand
/// handler transposed `usense` to slot 4 — fixed here via the generated bind).
pub(crate) fn lower_rectangular_trimmed_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRectangularTrimmedSurface,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::SurfaceId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_surface(entity_id, early.basis_surface, "basis_surface")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::RectangularTrimmed(RectangularTrimmedSurface {
            basis,
            u1: early.u1,
            u2: early.u2,
            usense: early.usense,
            v1: early.v1,
            v2: early.v2,
            vsense: early.vsense,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `COMPOSITE_CURVE_SEGMENT` — map-only (no arena/`id_cache`). Records
/// the segment metadata (raw `parent_curve` ref) in `composite_segment_map`; the
/// owning `COMPOSITE_CURVE` assembles `CompositeSegment` values from it.
pub(crate) fn lower_composite_curve_segment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCompositeCurveSegment,
) {
    ctx.composite_segment_map.insert(
        entity_id,
        (early.transition, early.same_sense, early.parent_curve),
    );
}

/// Lower one `COMPOSITE_CURVE`. Each segment ref resolves through
/// `composite_segment_map` (populated by the segment handler, dispatched first);
/// a missing entry is `MissingReference` (matching the legacy handler).
pub(crate) fn lower_composite_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCompositeCurve,
) -> Result<(), ConvertError> {
    let mut segments = Vec::with_capacity(early.segments.len());
    for seg_ref in &early.segments {
        let Some(&(transition, same_sense, parent_step_id)) =
            ctx.composite_segment_map.get(seg_ref)
        else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: *seg_ref,
                field_name: "segments",
            });
        };
        let parent_curve = ctx.resolve_curve(entity_id, parent_step_id, "parent_curve")?;
        segments.push(CompositeSegment {
            transition,
            same_sense,
            parent_curve,
        });
    }
    let id = ctx.geometry.curves.push(Curve::Composite(CompositeCurve {
        segments,
        self_intersect: early.self_intersect,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `TRIMMED_CURVE`. `trim_1`/`trim_2` map each `EarlyTrimSelect`: a
/// `Point` ref resolves through `id_cache` (unresolved → silently dropped, as the
/// legacy handler did); a `Param` real passes through. `master_representation`
/// (the `trimming_preference` enum) is carried unchanged.
pub(crate) fn lower_trimmed_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyTrimmedCurve,
) -> Result<(), ConvertError> {
    let basis = ctx.resolve_curve(entity_id, early.basis_curve, "basis_curve")?;
    let trim_1 = lower_trim_select(ctx, &early.trim_1);
    let trim_2 = lower_trim_select(ctx, &early.trim_2);
    let id = ctx.geometry.curves.push(Curve::Trimmed(TrimmedCurve {
        basis,
        trim_1,
        trim_2,
        sense_agreement: early.sense_agreement,
        master: early.master_representation,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Resolve a `TRIMMED_CURVE` trim slot's `EarlyTrimSelect` list into `TrimSelect`s
/// (unresolved point refs dropped).
fn lower_trim_select(ctx: &ReaderContext, items: &[EarlyTrimSelect]) -> Vec<TrimSelect> {
    items
        .iter()
        .filter_map(|s| match *s {
            EarlyTrimSelect::Point(r) => ctx
                .id_cache
                .get::<crate::ir::id::PointId>(r)
                .map(TrimSelect::Point),
            EarlyTrimSelect::Param(v) => Some(TrimSelect::Param(v)),
        })
        .collect()
}

/// Shared `surface_curve` subtype lowering: resolve `curve_3d` (a `CurveId`)
/// and keep only the `surface` branch of `associated_geometry` (`pcurve`
/// members + unresolved refs drop; the `master_representation` enum is already
/// decoded by `bind`). An unresolved `curve_3d` or an empty surface list drops
/// the curve — symmetric on re-read, a verbatim port of the legacy
/// `read_surface_curve_body`.
fn lower_surface_curve_data(
    ctx: &ReaderContext,
    name: String,
    curve_3d_ref: u64,
    assoc_refs: &[u64],
    master_representation: PreferredSurfaceCurveRepresentation,
) -> Option<SurfaceCurveData> {
    let curve_3d = ctx.id_cache.get::<crate::ir::id::CurveId>(curve_3d_ref)?;
    let associated_geometry: Vec<_> = assoc_refs
        .iter()
        .filter_map(|r| {
            ctx.id_cache
                .get::<crate::ir::id::SurfaceId>(*r)
                .map(PCurveOrSurface::Surface)
        })
        .collect();
    if associated_geometry.is_empty() {
        return None;
    }
    Some(SurfaceCurveData {
        name,
        curve_3d,
        associated_geometry,
        master_representation,
    })
}

/// Lower one `INTERSECTION_CURVE` into the shared `surface_curves` arena.
pub(crate) fn lower_intersection_curve(ctx: &mut ReaderContext, early: EarlyIntersectionCurve) {
    if let Some(body) = lower_surface_curve_data(
        ctx,
        early.name,
        early.curve_3d,
        &early.associated_geometry,
        early.master_representation,
    ) {
        ctx.geometry
            .surface_curves
            .push(SurfaceCurve::IntersectionCurve(body));
    }
}

/// Lower one `BOUNDED_SURFACE_CURVE` into the shared `surface_curves` arena.
pub(crate) fn lower_bounded_surface_curve(
    ctx: &mut ReaderContext,
    early: EarlyBoundedSurfaceCurve,
) {
    if let Some(body) = lower_surface_curve_data(
        ctx,
        early.name,
        early.curve_3d,
        &early.associated_geometry,
        early.master_representation,
    ) {
        ctx.geometry
            .surface_curves
            .push(SurfaceCurve::BoundedSurfaceCurve(body));
    }
}
