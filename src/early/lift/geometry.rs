//! Geometry-domain `lift` fns (W-RECURSIVE pilot). See the [module docs](super)
//! for the lifting contract. The legacy writer synthesised an empty `name`
//! (`String::new()`), so these lifts always set `name: String::new()`.

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement3d, EarlyCartesianPoint, EarlyCircle,
    EarlyCircularArea, EarlyConicalSurface, EarlyCurveBoundedSurface, EarlyCylindricalSurface,
    EarlyDegenerateToroidalSurface, EarlyDirection, EarlyEllipse, EarlyHyperbola, EarlyLine,
    EarlyOffsetCurve3d, EarlyOffsetSurface, EarlyParabola, EarlyPlanarBox, EarlyPlanarExtent,
    EarlyPlane, EarlyPolyline, EarlySphericalSurface, EarlySurfaceOfLinearExtrusion,
    EarlySurfaceOfRevolution, EarlyToroidalSurface, EarlyVector, EarlyVertexPoint,
};
use crate::ir::geometry::{Direction3, Logical, Point3};

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
