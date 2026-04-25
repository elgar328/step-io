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

/// All curve variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    Line(Line3),
    Circle(Circle3),
    Ellipse(Ellipse3),
    Nurbs(NurbsCurve),
    Trimmed(TrimmedCurve),
    Composite(CompositeCurve),
    // Future: Offset
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

/// A trimmed curve — a portion of a basis curve bounded by two trim points.
///
/// STEP `TRIMMED_CURVE(basis, trim_1, trim_2, sense_agreement, master_repr)`.
/// Each trim slot is a SET that may carry a `CARTESIAN_POINT` reference, a
/// `PARAMETER_VALUE`, or both (redundant form). `master` indicates which is
/// authoritative when both are present.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimmedCurve {
    pub basis: CurveId,
    pub trim_1_param: Option<f64>,
    pub trim_1_point: Option<PointId>,
    pub trim_2_param: Option<f64>,
    pub trim_2_point: Option<PointId>,
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
    /// `.T.` / `.F.` / `.U.` (None) from `self_intersect : LOGICAL`.
    pub self_intersect: Option<bool>,
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

/// A NURBS (Non-Uniform Rational B-Spline) curve.
///
/// Unifies all B-Spline variants: `B_SPLINE_CURVE_WITH_KNOTS` (non-rational,
/// `weights: None`) and `RATIONAL_B_SPLINE_CURVE` (rational, `weights: Some`).
// TODO: other B-Spline variants (UNIFORM_CURVE, BEZIER_CURVE) can also map here.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsCurve {
    pub degree: u32,
    pub control_points: Vec<PointId>,
    /// `None` = non-rational (all weights implicitly 1.0).
    /// `Some` = rational (NURBS) — populated from `RATIONAL_B_SPLINE_CURVE`.
    pub weights: Option<Vec<f64>>,
    pub knot_multiplicities: Vec<i64>,
    pub knots: Vec<f64>,
    pub closed: bool,
    pub form: CurveForm,
}

/// A NURBS (Non-Uniform Rational B-Spline) surface.
///
/// Unifies all B-Spline surface variants.
// TODO: other B-Spline variants (UNIFORM_SURFACE, BEZIER_SURFACE) can also map here.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsSurface {
    pub u_degree: u32,
    pub v_degree: u32,
    /// Row-major 2D grid: `control_points[u][v]`.
    pub control_points: Vec<Vec<PointId>>,
    /// `None` = non-rational. `Some` = rational — 2D grid matching `control_points`.
    pub weights: Option<Vec<Vec<f64>>>,
    pub u_knot_multiplicities: Vec<i64>,
    pub v_knot_multiplicities: Vec<i64>,
    pub u_knots: Vec<f64>,
    pub v_knots: Vec<f64>,
    pub u_closed: bool,
    pub v_closed: bool,
    pub form: SurfaceForm,
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
/// Corresponds to STEP `OFFSET_SURFACE`. The `self_intersect` LOGICAL attribute
/// of the source entity is informational and not stored (handled uniformly
/// with `B_SPLINE_SURFACE` family — see ROADMAP "LOGICAL 보존" for planned
/// global recovery).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceOfOffset {
    pub basis: SurfaceId,
    pub distance: f64,
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
/// with 2D weights) is not yet produced by reader — `weights` stays `None`
/// for all current fixtures. The field is kept for structural parity so a
/// future rational converter can populate it without an IR change.
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsCurve2d {
    pub degree: u32,
    pub control_points: Vec<Point2dId>,
    pub weights: Option<Vec<f64>>,
    pub knot_multiplicities: Vec<i64>,
    pub knots: Vec<f64>,
    pub closed: bool,
    pub form: CurveForm,
}

/// A 2D curve mounted on a 3D surface — one entry in a `SURFACE_CURVE` /
/// `SEAM_CURVE`'s `associated_geometry` list. Both fields are `Copy` Id
/// newtypes, so `Pcurve` itself is `Copy` and `Vec<Pcurve>` clones cheaply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pcurve {
    pub basis_surface: SurfaceId,
    pub curve_2d: Curve2dId,
}
