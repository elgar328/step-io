use super::id::{
    Curve2dId, CurveId, Direction2dId, DirectionId, Placement1dId, Placement2dId, Placement3dId,
    Point2dId, PointId, SurfaceId,
};

/// A point in 3D space. Coordinates are in the file's native units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A topological vertex — a point in space.
///
/// `vertex_point` in STEP carries multiple SUBTYPE OF (`vertex`,
/// `geometric_representation_item`); ir.toml resolves its arena to
/// `geometric_representation_item`, so the struct lives in the geometry
/// module even though references flow from topology (edge, wire) and
/// the arena is part of [`GeometryPool`](crate::ir::GeometryPool).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vertex {
    pub point: PointId,
}

/// A unit direction vector in 3D. Components are direction ratios
/// (not necessarily normalized — normalization is the kernel's responsibility).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 1D axis placement — STEP `AXIS1_PLACEMENT(name, location, axis)`.
///
/// Stored in [`GeometryPool::placements_1d`] and referenced by
/// [`Placement1dId`]. Currently only `SurfaceOfRevolution` references one.
///
/// STEP spec marks `axis` as optional, but reader requires a concrete
/// direction (`SURFACE_OF_REVOLUTION` always supplies one), so the field
/// is non-optional here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis1Placement {
    pub location: PointId,
    pub axis: DirectionId,
}

/// Axis placement in 3D — location + optional Z axis + optional X direction.
///
/// Stored in [`GeometryPool::placements`] and referenced by [`Placement3dId`].
/// Surface / Curve variants hold the id; the reader preserves every on-disk
/// `AXIS2_PLACEMENT_3D` as a distinct arena entry.
///
/// `None` means "omitted in the STEP file". The default interpretation
/// (`(0,0,1)` for axis, `(1,0,0)` for `ref_direction`) is the **kernel
/// adapter's** responsibility — the IR preserves the original omission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis2Placement3d {
    pub location: PointId,
    /// Z axis. `None` when the STEP attribute was `$` (unset).
    pub axis: Option<DirectionId>,
    /// X direction. `None` when the STEP attribute was `$` (unset).
    pub ref_direction: Option<DirectionId>,
}

/// All surface variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Surface {
    Plane(Plane3),
    Cylinder(CylindricalSurface),
    Sphere(SphericalSurface),
    Cone(ConicalSurface),
    Torus(ToroidalSurface),
    Revolution(SurfaceOfRevolution),
    Extrusion(SurfaceOfLinearExtrusion),
    Offset(SurfaceOfOffset),
    Nurbs(NurbsSurface),
    RectangularTrimmed(RectangularTrimmedSurface),
    DegenerateToroidal(DegenerateToroidalSurface),
}

/// `DEGENERATE_TOROIDAL_SURFACE(name, position, major_radius, minor_radius,
/// select_outer)`. SUBTYPE OF `toroidal_surface` — covers degenerate cases
/// where the minor radius produces self-intersection; `select_outer`
/// chooses the outer or inner sheet of the resulting surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DegenerateToroidalSurface {
    pub position: Placement3dId,
    pub major_radius: f64,
    pub minor_radius: f64,
    pub select_outer: bool,
}

/// An infinite plane defined by an axis placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane3 {
    pub position: Placement3dId,
}

/// A cylindrical surface defined by an axis placement and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylindricalSurface {
    pub position: Placement3dId,
    pub radius: f64,
}

/// A conical surface defined by an axis placement, a radius, and a semi-angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConicalSurface {
    pub position: Placement3dId,
    /// Radius at the base. Zero when the cone starts at its apex.
    pub radius: f64,
    /// Half-angle of the cone in radians.
    pub semi_angle: f64,
}

/// A toroidal surface defined by an axis placement, major and minor radii.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToroidalSurface {
    pub position: Placement3dId,
    pub major_radius: f64,
    pub minor_radius: f64,
}

/// A spherical surface defined by an axis placement and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalSurface {
    pub position: Placement3dId,
    pub radius: f64,
}

/// `RECTANGULAR_TRIMMED_SURFACE(name, basis_surface, u1, u2, usense, v1, v2, vsense)`.
/// Parameter-space rectangle trimming a basis surface. `usense` / `vsense`
/// select the trim direction along each parameter (`true` → increasing parameter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularTrimmedSurface {
    pub basis: SurfaceId,
    pub u1: f64,
    pub u2: f64,
    pub usense: bool,
    pub v1: f64,
    pub v2: f64,
    pub vsense: bool,
}

/// All curve variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    Line(Line3),
    Circle(Circle3),
    Ellipse(Ellipse3),
    Nurbs(NurbsCurve),
    Trimmed(TrimmedCurve),
    Composite(CompositeCurve),
    Polyline(Polyline),
    Hyperbola(Hyperbola),
    Parabola(Parabola),
    OffsetCurve3d(OffsetCurve3d),
}

/// `OFFSET_CURVE_3D(name, basis_curve, distance, self_intersect, ref_direction)`.
/// Offset of a basis 3D curve by `distance` along `ref_direction`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetCurve3d {
    pub basis: CurveId,
    pub distance: f64,
    pub self_intersect: Logical,
    pub ref_direction: DirectionId,
}

/// `HYPERBOLA(name, position, semi_axis, semi_imag_axis)` — 3D conic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hyperbola {
    pub position: Placement3dId,
    pub semi_axis: f64,
    pub semi_imag_axis: f64,
}

/// `PARABOLA(name, position, focal_dist)` — 3D conic. `focal_dist` is
/// signed; the sign selects orientation along `position.axis`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parabola {
    pub position: Placement3dId,
    pub focal_dist: f64,
}

/// A 3D polyline: ordered list of point ids forming a piecewise-linear curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Vec<PointId>,
}

/// A circle defined by an axis placement and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle3 {
    pub position: Placement3dId,
    pub radius: f64,
}

/// An ellipse defined by an axis placement and two semi-axis lengths.
///
/// `semi_axis_1` lies along `position.ref_direction` (X axis); `semi_axis_2`
/// along the axis normal to both `position.axis` and `ref_direction`
/// (Y axis). STEP does not guarantee `semi_axis_1 >= semi_axis_2`, so the
/// field names mirror the spec rather than imply major/minor ordering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipse3 {
    pub position: Placement3dId,
    pub semi_axis_1: f64,
    pub semi_axis_2: f64,
}

/// A line defined by a point, direction, and magnitude (from `VECTOR`).
///
/// In STEP, `LINE` references a `CARTESIAN_POINT` and a `VECTOR`.
/// `VECTOR` holds a `DIRECTION` and a magnitude scalar. The IR flattens
/// this into a single struct — `VECTOR` has no independent meaning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line3 {
    pub point: PointId,
    pub direction: DirectionId,
    /// Magnitude from the `VECTOR` entity. Affects line parameterization.
    pub magnitude: f64,
}

/// Shape-family hint from STEP `b_spline_curve_form`.
///
/// Informational only — control points + knots are authoritative for the
/// curve's geometry. Preserved verbatim so downstream tools can try to
/// recover the original primitive (circular arc, etc.) that the B-spline
/// was built to approximate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurveForm {
    PolylineForm,
    CircularArc,
    EllipticArc,
    ParabolicArc,
    HyperbolicArc,
    #[default]
    Unspecified,
}

impl CurveForm {
    /// Parse from a STEP enum name (e.g. `"CIRCULAR_ARC"`). Unknown values
    /// map to `Unspecified` — the hint is best-effort.
    #[must_use]
    pub fn from_step_enum(s: &str) -> Self {
        match s {
            "POLYLINE_FORM" => Self::PolylineForm,
            "CIRCULAR_ARC" => Self::CircularArc,
            "ELLIPTIC_ARC" => Self::EllipticArc,
            "PARABOLIC_ARC" => Self::ParabolicArc,
            "HYPERBOLIC_ARC" => Self::HyperbolicArc,
            _ => Self::Unspecified,
        }
    }

    /// The STEP enum name (unwrapped — caller applies the surrounding dots).
    #[must_use]
    pub fn as_step_enum(self) -> &'static str {
        match self {
            Self::PolylineForm => "POLYLINE_FORM",
            Self::CircularArc => "CIRCULAR_ARC",
            Self::EllipticArc => "ELLIPTIC_ARC",
            Self::ParabolicArc => "PARABOLIC_ARC",
            Self::HyperbolicArc => "HYPERBOLIC_ARC",
            Self::Unspecified => "UNSPECIFIED",
        }
    }
}

/// Shape-family hint from STEP `b_spline_surface_form`.
///
/// Informational only — control points + knots are authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceForm {
    PlaneSurf,
    CylindricalSurf,
    ConicalSurf,
    SphericalSurf,
    ToroidalSurf,
    SurfOfRevolution,
    RuledSurf,
    GeneralisedCone,
    QuadricSurf,
    SurfOfLinearExtrusion,
    #[default]
    Unspecified,
}

impl SurfaceForm {
    #[must_use]
    pub fn from_step_enum(s: &str) -> Self {
        match s {
            "PLANE_SURF" => Self::PlaneSurf,
            "CYLINDRICAL_SURF" => Self::CylindricalSurf,
            "CONICAL_SURF" => Self::ConicalSurf,
            "SPHERICAL_SURF" => Self::SphericalSurf,
            "TOROIDAL_SURF" => Self::ToroidalSurf,
            "SURF_OF_REVOLUTION" => Self::SurfOfRevolution,
            "RULED_SURF" => Self::RuledSurf,
            "GENERALISED_CONE" => Self::GeneralisedCone,
            "QUADRIC_SURF" => Self::QuadricSurf,
            "SURF_OF_LINEAR_EXTRUSION" => Self::SurfOfLinearExtrusion,
            _ => Self::Unspecified,
        }
    }

    #[must_use]
    pub fn as_step_enum(self) -> &'static str {
        match self {
            Self::PlaneSurf => "PLANE_SURF",
            Self::CylindricalSurf => "CYLINDRICAL_SURF",
            Self::ConicalSurf => "CONICAL_SURF",
            Self::SphericalSurf => "SPHERICAL_SURF",
            Self::ToroidalSurf => "TOROIDAL_SURF",
            Self::SurfOfRevolution => "SURF_OF_REVOLUTION",
            Self::RuledSurf => "RULED_SURF",
            Self::GeneralisedCone => "GENERALISED_CONE",
            Self::QuadricSurf => "QUADRIC_SURF",
            Self::SurfOfLinearExtrusion => "SURF_OF_LINEAR_EXTRUSION",
            Self::Unspecified => "UNSPECIFIED",
        }
    }
}

/// SET element of a `TRIMMED_CURVE` trim slot — STEP SELECT
/// `(PARAMETER_VALUE | CARTESIAN_POINT)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrimSelect {
    Param(f64),
    Point(PointId),
}

/// A trimmed curve — a portion of a basis curve bounded by two trim points.
///
/// STEP `TRIMMED_CURVE(basis, trim_1, trim_2, sense_agreement, master_repr)`.
/// Each trim slot is a SET (cardinality 0~2) that may carry a `CARTESIAN_POINT`
/// reference, a `PARAMETER_VALUE`, or both (redundant form). `master`
/// indicates which form is authoritative when both are present.
#[derive(Debug, Clone, PartialEq)]
pub struct TrimmedCurve {
    pub basis: CurveId,
    pub trim_1: Vec<TrimSelect>,
    pub trim_2: Vec<TrimSelect>,
    pub sense_agreement: bool,
    pub master: TrimMaster,
}

/// `master_representation` of a `TRIMMED_CURVE` — which trim form is
/// authoritative when both `cartesian` and `parameter` are present.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TrimMaster {
    Cartesian,
    Parameter,
    #[default]
    Unspecified,
}

/// A composite curve — sequence of segments joined end-to-end.
///
/// STEP `COMPOSITE_CURVE(segments, self_intersect)`. Each segment carries its
/// own transition continuity and orientation flag with a reference to a
/// parent curve (line, arc, trimmed curve, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeCurve {
    pub segments: Vec<CompositeSegment>,
    /// `self_intersect : LOGICAL`.
    pub self_intersect: Logical,
}

/// One segment of a `CompositeCurve`. Mirrors STEP `COMPOSITE_CURVE_SEGMENT`
/// but inlined inside the parent — segments are not arena entries on their
/// own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositeSegment {
    pub transition: TransitionCode,
    pub same_sense: bool,
    pub parent_curve: CurveId,
}

/// STEP `LOGICAL` 3-state. `Unknown` corresponds to the `.U.` literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Logical {
    True,
    False,
    #[default]
    Unknown,
}

/// `transition_code` of a `COMPOSITE_CURVE_SEGMENT` — geometric continuity
/// between consecutive segments.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TransitionCode {
    #[default]
    Continuous,
    Discontinuous,
    ContSameGradient,
    ContSameGradientSameCurvature,
    Unspecified,
}

/// Rationality of a [`NurbsCurve`].
///
/// `NonRational` = `B_SPLINE_CURVE_WITH_KNOTS` (all weights implicitly 1.0).
/// `Rational { weights }` = `RATIONAL_B_SPLINE_CURVE`; the writer expects
/// `weights.len() == control_points.len()`.
#[derive(Debug, Clone, PartialEq)]
pub enum NurbsKind {
    NonRational,
    Rational { weights: Vec<f64> },
}

/// Rationality of a [`NurbsSurface`].
///
/// `Rational { weights }` carries a 2D grid matching the parent surface's
/// `control_points` shape.
#[derive(Debug, Clone, PartialEq)]
pub enum NurbsSurfaceKind {
    NonRational,
    Rational { weights: Vec<Vec<f64>> },
}

/// A NURBS (Non-Uniform Rational B-Spline) curve.
///
/// Unifies `B_SPLINE_CURVE_WITH_KNOTS` (non-rational) and
/// `RATIONAL_B_SPLINE_CURVE` (rational) via [`NurbsKind`].
// TODO: other B-Spline variants (UNIFORM_CURVE, BEZIER_CURVE) can also map here.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsCurve {
    pub degree: u32,
    pub control_points: Vec<PointId>,
    pub kind: NurbsKind,
    pub knot_multiplicities: Vec<i64>,
    pub knots: Vec<f64>,
    pub closed: bool,
    pub form: CurveForm,
    /// `self_intersect : LOGICAL`.
    pub self_intersect: Logical,
}

impl NurbsCurve {
    /// Returns `Some(&weights)` for rational, `None` for non-rational.
    /// Convenience accessor for sites that only need to check rationality
    /// or read the weight slice.
    #[must_use]
    pub fn weights(&self) -> Option<&[f64]> {
        match &self.kind {
            NurbsKind::NonRational => None,
            NurbsKind::Rational { weights } => Some(weights),
        }
    }
}

/// A NURBS (Non-Uniform Rational B-Spline) surface.
///
/// Unifies all B-Spline surface variants via [`NurbsSurfaceKind`].
// TODO: other B-Spline variants (UNIFORM_SURFACE, BEZIER_SURFACE) can also map here.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsSurface {
    pub u_degree: u32,
    pub v_degree: u32,
    /// Row-major 2D grid: `control_points[u][v]`.
    pub control_points: Vec<Vec<PointId>>,
    pub kind: NurbsSurfaceKind,
    pub u_knot_multiplicities: Vec<i64>,
    pub v_knot_multiplicities: Vec<i64>,
    pub u_knots: Vec<f64>,
    pub v_knots: Vec<f64>,
    pub u_closed: bool,
    pub v_closed: bool,
    pub form: SurfaceForm,
    /// `self_intersect : LOGICAL`.
    pub self_intersect: Logical,
}

impl NurbsSurface {
    /// Returns `Some(&weights)` for rational, `None` for non-rational.
    #[must_use]
    pub fn weights(&self) -> Option<&[Vec<f64>]> {
        match &self.kind {
            NurbsSurfaceKind::NonRational => None,
            NurbsSurfaceKind::Rational { weights } => Some(weights),
        }
    }
}

/// A surface of revolution defined by rotating a curve around an axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceOfRevolution {
    pub swept_curve: CurveId,
    pub axis_placement: Placement1dId,
}

/// A surface of linear extrusion — a curve swept along a straight direction.
///
/// `depth` is the extrusion distance from the `VECTOR`'s magnitude.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceOfLinearExtrusion {
    pub swept_curve: CurveId,
    pub extrusion_direction: DirectionId,
    pub depth: f64,
}

/// A surface offset from a basis surface along its normal by a signed distance.
///
/// Corresponds to STEP `OFFSET_SURFACE`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceOfOffset {
    pub basis: SurfaceId,
    pub distance: f64,
    /// `self_intersect : LOGICAL`.
    pub self_intersect: Logical,
}

// ---------------------------------------------------------------------------
// 2D geometry (PCURVE parametric space)
// ---------------------------------------------------------------------------
//
// 2D types live alongside their 3D counterparts but in a separate Id / arena
// family. They appear exclusively inside `PCURVE` / `SURFACE_CURVE` /
// `SEAM_CURVE` subtrees — i.e. the UV-parameter-space representation of an
// edge on a surface. A STEP `DEFINITIONAL_REPRESENTATION` groups such 2D
// curves under a `GEOMETRIC_REPRESENTATION_CONTEXT(2)` context; step-io
// does not keep a typed representation of that wrapper, only the 2D
// primitives it contains.

/// A point in 2D parametric space. Unlike [`Point3`] these coordinates are
/// surface-local (u, v) values, not world-space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

/// A unit direction vector in 2D parametric space. Normalisation is the
/// kernel adapter's responsibility, same policy as [`Direction3`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction2 {
    pub x: f64,
    pub y: f64,
}

/// 2D axis placement used by 2D circles and ellipses inside PCURVE data.
/// Unlike [`Axis2Placement3d`] it has no `axis` field — 2D needs only the
/// reference direction to fix rotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis2Placement2d {
    pub location: Point2dId,
    pub ref_direction: Option<Direction2dId>,
}

/// All 2D curve variants. Mirrors [`Curve`] but in 2D parametric space.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve2d {
    Line(Line2),
    Circle(Circle2),
    Ellipse(Ellipse2),
    Nurbs(NurbsCurve2d),
    Polyline(Polyline2d),
}

/// A 2D polyline: ordered list of 2D point ids forming a piecewise-linear curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline2d {
    pub points: Vec<Point2dId>,
}

/// A 2D line: point + direction + magnitude (2D VECTOR scalar).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line2 {
    pub point: Point2dId,
    pub direction: Direction2dId,
    pub magnitude: f64,
}

/// A 2D circle defined by a 2D axis placement and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle2 {
    pub position: Placement2dId,
    pub radius: f64,
}

/// A 2D ellipse defined by a 2D axis placement and two semi-axis lengths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipse2 {
    pub position: Placement2dId,
    pub semi_axis_1: f64,
    pub semi_axis_2: f64,
}

/// A 2D NURBS curve. Structure mirrors the 3D [`NurbsCurve`] but with 2D
/// control points. Rational 2D NURBS (complex entity `RATIONAL_B_SPLINE_CURVE`
/// with 2D weights) is not yet emitted by writer (see [`NurbsKind2d`]).
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsCurve2d {
    pub degree: u32,
    pub control_points: Vec<Point2dId>,
    pub kind: NurbsKind2d,
    pub knot_multiplicities: Vec<i64>,
    pub knots: Vec<f64>,
    pub closed: bool,
    pub form: CurveForm,
}

/// Rationality of a [`NurbsCurve2d`]. Same shape as 3D [`NurbsKind`].
#[derive(Debug, Clone, PartialEq)]
pub enum NurbsKind2d {
    NonRational,
    Rational { weights: Vec<f64> },
}

impl NurbsCurve2d {
    /// Returns `Some(&weights)` for rational, `None` for non-rational.
    #[must_use]
    pub fn weights(&self) -> Option<&[f64]> {
        match &self.kind {
            NurbsKind2d::NonRational => None,
            NurbsKind2d::Rational { weights } => Some(weights),
        }
    }
}

/// A 2D curve mounted on a 3D surface — one entry in a `SURFACE_CURVE` /
/// `SEAM_CURVE`'s `associated_geometry` list. Both fields are `Copy` Id
/// newtypes, so `Pcurve` itself is `Copy` and `Vec<Pcurve>` clones cheaply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pcurve {
    pub basis_surface: SurfaceId,
    pub curve_2d: Curve2dId,
}

/// `PLANAR_EXTENT` / `PLANAR_BOX` — a rectangular planar region. The
/// ir.toml blueprint models them as a `concrete_supertype`: one arena, an
/// enum with the base `Itself` variant plus the `PlanarBox` subtype.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanarExtent {
    Itself(PlanarExtentData),
    PlanarBox(PlanarBox),
}

/// `PLANAR_EXTENT(name, size_in_x, size_in_y)` — the base form.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanarExtentData {
    pub name: String,
    pub size_in_x: f64,
    pub size_in_y: f64,
}

/// `PLANAR_BOX(name, size_in_x, size_in_y, placement)` — a planar extent
/// anchored by a coordinate placement.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanarBox {
    pub name: String,
    pub size_in_x: f64,
    pub size_in_y: f64,
    pub placement: PlanarBoxPlacement,
}

/// `PLANAR_BOX.placement` — the STEP `axis2_placement` SELECT
/// (`AXIS2_PLACEMENT_2D` | `AXIS2_PLACEMENT_3D`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanarBoxPlacement {
    Placement2d(Placement2dId),
    Placement3d(Placement3dId),
}

/// `CIRCULAR_AREA(name, centre, radius)` — `primitive_2d` SUBTYPE.
/// Orphan in step-io; corpus 1 instance.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularArea {
    pub name: String,
    pub centre: PointId,
    pub radius: f64,
}
