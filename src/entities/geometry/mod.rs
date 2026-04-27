//! Geometry-group entity handlers (subset of the catalog `geometry_3d` group).
//!
//! Step 1 pilot: DIRECTION + VECTOR. Migration of the rest happens in
//! Plan 5 (`INFRA_PLAN.md`).

pub mod axis1_placement;
pub mod axis2_placement_2d;
pub mod axis2_placement_3d;
pub mod b_spline_curve_2d_with_knots;
pub mod b_spline_curve_with_knots;
pub mod b_spline_surface_with_knots;
pub mod cartesian_point;
pub mod cartesian_point_2d;
pub mod circle;
pub mod circle_2d;
pub mod composite_curve;
pub mod composite_curve_segment;
pub mod conical_surface;
pub mod cylindrical_surface;
pub mod direction;
pub mod direction_2d;
pub mod ellipse;
pub mod ellipse_2d;
pub mod line;
pub mod line_2d;
pub mod offset_surface;
pub mod plane;
pub mod rational_bspline_curve;
pub mod rational_bspline_surface;
pub mod seam_curve;
pub mod spherical_surface;
pub mod surface_curve;
pub mod surface_of_linear_extrusion;
pub mod surface_of_revolution;
pub mod toroidal_surface;
pub mod trimmed_curve;
pub mod vector;
pub mod vector_2d;
