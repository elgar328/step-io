//! Geometry-domain `lift` fns (W-RECURSIVE pilot). See the [module docs](super)
//! for the lifting contract. The legacy writer synthesised an empty `name`
//! (`String::new()`), so these lifts always set `name: String::new()`.

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement2d, EarlyAxis2Placement3d, EarlyBSplineCurveWithKnots,
    EarlyBSplineSurfaceWithKnots, EarlyBoundedPcurve, EarlyBoundedSurfaceCurve,
    EarlyCartesianPoint, EarlyCircle, EarlyCircularArea, EarlyCompositeCurve,
    EarlyCompositeCurveSegment, EarlyConicalSurface, EarlyCurveBoundedSurface,
    EarlyCylindricalSurface, EarlyDegenerateToroidalSurface, EarlyDirection, EarlyEllipse,
    EarlyHyperbola, EarlyIntersectionCurve, EarlyKnotType, EarlyLine, EarlyOffsetCurve3d,
    EarlyOffsetSurface, EarlyParabola, EarlyPlanarBox, EarlyPlanarExtent, EarlyPlane,
    EarlyPolyline, EarlyRationalBSplineCurve, EarlyRationalBSplineSurface,
    EarlyRectangularTrimmedSurface, EarlySphericalSurface, EarlySurfaceOfLinearExtrusion,
    EarlySurfaceOfRevolution, EarlyToroidalSurface, EarlyTrimSelect, EarlyTrimmedCurve,
    EarlyVector, EarlyVertexPoint,
};
use crate::ir::geometry::{
    Direction2, Direction3, Logical, NurbsCurve, NurbsCurve2d, NurbsSurface, PCurveOrSurface,
    Point2, Point3, SurfaceCurveData, TransitionCode, TrimMaster,
};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Lift one `BOUNDED_PCURVE`. Unlike most geometry, it round-trips its own
/// `name` (the legacy writer emitted `BoundedPCurve.name`), so it is threaded
/// through rather than synthesised empty.
pub(crate) fn lift_bounded_pcurve(
    name: String,
    basis_surface: u64,
    reference_to_curve: u64,
) -> EarlyBoundedPcurve {
    EarlyBoundedPcurve {
        name,
        basis_surface,
        reference_to_curve,
    }
}

/// Lift one `B_SPLINE_CURVE_WITH_KNOTS` (non-rational). `control_points_list` =
/// emitted `CARTESIAN_POINT` step ids. `knot_spec` is always re-emitted as
/// `Unspecified` (the IR does not track the label; see plan).
pub(crate) fn lift_b_spline_curve_with_knots(
    nurbs: &NurbsCurve,
    control_points_list: Vec<u64>,
) -> EarlyBSplineCurveWithKnots {
    EarlyBSplineCurveWithKnots {
        name: String::new(),
        degree: i64::from(nurbs.degree),
        control_points_list,
        curve_form: nurbs.form,
        closed_curve: nurbs.closed,
        self_intersect: nurbs.self_intersect,
        knot_multiplicities: nurbs.knot_multiplicities.clone(),
        knots: nurbs.knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
    }
}

/// Lift the 2D-arena variant of `B_SPLINE_CURVE_WITH_KNOTS` (non-rational;
/// sister of [`lift_b_spline_curve_with_knots`]). `NurbsCurve2d` does not model
/// `self_intersect`, so it is re-emitted as `.F.` (`Logical::False`), matching
/// the legacy handler; `knot_spec` is always `Unspecified`.
pub(crate) fn lift_b_spline_curve_2d_with_knots(
    nurbs: &NurbsCurve2d,
    control_points_list: Vec<u64>,
) -> EarlyBSplineCurveWithKnots {
    EarlyBSplineCurveWithKnots {
        name: String::new(),
        degree: i64::from(nurbs.degree),
        control_points_list,
        curve_form: nurbs.form,
        closed_curve: nurbs.closed,
        self_intersect: Logical::False,
        knot_multiplicities: nurbs.knot_multiplicities.clone(),
        knots: nurbs.knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
    }
}

/// Lift one `B_SPLINE_SURFACE_WITH_KNOTS` (non-rational). `control_points_list` =
/// emitted `CARTESIAN_POINT` step ids in the u/v grid. `knot_spec` is always
/// re-emitted as `Unspecified` (the IR does not track the label; see plan).
pub(crate) fn lift_b_spline_surface_with_knots(
    nurbs: &NurbsSurface,
    control_points_list: Vec<Vec<u64>>,
) -> EarlyBSplineSurfaceWithKnots {
    EarlyBSplineSurfaceWithKnots {
        name: String::new(),
        u_degree: i64::from(nurbs.u_degree),
        v_degree: i64::from(nurbs.v_degree),
        control_points_list,
        surface_form: nurbs.form,
        u_closed: nurbs.u_closed,
        v_closed: nurbs.v_closed,
        self_intersect: nurbs.self_intersect,
        u_multiplicities: nurbs.u_knot_multiplicities.clone(),
        v_multiplicities: nurbs.v_knot_multiplicities.clone(),
        u_knots: nurbs.u_knots.clone(),
        v_knots: nurbs.v_knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
    }
}

/// Lift one `RATIONAL_B_SPLINE_CURVE` (complex). `control_points` = emitted
/// `CARTESIAN_POINT` step ids; `weights` = the rational weights (the handler
/// extracts them from `NurbsKind::Rational`). `knot_spec` is always re-emitted
/// as `Unspecified` (see plan).
pub(crate) fn lift_rational_bspline_curve(
    nurbs: &NurbsCurve,
    control_points: Vec<u64>,
    weights: Vec<f64>,
) -> EarlyRationalBSplineCurve {
    EarlyRationalBSplineCurve {
        name: String::new(),
        degree: i64::from(nurbs.degree),
        control_points_list: control_points,
        curve_form: nurbs.form,
        closed_curve: nurbs.closed,
        self_intersect: nurbs.self_intersect,
        knot_multiplicities: nurbs.knot_multiplicities.clone(),
        knots: nurbs.knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
        weights_data: weights,
    }
}

/// Lift the 2D-arena variant of `RATIONAL_B_SPLINE_CURVE` (complex; sister of
/// [`lift_rational_bspline_curve`]). `NurbsCurve2d` does not model
/// `self_intersect`, so it is re-emitted as `.F.` (`Logical::False`), matching the
/// legacy handler; `knot_spec` is always `Unspecified`.
pub(crate) fn lift_rational_bspline_curve_2d(
    nurbs: &NurbsCurve2d,
    control_points: Vec<u64>,
    weights: Vec<f64>,
) -> EarlyRationalBSplineCurve {
    EarlyRationalBSplineCurve {
        name: String::new(),
        degree: i64::from(nurbs.degree),
        control_points_list: control_points,
        curve_form: nurbs.form,
        closed_curve: nurbs.closed,
        self_intersect: Logical::False,
        knot_multiplicities: nurbs.knot_multiplicities.clone(),
        knots: nurbs.knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
        weights_data: weights,
    }
}

/// Lift one `RATIONAL_B_SPLINE_SURFACE` (complex). `control_points`/`weights` are
/// the u/v grids (the handler extracts weights from `NurbsSurfaceKind::Rational`).
/// `knot_spec` is always re-emitted as `Unspecified` (see plan).
pub(crate) fn lift_rational_bspline_surface(
    nurbs: &NurbsSurface,
    control_points: Vec<Vec<u64>>,
    weights: Vec<Vec<f64>>,
) -> EarlyRationalBSplineSurface {
    EarlyRationalBSplineSurface {
        name: String::new(),
        u_degree: i64::from(nurbs.u_degree),
        v_degree: i64::from(nurbs.v_degree),
        control_points_list: control_points,
        surface_form: nurbs.form,
        u_closed: nurbs.u_closed,
        v_closed: nurbs.v_closed,
        self_intersect: nurbs.self_intersect,
        u_multiplicities: nurbs.u_knot_multiplicities.clone(),
        v_multiplicities: nurbs.v_knot_multiplicities.clone(),
        u_knots: nurbs.u_knots.clone(),
        v_knots: nurbs.v_knots.clone(),
        knot_spec: EarlyKnotType::Unspecified,
        weights_data: weights,
    }
}

/// Lift one `CARTESIAN_POINT` from its arena `Point3`.
pub(crate) fn lift_cartesian_point(p: Point3) -> EarlyCartesianPoint {
    EarlyCartesianPoint {
        name: String::new(),
        coordinates: vec![p.x, p.y, p.z],
    }
}

/// Lift one `DIRECTION` from its arena `Direction3`.
pub(crate) fn lift_direction(d: Direction3) -> EarlyDirection {
    EarlyDirection {
        name: String::new(),
        direction_ratios: vec![d.x, d.y, d.z],
    }
}

/// Lift the 2D-arena `CARTESIAN_POINT` from its `Point2` (sister of
/// [`lift_cartesian_point`]); reuses the shared `serialize_cartesian_point`.
pub(crate) fn lift_cartesian_point_2d(p: Point2) -> EarlyCartesianPoint {
    EarlyCartesianPoint {
        name: String::new(),
        coordinates: vec![p.x, p.y],
    }
}

/// Lift the 2D-arena `DIRECTION` from its `Direction2` (sister of
/// [`lift_direction`]); reuses the shared `serialize_direction`.
pub(crate) fn lift_direction_2d(d: Direction2) -> EarlyDirection {
    EarlyDirection {
        name: String::new(),
        direction_ratios: vec![d.x, d.y],
    }
}

/// Lift the 2D `AXIS2_PLACEMENT_2D` from pre-emitted child step ids (mirror of
/// [`lift_axis2_placement_3d`] minus `axis`). `vector_2d` reuses `lift_vector`.
pub(crate) fn lift_axis2_placement_2d(
    location: u64,
    ref_direction: Option<u64>,
) -> EarlyAxis2Placement2d {
    EarlyAxis2Placement2d {
        name: String::new(),
        location,
        ref_direction,
    }
}

/// Lift one `VERTEX_POINT` (the child point's step id is pre-resolved by the
/// handler's `emit_point` recursion).
pub(crate) fn lift_vertex_point(vertex_geometry: u64) -> EarlyVertexPoint {
    EarlyVertexPoint {
        name: String::new(),
        vertex_geometry,
    }
}

/// Lift one `VECTOR` (orientation = child direction's output step id).
pub(crate) fn lift_vector(orientation: u64, magnitude: f64) -> EarlyVector {
    EarlyVector {
        name: String::new(),
        orientation,
        magnitude,
    }
}

/// Lift one `LINE` (pnt/dir = child point/VECTOR output step ids).
pub(crate) fn lift_line(pnt: u64, dir: u64) -> EarlyLine {
    EarlyLine {
        name: String::new(),
        pnt,
        dir,
    }
}

/// Lift one `AXIS1_PLACEMENT` (location/axis = child output step ids; axis is
/// always present in practice, so `Some`).
pub(crate) fn lift_axis1_placement(location: u64, axis: u64) -> EarlyAxis1Placement {
    EarlyAxis1Placement {
        name: String::new(),
        location,
        axis: Some(axis),
    }
}

/// Lift one `AXIS2_PLACEMENT_3D` (optional `axis`/`ref_direction` pass through;
/// `None` → `$`).
pub(crate) fn lift_axis2_placement_3d(
    location: u64,
    axis: Option<u64>,
    ref_direction: Option<u64>,
) -> EarlyAxis2Placement3d {
    EarlyAxis2Placement3d {
        name: String::new(),
        location,
        axis,
        ref_direction,
    }
}

/// Lift one `CIRCLE` (position = child placement step id).
pub(crate) fn lift_circle(position: u64, radius: f64) -> EarlyCircle {
    EarlyCircle {
        name: String::new(),
        position,
        radius,
    }
}

/// Lift one `PLANE` (position = child placement step id).
pub(crate) fn lift_plane(position: u64) -> EarlyPlane {
    EarlyPlane {
        name: String::new(),
        position,
    }
}

/// Lift one `ELLIPSE` (position = child placement step id).
pub(crate) fn lift_ellipse(position: u64, semi_axis_1: f64, semi_axis_2: f64) -> EarlyEllipse {
    EarlyEllipse {
        name: String::new(),
        position,
        semi_axis_1,
        semi_axis_2,
    }
}

/// Lift one `PARABOLA` (position = child placement step id).
pub(crate) fn lift_parabola(position: u64, focal_dist: f64) -> EarlyParabola {
    EarlyParabola {
        name: String::new(),
        position,
        focal_dist,
    }
}

/// Lift one `HYPERBOLA` (position = child placement step id).
pub(crate) fn lift_hyperbola(position: u64, semi_axis: f64, semi_imag_axis: f64) -> EarlyHyperbola {
    EarlyHyperbola {
        name: String::new(),
        position,
        semi_axis,
        semi_imag_axis,
    }
}

/// Lift one `CONICAL_SURFACE` (position = child placement step id).
pub(crate) fn lift_conical_surface(
    position: u64,
    radius: f64,
    semi_angle: f64,
) -> EarlyConicalSurface {
    EarlyConicalSurface {
        name: String::new(),
        position,
        radius,
        semi_angle,
    }
}

/// Lift one `CYLINDRICAL_SURFACE` (position = child placement step id).
pub(crate) fn lift_cylindrical_surface(position: u64, radius: f64) -> EarlyCylindricalSurface {
    EarlyCylindricalSurface {
        name: String::new(),
        position,
        radius,
    }
}

/// Lift one `SPHERICAL_SURFACE` (position = child placement step id).
pub(crate) fn lift_spherical_surface(position: u64, radius: f64) -> EarlySphericalSurface {
    EarlySphericalSurface {
        name: String::new(),
        position,
        radius,
    }
}

/// Lift one `TOROIDAL_SURFACE` (position = child placement step id).
pub(crate) fn lift_toroidal_surface(
    position: u64,
    major_radius: f64,
    minor_radius: f64,
) -> EarlyToroidalSurface {
    EarlyToroidalSurface {
        name: String::new(),
        position,
        major_radius,
        minor_radius,
    }
}

/// Lift one `SURFACE_OF_REVOLUTION` (swept/axis = child output step ids).
pub(crate) fn lift_surface_of_revolution(
    swept_curve: u64,
    axis_position: u64,
) -> EarlySurfaceOfRevolution {
    EarlySurfaceOfRevolution {
        name: String::new(),
        swept_curve,
        axis_position,
    }
}

/// Lift one `SURFACE_OF_LINEAR_EXTRUSION` (swept curve + extrusion VECTOR
/// step ids).
pub(crate) fn lift_surface_of_linear_extrusion(
    swept_curve: u64,
    extrusion_axis: u64,
) -> EarlySurfaceOfLinearExtrusion {
    EarlySurfaceOfLinearExtrusion {
        name: String::new(),
        swept_curve,
        extrusion_axis,
    }
}

/// Lift one `POLYLINE` (points = child point output step ids).
pub(crate) fn lift_polyline(points: Vec<u64>) -> EarlyPolyline {
    EarlyPolyline {
        name: String::new(),
        points,
    }
}

/// Lift one `PLANAR_EXTENT` (base form). Unlike the synthesised-name leaves
/// above, `name` is preserved from the L2 `PlanarExtentData`.
pub(crate) fn lift_planar_extent(
    name: String,
    size_in_x: f64,
    size_in_y: f64,
) -> EarlyPlanarExtent {
    EarlyPlanarExtent {
        name,
        size_in_x,
        size_in_y,
    }
}

/// Lift one `DEGENERATE_TOROIDAL_SURFACE` (position = child placement step id).
pub(crate) fn lift_degenerate_toroidal_surface(
    position: u64,
    major_radius: f64,
    minor_radius: f64,
    select_outer: bool,
) -> EarlyDegenerateToroidalSurface {
    EarlyDegenerateToroidalSurface {
        name: String::new(),
        position,
        major_radius,
        minor_radius,
        select_outer,
    }
}

/// Lift one `CIRCULAR_AREA` (centre = resolved point/external step id). `name`
/// is preserved.
pub(crate) fn lift_circular_area(name: String, centre: u64, radius: f64) -> EarlyCircularArea {
    EarlyCircularArea {
        name,
        centre,
        radius,
    }
}

/// Lift one `CURVE_BOUNDED_SURFACE` (basis = child surface step id, boundaries
/// = child curve output step ids). `name` is preserved.
pub(crate) fn lift_curve_bounded_surface(
    name: String,
    basis_surface: u64,
    boundaries: Vec<u64>,
    implicit_outer: bool,
) -> EarlyCurveBoundedSurface {
    EarlyCurveBoundedSurface {
        name,
        basis_surface,
        boundaries,
        implicit_outer,
    }
}

/// Lift one `OFFSET_SURFACE` (basis = child surface step id). `self_intersect`
/// (`Logical`) passes through unchanged.
pub(crate) fn lift_offset_surface(
    basis_surface: u64,
    distance: f64,
    self_intersect: Logical,
) -> EarlyOffsetSurface {
    EarlyOffsetSurface {
        name: String::new(),
        basis_surface,
        distance,
        self_intersect,
    }
}

/// Lift one `OFFSET_CURVE_3D` (basis = child curve step id, `ref_direction` =
/// child direction step id).
pub(crate) fn lift_offset_curve_3d(
    basis_curve: u64,
    distance: f64,
    self_intersect: Logical,
    ref_direction: u64,
) -> EarlyOffsetCurve3d {
    EarlyOffsetCurve3d {
        name: String::new(),
        basis_curve,
        distance,
        self_intersect,
        ref_direction,
    }
}

/// Lift one `PLANAR_BOX` (placement = child `AXIS2_PLACEMENT` step id). `name`
/// is preserved.
pub(crate) fn lift_planar_box(
    name: String,
    size_in_x: f64,
    size_in_y: f64,
    placement: u64,
) -> EarlyPlanarBox {
    EarlyPlanarBox {
        name,
        size_in_x,
        size_in_y,
        placement,
    }
}

/// Lift one `RECTANGULAR_TRIMMED_SURFACE` (basis = child surface step id).
/// Emits EXPRESS positional order `u1,u2,v1,v2,usense,vsense`.
#[allow(clippy::similar_names)] // `usense` / `vsense` mirror the EXPRESS field names.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lift_rectangular_trimmed_surface(
    basis_surface: u64,
    u1: f64,
    u2: f64,
    usense: bool,
    v1: f64,
    v2: f64,
    vsense: bool,
) -> EarlyRectangularTrimmedSurface {
    EarlyRectangularTrimmedSurface {
        name: String::new(),
        basis_surface,
        u1,
        u2,
        v1,
        v2,
        usense,
        vsense,
    }
}

/// Lift one `COMPOSITE_CURVE_SEGMENT` (`parent_curve` = child curve step id).
pub(crate) fn lift_composite_curve_segment(
    transition: TransitionCode,
    same_sense: bool,
    parent_curve: u64,
) -> EarlyCompositeCurveSegment {
    EarlyCompositeCurveSegment {
        transition,
        same_sense,
        parent_curve,
    }
}

/// Lift one `COMPOSITE_CURVE` (segments = child segment output step ids).
pub(crate) fn lift_composite_curve(
    segments: Vec<u64>,
    self_intersect: Logical,
) -> EarlyCompositeCurve {
    EarlyCompositeCurve {
        name: String::new(),
        segments,
        self_intersect,
    }
}

/// Lift one `TRIMMED_CURVE`. `basis_curve` and each `trim_*` `Point` step id are
/// pre-resolved by the handler's `emit_*` recursion; `Param` reals pass through.
pub(crate) fn lift_trimmed_curve(
    basis_curve: u64,
    trim_1: Vec<EarlyTrimSelect>,
    trim_2: Vec<EarlyTrimSelect>,
    sense_agreement: bool,
    master_representation: TrimMaster,
) -> EarlyTrimmedCurve {
    EarlyTrimmedCurve {
        name: String::new(),
        basis_curve,
        trim_1,
        trim_2,
        sense_agreement,
        master_representation,
    }
}

/// Shared `surface_curve` subtype lifting: emit `curve_3d` + each
/// `associated_geometry` member (a `pcurve` or `surface`) to step ids. Returns
/// the emitted refs alongside the pass-through `name`/`master_representation`.
/// A verbatim port of the legacy `emit_surface_curve_body`.
fn lift_surface_curve_data(
    buf: &mut WriteBuffer,
    body: SurfaceCurveData,
) -> Result<
    (
        String,
        u64,
        Vec<u64>,
        crate::ir::geometry::PreferredSurfaceCurveRepresentation,
    ),
    WriteError,
> {
    let curve_3d = buf.emit_curve(body.curve_3d)?;
    let mut associated_geometry = Vec::with_capacity(body.associated_geometry.len());
    for item in body.associated_geometry {
        let step = match item {
            PCurveOrSurface::Pcurve(pc) => buf.emit_pcurve(pc)?,
            PCurveOrSurface::Surface(id) => buf.emit_surface(id)?,
        };
        associated_geometry.push(step);
    }
    Ok((
        body.name,
        curve_3d,
        associated_geometry,
        body.master_representation,
    ))
}

/// Lift one `INTERSECTION_CURVE` (refs emitted to step ids by the handler).
pub(crate) fn lift_intersection_curve(
    buf: &mut WriteBuffer,
    body: SurfaceCurveData,
) -> Result<EarlyIntersectionCurve, WriteError> {
    let (name, curve_3d, associated_geometry, master_representation) =
        lift_surface_curve_data(buf, body)?;
    Ok(EarlyIntersectionCurve {
        name,
        curve_3d,
        associated_geometry,
        master_representation,
    })
}

/// Lift one `BOUNDED_SURFACE_CURVE`.
pub(crate) fn lift_bounded_surface_curve(
    buf: &mut WriteBuffer,
    body: SurfaceCurveData,
) -> Result<EarlyBoundedSurfaceCurve, WriteError> {
    let (name, curve_3d, associated_geometry, master_representation) =
        lift_surface_curve_data(buf, body)?;
    Ok(EarlyBoundedSurfaceCurve {
        name,
        curve_3d,
        associated_geometry,
        master_representation,
    })
}
