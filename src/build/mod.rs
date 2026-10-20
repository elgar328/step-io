//! [`StepBuilder`] — the convenience authoring layer over the strict
//! [`Ap242Author`] constructors.
//!
//! [`new`](StepBuilder::new) lays down the skeleton every exporter needs
//! (application protocol, product contexts, units —
//! [`new_with`](StepBuilder::new_with) to choose them);
//! [`finish`](StepBuilder::finish) assembles everything added so far into
//! the finished model, which [`write`](crate::write) sends out as Part 21
//! text.
//!
//! The core is parts and their shape. [`part`](StepBuilder::part) adds one
//! product with its full definition chain
//! ([`part_with`](StepBuilder::part_with) also sets part number / version /
//! descriptions). Topology mirrors the kernel's own —
//! [`vertex`](StepBuilder::vertex) → [`edge`](StepBuilder::edge) →
//! [`face`](StepBuilder::face) → [`solid`](StepBuilder::solid), with analytic
//! surfaces and curves as plain inputs ([`SurfaceInput`], [`CurveInput`],
//! [`ProfileInput`]). Assemblies are one
//! [`place(parent, child, at)`](StepBuilder::place) call per placed
//! instance, the child's definition shared between them.
//!
//! Everything else is one call each. [`mesh`](StepBuilder::mesh) attaches a
//! display mesh (vertex array + index triangles), optionally linked to the
//! b-rep solid it renders. [`style`](StepBuilder::style) colours a solid or
//! a single face, with optional transparency; [`layer`](StepBuilder::layer),
//! [`hide`](StepBuilder::hide), and [`hide_layer`](StepBuilder::hide_layer)
//! manage presentation layers and default visibility.
//! [`contributor`](StepBuilder::contributor),
//! [`approve`](StepBuilder::approve), [`document`](StepBuilder::document),
//! and [`security`](StepBuilder::security) attach PLM records to a part's
//! version, with people as shared
//! [`person_and_org`](StepBuilder::person_and_org) handles.
//! [`header`](StepBuilder::header) fills the Part 21 file header.
//!
//! Anything the builder does not cover is still reachable:
//! [`author`](StepBuilder::author) hands out the underlying [`Ap242Author`],
//! and ids cross freely between the two levels.
//!
//! The minimal flow — a planar part handed over from a kernel:
//!
//! ```no_run
//! use step_io::Orientation;
//! use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
//!
//! # struct KernelEdge { start: usize, end: usize }
//! # struct KernelFace { origin: [f64; 3], normal: [f64; 3], x_dir: [f64; 3], bound: Vec<usize> }
//! # struct Kernel { points: Vec<[f64; 3]>, edges: Vec<KernelEdge>, faces: Vec<KernelFace> }
//! # let kernel = Kernel { points: vec![], edges: vec![], faces: vec![] };
//! let mut b = step_io::StepBuilder::new().unwrap();
//! let plate = b.part("plate").unwrap();
//!
//! let vs: Vec<_> = kernel.points.iter()
//!     .map(|p| b.vertex(*p).unwrap())
//!     .collect();
//! let es: Vec<_> = kernel.edges.iter()
//!     .map(|e| b.edge(vs[e.start], vs[e.end], CurveInput::Line).unwrap())
//!     .collect();
//! let fs: Vec<_> = kernel.faces.iter()
//!     .map(|f| {
//!         let plane = Frame { origin: f.origin, axis: f.normal, ref_dir: f.x_dir };
//!         let bound = f.bound.iter().map(|&i| (es[i], Orientation::Forward)).collect();
//!         b.face(SurfaceInput::Plane(plane), Orientation::Forward, vec![FaceBoundInput::outer(bound)])
//!             .unwrap()
//!     })
//!     .collect();
//! b.solid(plate, "plate body", fs).unwrap();
//!
//! step_io::write(&b.finish().unwrap(), std::fs::File::create("plate.step").unwrap()).unwrap();
//! ```
//!
//! A kernel that already holds an edge's exact direction passes it as
//! [`CurveInput::LineAlong`] instead, and the file keeps those bits rather than
//! the direction two rounded vertex positions imply.
//!
//! The full tour — assembly, display mesh, presentation, metadata, header:
//!
//! ```no_run
//! use step_io::build::{
//!     ApprovalInput, Frame, HeaderInput, MeshInput, MeshNormalsInput, PersonOrg, StyleTarget,
//! };
//! use step_io::scene::Rgb;
//!
//! # struct Kernel { mesh_points: Vec<[f64; 3]>, mesh_triangles: Vec<[usize; 3]> }
//! # let kernel = Kernel { mesh_points: vec![], mesh_triangles: vec![] };
//! # fn faces(_b: &mut step_io::StepBuilder) -> Vec<step_io::model::AdvancedFaceId> { vec![] }
//! let mut b = step_io::StepBuilder::new().unwrap();
//! b.header(&HeaderInput {
//!     file_name: Some("car.step".into()),
//!     organizations: vec!["ACME".into()],
//!     originating_system: Some("MyKernel 2.1".into()),
//!     ..Default::default() // timestamp auto-stamped
//! });
//!
//! let car = b.part("car").unwrap();
//! let wheel = b.part("wheel").unwrap();
//! let wheel_faces = faces(&mut b); // b-rep translated as in the minimal flow
//! let body = b.solid(wheel, "wheel body", wheel_faces).unwrap();
//!
//! // One placed instance per call; the wheel's definition is shared.
//! let z = [0.0, 0.0, 1.0];
//! let x = [1.0, 0.0, 0.0];
//! b.place(car, wheel, Frame { origin: [900.0, 700.0, 300.0], axis: z, ref_dir: x }).unwrap();
//! b.place(car, wheel, Frame { origin: [900.0, -700.0, 300.0], axis: z, ref_dir: x }).unwrap();
//!
//! // Display mesh (the kernel's tessellation) linked to the b-rep it renders.
//! b.mesh(wheel, "wheel mesh", &MeshInput {
//!     points: kernel.mesh_points,
//!     triangles: kernel.mesh_triangles,
//!     normals: MeshNormalsInput::None,
//! }, Some(body.into())).unwrap();
//!
//! // Presentation: colour/transparency, layers, default visibility.
//! b.style(StyleTarget::Solid(body), Rgb { red: 0.8, green: 0.2, blue: 0.1 }, None).unwrap();
//! let hidden = b.layer("REFERENCE", vec![StyleTarget::Solid(body)]).unwrap();
//! b.hide_layer(hidden);
//!
//! // PLM metadata on the part's version.
//! let john = b.person_and_org(&PersonOrg {
//!     person_id: "p1".into(),
//!     first_name: Some("John".into()),
//!     last_name: Some("Doe".into()),
//!     organization: "ACME".into(),
//! }).unwrap();
//! b.contributor(wheel, "creator", john).unwrap();
//! b.approve(wheel, &ApprovalInput {
//!     status: "approved".into(),
//!     level: "final".into(),
//!     approvers: vec![(john, "approver".into())],
//!     date: None,
//! }).unwrap();
//!
//! let model = b.finish().unwrap();
//! # let _ = model;
//! ```

use std::collections::HashSet;

// Shared with the read side (`scene`): the NURBS forms `to_nurbs()` returns are
// exactly what the builder accepts, and colours round-trip as the same type.
use crate::Orientation;
use crate::generated::author::{Ap242Author, AuthorError};
use crate::generated::model as m;
use crate::header::FileHeader;
use crate::scene::{NurbsCurve, NurbsSurface, Rgb};

/// Part 21 HEADER fields for [`StepBuilder::header`]. Everything defaults
/// to empty except the pieces the builder supplies itself: the timestamp
/// (current UTC time unless overridden) and the preprocessor stamp
/// (always `step-io <version>`).
#[derive(Debug, Clone, Default)]
pub struct HeaderInput {
    /// `FILE_NAME.name` — the customary file name.
    pub file_name: Option<String>,
    /// `FILE_DESCRIPTION` entry.
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub organizations: Vec<String>,
    /// `FILE_NAME.originating_system` — the exporting kernel/application.
    pub originating_system: Option<String>,
    pub authorisation: Option<String>,
    /// `None` = stamp the current UTC time; `Some` = used verbatim (fix it
    /// for reproducible output, or `""` to leave the field blank).
    pub timestamp: Option<String>,
}

/// Days-from-civil inverse (Howard Hinnant's algorithm): unix seconds to a
/// `YYYY-MM-DDThh:mm:ssZ` UTC stamp.
fn iso_utc_from_unix(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The current time as an ISO 8601 UTC stamp.
fn now_iso_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    iso_utc_from_unix(secs)
}

/// The file's length unit, chosen at [`StepBuilder::new_with`] time (the
/// units are file-global: every geometric context refers to the same ones).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LengthUnit {
    #[default]
    Millimetre,
    Metre,
}

/// Unit setup for [`StepBuilder::new_with`]; the default is what plain
/// [`new`](StepBuilder::new) uses (millimetre, 1e-7 uncertainty).
#[derive(Debug, Clone, Copy)]
pub struct UnitsInput {
    pub length: LengthUnit,
    /// Length uncertainty (modelling precision), in the file's own length
    /// unit — expected finite and positive (degenerate values are the
    /// caller's responsibility).
    pub uncertainty: f64,
}

impl Default for UnitsInput {
    fn default() -> Self {
        Self {
            length: LengthUnit::Millimetre,
            uncertainty: 1.0e-7,
        }
    }
}

/// A right-handed placement frame: `origin` plus the `axis` (local Z) and
/// `ref_dir` (local X) directions — the builder's input for
/// `AXIS2_PLACEMENT_3D`. Directions are passed through as given (kernels
/// supply unit vectors); geometric well-formedness is the caller's
/// responsibility.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub ref_dir: [f64; 3],
}

/// Surface input for [`StepBuilder::face`]: the stored analytic forms, plus
/// NURBS for everything else — [`NurbsSurface`] is the same type
/// `Surface::to_nurbs()` returns on the read side, so read-side output can
/// be fed straight back in. Non-rational data (all weights `1.0`) becomes a
/// plain `B_SPLINE_SURFACE_WITH_KNOTS`; rational data becomes the customary
/// multi-part complex instance.
#[derive(Debug, Clone)]
pub enum SurfaceInput {
    Plane(Frame),
    /// Cylinder around the frame's axis with the given radius.
    Cylinder(Frame, f64),
    /// Sphere centred at the frame's origin with the given radius.
    Sphere(Frame, f64),
    /// Torus around the frame's axis: major radius, then minor radius.
    Torus(Frame, f64, f64),
    /// Cone around the frame's axis: radius at the frame's origin, then the
    /// half-angle in radians.
    Cone(Frame, f64, f64),
    /// Profile swept along an extrusion vector (direction × length).
    LinearExtrusion(ProfileInput, [f64; 3]),
    /// Profile revolved around an axis: axis origin, then axis direction.
    Revolution(ProfileInput, [f64; 3], [f64; 3]),
    Nurbs(NurbsSurface),
}

/// Curve input for [`StepBuilder::edge`]. [`NurbsCurve`] mirrors the
/// read-side `to_nurbs()` type; see [`SurfaceInput`] for the
/// rational/non-rational split.
#[derive(Debug, Clone)]
pub enum CurveInput {
    /// Straight segment — its direction computed from the two vertex positions.
    /// Written as a `LINE` through the start vertex whose `VECTOR` has magnitude
    /// 1: the magnitude is only the parameter scale, and the edge is bounded by
    /// its vertices, not by a parameter range.
    Line,
    /// Straight segment along a unit `direction` the caller already holds,
    /// written bit for bit — so a kernel that rounded the exact direction once,
    /// from its own definition, keeps it instead of the one two rounded vertex
    /// positions imply. A direction pointing from the end vertex back to the
    /// start is negated (exactly) so the line runs with the edge. Unit length
    /// and being parallel to the edge are the caller's to keep: a direction off
    /// the edge writes a line the end vertex does not lie on. Same unit
    /// `VECTOR` magnitude as [`CurveInput::Line`].
    LineAlong([f64; 3]),
    /// Circle in the frame's XY plane with the given radius; with equal
    /// end vertices this is a full circle, otherwise an arc.
    Circle(Frame, f64),
    /// Ellipse in the frame's XY plane: first and second semi-axis; with
    /// equal end vertices a full ellipse, otherwise an arc.
    Ellipse(Frame, f64, f64),
    /// Parabola in the frame's XY plane with the given focal distance (apex at
    /// the origin, opening along X, or along −X for a negative distance): the
    /// span between the two vertices, which must differ.
    Parabola(Frame, f64),
    /// The branch along X of a hyperbola in the frame's XY plane: semi-axis
    /// and semi-imaginary axis. The span between the two vertices, which must
    /// differ.
    Hyperbola(Frame, f64, f64),
    /// Piecewise-linear curve through the given points (customarily the
    /// first/last coincide with the edge's vertices).
    Polyline(Vec<[f64; 3]>),
    Nurbs(NurbsCurve),
}

/// A profile curve for the swept surfaces — plain geometry, no vertices
/// (unlike [`CurveInput::Line`], which derives from edge vertices).
#[derive(Debug, Clone)]
pub enum ProfileInput {
    /// Infinite straight line: a point and a direction.
    Line([f64; 3], [f64; 3]),
    Circle(Frame, f64),
    Ellipse(Frame, f64, f64),
    Nurbs(NurbsCurve),
}

/// One face bound for [`StepBuilder::face`]: the loop's edges, each with the
/// [`Orientation`] in which the loop runs along it, whether this is the outer
/// bound, and the [`Orientation`] in which the face uses the whole loop.
#[derive(Debug, Clone)]
pub struct FaceBoundInput {
    pub edges: Vec<(m::EdgeCurveId, Orientation)>,
    pub outer: bool,
    pub orientation: Orientation,
}

impl FaceBoundInput {
    /// The customary outer bound (loop used [`Forward`](Orientation::Forward)).
    #[must_use]
    pub fn outer(edges: Vec<(m::EdgeCurveId, Orientation)>) -> Self {
        Self {
            edges,
            outer: true,
            orientation: Orientation::Forward,
        }
    }

    /// An inner (hole) bound (loop used [`Forward`](Orientation::Forward)).
    #[must_use]
    pub fn inner(edges: Vec<(m::EdgeCurveId, Orientation)>) -> Self {
        Self {
            edges,
            outer: false,
            orientation: Orientation::Forward,
        }
    }
}

/// Compress an expanded knot vector (each knot repeated by multiplicity —
/// the [`NurbsCurve`]/[`NurbsSurface`] form) into STEP's distinct-knots +
/// multiplicities pair. Exact equality groups repeats; the read side expands
/// them back identically, so the round trip is lossless.
fn compress_knots(expanded: &[f64]) -> (Vec<f64>, Vec<i64>) {
    let mut knots: Vec<f64> = Vec::new();
    let mut multiplicities: Vec<i64> = Vec::new();
    for &k in expanded {
        if knots.last() == Some(&k) {
            *multiplicities.last_mut().unwrap() += 1;
        } else {
            knots.push(k);
            multiplicities.push(1);
        }
    }
    (knots, multiplicities)
}

/// Check a B-spline's degree and expanded knot vector against its `n`
/// control points by the standard's `constraints_param_b_spline`: degree at
/// least 1, more control points than the degree, `n + degree + 1` knots that
/// never decrease, and no knot repeated more than `degree` times inside or
/// `degree + 1` times at an end.
fn check_bspline(degree: usize, n: usize, knots: &[f64]) -> Result<(), AuthorError> {
    let bad = |reason| Err(AuthorError::InvalidInput { reason });
    if degree < 1 {
        return bad("B-spline degree below 1");
    }
    if n <= degree {
        return bad("B-spline with no more control points than its degree");
    }
    if knots.len() != n + degree + 1 {
        return bad("B-spline knot count is not control points + degree + 1");
    }
    if knots.windows(2).any(|w| w[1] < w[0]) {
        return bad("B-spline knots decrease");
    }
    let (_, multiplicities) = compress_knots(knots);
    let last = multiplicities.len() - 1;
    for (i, &mult) in multiplicities.iter().enumerate() {
        let max = if i == 0 || i == last {
            degree + 1
        } else {
            degree
        };
        if usize::try_from(mult).map_or(true, |mult| mult > max) {
            return bad("B-spline knot repeated more often than its degree allows");
        }
    }
    Ok(())
}

/// Check B-spline weights: one per control point, each positive.
fn check_weights(n: usize, weights: &[f64]) -> Result<(), AuthorError> {
    let bad = |reason| Err(AuthorError::InvalidInput { reason });
    if weights.len() != n {
        return bad("B-spline weights do not match its control points");
    }
    if weights.iter().any(|w| w.is_nan() || *w <= 0.0) {
        return bad("B-spline weight not positive");
    }
    Ok(())
}

/// Whether an edge from `start` to `end` on a parabola or hyperbola in `frame`
/// runs with the curve's parameter, which grows with `y / scale` (`scale` the
/// focal distance of a parabola, 1 for a hyperbola). Both curves are open, so
/// equal end points cannot make an edge.
// Exact comparison intended: only end points at the very same parameter make
// an edge without a span.
#[allow(clippy::float_cmp)]
fn open_conic_ahead(
    frame: &Frame,
    start: [f64; 3],
    end: [f64; 3],
    scale: f64,
) -> Result<bool, AuthorError> {
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (axis, ref_dir) = (frame.axis, frame.ref_dir);
    let along = dot(ref_dir, axis) / dot(axis, axis);
    let x_dir = [
        ref_dir[0] - along * axis[0],
        ref_dir[1] - along * axis[1],
        ref_dir[2] - along * axis[2],
    ];
    let y_dir = [
        axis[1] * x_dir[2] - axis[2] * x_dir[1],
        axis[2] * x_dir[0] - axis[0] * x_dir[2],
        axis[0] * x_dir[1] - axis[1] * x_dir[0],
    ];
    let offset = |p: [f64; 3]| {
        let rel = [
            p[0] - frame.origin[0],
            p[1] - frame.origin[1],
            p[2] - frame.origin[2],
        ];
        dot(rel, y_dir) / scale
    };
    let (from, to) = (offset(start), offset(end));
    if from == to {
        return Err(AuthorError::InvalidInput {
            reason: "parabola or hyperbola edge without a span",
        });
    }
    Ok(from < to)
}

/// Optional overrides for [`StepBuilder::part_with`] — every field defaults
/// to the plain [`part`](StepBuilder::part) behaviour.
#[derive(Debug, Clone, Default)]
pub struct PartOptions {
    /// `PRODUCT.id` (part number); defaults to the part name.
    pub id: Option<String>,
    /// `PRODUCT.description`.
    pub description: Option<String>,
    /// Version label (`PRODUCT_DEFINITION_FORMATION.id`); defaults to `""`.
    pub version: Option<String>,
    /// `PRODUCT_DEFINITION_FORMATION.description`.
    pub version_description: Option<String>,
    /// `PRODUCT_DEFINITION.description`.
    pub definition_description: Option<String>,
}

/// A person at an organization, for [`StepBuilder::person_and_org`].
#[derive(Debug, Clone)]
pub struct PersonOrg {
    pub person_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: String,
}

/// A calendar date with wall-clock time (no time zone — emitted as UTC).
#[derive(Debug, Clone, Copy)]
pub struct DateTimeInput {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: Option<i64>,
}

/// One approval for [`StepBuilder::approve`]: a status word (customarily
/// `"approved"`), a level, any number of approvers with their roles, and an
/// optional approval date.
#[derive(Debug, Clone, Default)]
pub struct ApprovalInput {
    pub status: String,
    pub level: String,
    pub approvers: Vec<(m::PersonAndOrganizationId, String)>,
    pub date: Option<DateTimeInput>,
}

/// An external document reference for [`StepBuilder::document`].
#[derive(Debug, Clone)]
pub struct DocumentInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// `DOCUMENT_TYPE.product_data_type`, e.g. `"drawing"`.
    pub kind: String,
}

/// Normals for a [`MeshInput`], mirroring what the read side exposes: none,
/// one for the whole mesh, or one per vertex (row count must match the
/// point count — a mismatch is the caller's responsibility).
#[derive(Debug, Clone, Default)]
pub enum MeshNormalsInput {
    #[default]
    None,
    Uniform([f64; 3]),
    PerVertex(Vec<[f64; 3]>),
}

/// A display mesh for [`StepBuilder::mesh`]: a vertex array plus 0-based
/// index triangles — the plain form kernels and renderers already hold.
#[derive(Debug, Clone)]
pub struct MeshInput {
    pub points: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub normals: MeshNormalsInput,
}

/// What a [`StepBuilder::style`] call colours: a whole solid (with or without
/// voids) or one face — the targets the read-side `Scene` resolves styles for.
#[derive(Debug, Clone, Copy)]
pub enum StyleTarget {
    Face(m::AdvancedFaceId),
    Solid(m::ManifoldSolidBrepId),
    /// A solid with internal voids ([`StepBuilder::solid_with_voids`]).
    VoidSolid(m::BrepWithVoidsId),
}

/// Which solid a display mesh tessellates ([`StepBuilder::mesh`]): a plain
/// [`solid`](StepBuilder::solid) or one with voids
/// ([`solid_with_voids`](StepBuilder::solid_with_voids)). The id types convert
/// with [`Into`], so `Some(body.into())` works for either.
#[derive(Debug, Clone, Copy)]
pub enum SolidRef {
    Solid(m::ManifoldSolidBrepId),
    VoidSolid(m::BrepWithVoidsId),
}

impl From<m::ManifoldSolidBrepId> for SolidRef {
    fn from(id: m::ManifoldSolidBrepId) -> Self {
        SolidRef::Solid(id)
    }
}

impl From<m::BrepWithVoidsId> for SolidRef {
    fn from(id: m::BrepWithVoidsId) -> Self {
        SolidRef::VoidSolid(id)
    }
}

/// Which way the face normals of the void shells passed to
/// [`StepBuilder::solid_with_voids`] point. The caller declares this so
/// step-io can orient each cavity's `ORIENTED_CLOSED_SHELL` correctly without
/// evaluating geometry. See that method for how to choose.
#[derive(Debug, Clone, Copy)]
pub enum VoidShellNormals {
    /// Normals point away from the solid material — for a cavity, into the
    /// empty hole. This is the same rule the outer shell follows, the
    /// validator- and mass-property-friendly orientation, and what most
    /// kernels emit for a cavity (a reversed shell). Kept as authored,
    /// written as `ORIENTED_CLOSED_SHELL(..., .T.)`. The right choice when unsure.
    AwayFromMaterial,
    /// Normals point into the solid material — a cavity wound like an ordinary
    /// outward-facing solid box dropped into the hole. step-io reverses it,
    /// written as `ORIENTED_CLOSED_SHELL(..., .F.)`.
    TowardMaterial,
}

/// Handle for a vertex created by [`StepBuilder::vertex`] — an index into
/// the builder that created it (a different builder's handle panics or
/// targets the wrong vertex). The builder keeps the vertex position so
/// straight edges can derive their line geometry.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Vertex(usize);

/// Handle for a part created by [`StepBuilder::part`] — an index into the
/// builder that created it. Passing it to a different builder panics (out
/// of range) or silently targets the wrong part (in range).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Part(usize);

/// A part's product chain plus the shape items collected for it; the shape
/// representation itself is materialized in [`StepBuilder::finish`] because
/// representation items are fixed at insertion.
struct PendingPart {
    product: m::ProductId,
    formation: m::ProductDefinitionFormationId,
    definition: m::ProductDefinitionId,
    shape: m::ProductDefinitionShapeId,
    origin: m::Axis2Placement3dId,
    items: Vec<m::RepresentationItemRef>,
    meshes: Vec<m::TessellatedSolidId>,
    /// The placements of this part's children, by index.
    placements: Vec<usize>,
}

/// One [`StepBuilder::place`] call: the eagerly-created usage occurrence
/// plus the wiring deferred to [`StepBuilder::finish`] (the transformation
/// relationship needs both parts' shape representations, which exist only
/// then).
struct PendingPlacement {
    nauo: m::NextAssemblyUsageOccurrenceId,
    parent: usize,
    child: usize,
    to: m::Axis2Placement3dId,
}

/// Ergonomic writer for the standard AP242 file shape: contexts and units up
/// front, one product chain per [`part`](Self::part), everything wired up by
/// [`finish`](Self::finish).
///
/// # Errors
///
/// A method that fails returns an [`AuthorError`]:
/// - [`NonFiniteReal`](AuthorError::NonFiniteReal) for a NaN or infinite
///   coordinate, length or other real, which Part 21 cannot write;
/// - [`InvalidInput`](AuthorError::InvalidInput) for a NURBS curve or surface,
///   or a mesh, that breaks the standard's rules (see [`edge`](Self::edge),
///   [`face`](Self::face) and [`mesh`](Self::mesh));
/// - [`DanglingRef`](AuthorError::DanglingRef) for an id out of this
///   builder's range. An id from another builder that happens to be in range
///   is not caught, and stands for whatever this builder holds there;
/// - [`Cardinality`](AuthorError::Cardinality) for an empty list where the
///   standard needs at least one item, such as a solid without faces.
///
/// Any other error indicates a bug in the builder itself.
pub struct StepBuilder {
    author: Ap242Author,
    /// The geometric context of every part not placed in an assembly, and of
    /// the presentation; a placed part gets one of its own.
    ctx: m::ComplexUnitId,
    /// The length uncertainty and the units every geometric context shares.
    uncertainty: m::UncertaintyMeasureWithUnitId,
    units: [m::ComplexUnitId; 3],
    product_context: m::ProductContextId,
    pd_context: m::ProductDefinitionContextId,
    parts: Vec<PendingPart>,
    placements: Vec<PendingPlacement>,
    vertices: Vec<(m::VertexPointId, [f64; 3])>,
    styles: Vec<m::StyledItemId>,
    hidden: Vec<m::InvisibleItemRef>,
    header: HeaderInput,
}

impl StepBuilder {
    /// Set up the shared skeleton with the default units (millimetre,
    /// 1e-7 uncertainty) — see [`new_with`](Self::new_with) to choose.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn new() -> Result<Self, AuthorError> {
        Self::new_with(&UnitsInput::default())
    }

    /// Set up the shared skeleton: application context + protocol definition
    /// (the AP242 identity values), product contexts, and the geometric
    /// context complex carrying the chosen SI length unit, radian,
    /// steradian, and the given length uncertainty. A part placed in an
    /// assembly gets a context of its own over the same units at
    /// [`finish`](Self::finish).
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn new_with(units: &UnitsInput) -> Result<Self, AuthorError> {
        let mut a = Ap242Author::new();

        let ac = a.add_application_context("managed model based 3d engineering".to_owned())?;
        a.add_application_protocol_definition(
            "international standard".to_owned(),
            "ap242_managed_model_based_3d_engineering_mim_lf".to_owned(),
            2020,
            m::ApplicationContextRef::ApplicationContext(ac),
        )?;
        let product_context = a.add_product_context(
            String::new(),
            m::ApplicationContextRef::ApplicationContext(ac),
            "mechanical".to_owned(),
        )?;
        let pd_context = a.add_product_definition_context(
            "part definition".to_owned(),
            m::ApplicationContextRef::ApplicationContext(ac),
            "design".to_owned(),
        )?;

        let length_prefix = match units.length {
            LengthUnit::Millimetre => Some(m::SiPrefix::Milli),
            LengthUnit::Metre => None,
        };
        let mm = a.add_complex(vec![
            m::UnitPart::LengthUnit,
            m::UnitPart::NamedUnit { dimensions: None },
            m::UnitPart::SiUnit {
                prefix: length_prefix,
                name: m::SiUnitName::Metre,
            },
        ])?;
        let rad = a.add_complex(vec![
            m::UnitPart::NamedUnit { dimensions: None },
            m::UnitPart::PlaneAngleUnit,
            m::UnitPart::SiUnit {
                prefix: None,
                name: m::SiUnitName::Radian,
            },
        ])?;
        let sr = a.add_complex(vec![
            m::UnitPart::NamedUnit { dimensions: None },
            m::UnitPart::SiUnit {
                prefix: None,
                name: m::SiUnitName::Steradian,
            },
            m::UnitPart::SolidAngleUnit,
        ])?;
        let length_measure = a.text("LENGTH_MEASURE");
        let uncertainty = a.add_uncertainty_measure_with_unit(
            m::MeasureValue {
                type_name: Some(length_measure),
                value: m::MeasureScalar::Real(units.uncertainty),
            },
            m::UnitRef::Complex(mm),
            "distance_accuracy_value".to_owned(),
            Some("confusion accuracy".to_owned()),
        )?;
        let units = [mm, rad, sr];
        let ctx = geometric_context(&mut a, uncertainty, units)?;

        Ok(Self {
            author: a,
            ctx,
            uncertainty,
            units,
            product_context,
            pd_context,
            parts: Vec::new(),
            placements: Vec::new(),
            vertices: Vec::new(),
            styles: Vec::new(),
            hidden: Vec::new(),
            header: HeaderInput::default(),
        })
    }

    /// Set the Part 21 HEADER fields (file name, description, authors,
    /// organizations, originating system, authorisation, timestamp) applied
    /// on [`finish`](Self::finish). May be called any time before `finish`;
    /// the last call wins. Without it the header carries only the automatic
    /// timestamp and preprocessor stamp.
    pub fn header(&mut self, h: &HeaderInput) {
        self.header = h.clone();
    }

    /// Escape hatch to the strict low-level layer — the write-side
    /// counterpart of the read side's `Scene::model()`. Entities the builder
    /// does not cover go through [`Ap242Author`]'s generated constructors
    /// directly; ids cross freely in both directions (builder-produced ids
    /// feed low-level calls and vice versa — same model, same validation).
    ///
    /// Only borrowed access is offered: taking the author out would bypass
    /// [`finish`](Self::finish), which still has to materialize the pending
    /// shape representations, placements, style anchors, category, and
    /// header.
    pub fn author(&mut self) -> &mut Ap242Author {
        &mut self.author
    }

    /// Colour `target` (a solid or one face), optionally with a transparency
    /// (`0.0` = opaque, `1.0` = fully transparent). Styling the same target
    /// again adds another style record; readers use the first they find.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn style(
        &mut self,
        target: StyleTarget,
        color: Rgb,
        transparency: Option<f64>,
    ) -> Result<m::StyledItemId, AuthorError> {
        let a = &mut self.author;
        let rgb = a.add_colour_rgb(String::new(), color.red, color.green, color.blue)?;
        // Colour alone rides the customary fill-area chain; with a
        // transparency the rendering form carries both.
        let element = if let Some(t) = transparency {
            let transparent = a.add_surface_style_transparent(t)?;
            let rendering = a.add_surface_style_rendering_with_properties(
                m::ShadingSurfaceMethod::NormalShading,
                m::ColourRef::ColourRgb(rgb),
                vec![m::RenderingPropertiesSelectRef::SurfaceStyleTransparent(
                    transparent,
                )],
            )?;
            m::SurfaceStyleElementSelectRef::SurfaceStyleRenderingWithProperties(rendering)
        } else {
            let fill_colour =
                a.add_fill_area_style_colour(String::new(), m::ColourRef::ColourRgb(rgb))?;
            let fill_style = a.add_fill_area_style(
                String::new(),
                vec![m::FillStyleSelectRef::FillAreaStyleColour(fill_colour)],
            )?;
            let fill_area =
                a.add_surface_style_fill_area(m::FillAreaStyleRef::FillAreaStyle(fill_style))?;
            m::SurfaceStyleElementSelectRef::SurfaceStyleFillArea(fill_area)
        };
        let side_style = a.add_surface_side_style(String::new(), vec![element])?;
        let usage = a.add_surface_style_usage(
            m::SurfaceSide::Both,
            m::SurfaceSideStyleSelectRef::SurfaceSideStyle(side_style),
        )?;
        let assignment = a.add_presentation_style_assignment(vec![
            m::PresentationStyleSelectRef::SurfaceStyleUsage(usage),
        ])?;
        let item = match target {
            StyleTarget::Face(f) => m::StyledItemTargetRef::AdvancedFace(f),
            StyleTarget::Solid(s) => m::StyledItemTargetRef::ManifoldSolidBrep(s),
            StyleTarget::VoidSolid(s) => m::StyledItemTargetRef::BrepWithVoids(s),
        };
        let styled = a.add_styled_item(
            String::new(),
            vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(assignment)],
            item,
        )?;
        self.styles.push(styled);
        Ok(styled)
    }

    /// Assign `targets` to a named presentation layer; the returned id can
    /// be fed to [`hide_layer`](Self::hide_layer) to switch the whole layer
    /// off by default.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn layer(
        &mut self,
        name: &str,
        targets: Vec<StyleTarget>,
    ) -> Result<m::PresentationLayerAssignmentId, AuthorError> {
        let items = targets
            .into_iter()
            .map(|t| match t {
                StyleTarget::Face(f) => m::LayeredItemRef::AdvancedFace(f),
                StyleTarget::Solid(s) => m::LayeredItemRef::ManifoldSolidBrep(s),
                StyleTarget::VoidSolid(s) => m::LayeredItemRef::BrepWithVoids(s),
            })
            .collect();
        self.author
            .add_presentation_layer_assignment(name.to_owned(), String::new(), items)
    }

    /// Mark `target` invisible by default. Implemented the way STEP requires
    /// it: an `INVISIBILITY` cannot point at geometry directly, so the
    /// builder attaches an empty (`.NULL.`-styled) styled item to the target
    /// and registers that — colour styles on the same target are unaffected.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn hide(&mut self, target: StyleTarget) -> Result<(), AuthorError> {
        let a = &mut self.author;
        let assignment =
            a.add_presentation_style_assignment(vec![m::PresentationStyleSelectRef::NullStyle(
                m::NullStyle::Null,
            )])?;
        let item = match target {
            StyleTarget::Face(f) => m::StyledItemTargetRef::AdvancedFace(f),
            StyleTarget::Solid(s) => m::StyledItemTargetRef::ManifoldSolidBrep(s),
            StyleTarget::VoidSolid(s) => m::StyledItemTargetRef::BrepWithVoids(s),
        };
        let styled = a.add_styled_item(
            String::new(),
            vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(assignment)],
            item,
        )?;
        self.styles.push(styled);
        self.hidden.push(m::InvisibleItemRef::StyledItem(styled));
        Ok(())
    }

    /// Switch a whole layer (from [`layer`](Self::layer)) invisible by
    /// default. Infallible at call time — an id out of this builder's range
    /// is caught by the `INVISIBILITY` validation inside
    /// [`finish`](Self::finish); one from another builder is not.
    pub fn hide_layer(&mut self, layer: m::PresentationLayerAssignmentId) {
        self.hidden
            .push(m::InvisibleItemRef::PresentationLayerAssignment(layer));
    }

    /// `AXIS2_PLACEMENT_3D` from a [`Frame`].
    fn placement(&mut self, f: &Frame) -> Result<m::Axis2Placement3dId, AuthorError> {
        let a = &mut self.author;
        let location = a.add_cartesian_point(String::new(), f.origin.into())?;
        let axis = a.add_direction(String::new(), f.axis.into())?;
        let ref_direction = a.add_direction(String::new(), f.ref_dir.into())?;
        a.add_axis2_placement3d(
            String::new(),
            m::CartesianPointRef::CartesianPoint(location),
            Some(m::DirectionRef::Direction(axis)),
            Some(m::DirectionRef::Direction(ref_direction)),
        )
    }

    /// One `CARTESIAN_POINT` ref per control point.
    fn control_points(
        &mut self,
        row: &[[f64; 3]],
    ) -> Result<Vec<m::CartesianPointRef>, AuthorError> {
        row.iter()
            .map(|p| {
                self.author
                    .add_cartesian_point(String::new(), (*p).into())
                    .map(m::CartesianPointRef::CartesianPoint)
            })
            .collect()
    }

    /// Geometry for a swept-surface profile (also backs the ellipse edge).
    fn profile_geometry(&mut self, profile: &ProfileInput) -> Result<m::CurveRef, AuthorError> {
        match profile {
            ProfileInput::Line(point, dir) => {
                let a = &mut self.author;
                let pnt = a.add_cartesian_point(String::new(), (*point).into())?;
                let orientation = a.add_direction(String::new(), (*dir).into())?;
                let vector =
                    a.add_vector(String::new(), m::DirectionRef::Direction(orientation), 1.0)?;
                let line = a.add_line(
                    String::new(),
                    m::CartesianPointRef::CartesianPoint(pnt),
                    m::VectorRef::Vector(vector),
                )?;
                Ok(m::CurveRef::Line(line))
            }
            ProfileInput::Circle(frame, radius) => {
                let position = self.placement(frame)?;
                let circle = self.author.add_circle(
                    String::new(),
                    m::Axis2PlacementRef::Axis2Placement3d(position),
                    *radius,
                )?;
                Ok(m::CurveRef::Circle(circle))
            }
            ProfileInput::Ellipse(frame, semi_1, semi_2) => {
                let position = self.placement(frame)?;
                let ellipse = self.author.add_ellipse(
                    String::new(),
                    m::Axis2PlacementRef::Axis2Placement3d(position),
                    *semi_1,
                    *semi_2,
                )?;
                Ok(m::CurveRef::Ellipse(ellipse))
            }
            ProfileInput::Nurbs(curve) => self.nurbs_curve_geometry(curve),
        }
    }

    /// NURBS curve geometry: non-rational (all weights `1.0`) as a plain
    /// `B_SPLINE_CURVE_WITH_KNOTS`, rational as the customary multi-part
    /// complex instance.
    // Exact comparison intended: only weights that are exactly 1.0 take the
    // non-rational form; anything else keeps its value in weights_data.
    #[allow(clippy::float_cmp)]
    fn nurbs_curve_geometry(&mut self, c: &NurbsCurve) -> Result<m::CurveRef, AuthorError> {
        check_bspline(c.degree, c.control_points.len(), &c.knots)?;
        check_weights(c.control_points.len(), &c.weights)?;
        let cps = self.control_points(&c.control_points)?;
        let (knots, knot_multiplicities) = compress_knots(&c.knots);
        let degree = i64::try_from(c.degree).unwrap_or(i64::MAX);
        if c.weights.iter().all(|w| *w == 1.0) {
            let id = self.author.add_b_spline_curve_with_knots(
                String::new(),
                degree,
                cps,
                m::BSplineCurveForm::Unspecified,
                m::Logical::Unknown,
                m::Logical::Unknown,
                knot_multiplicities,
                knots,
                m::KnotType::Unspecified,
            )?;
            Ok(m::CurveRef::BSplineCurveWithKnots(id))
        } else {
            let no_name = self.author.text("");
            let id = self.author.add_complex(vec![
                m::UnitPart::BoundedCurve,
                m::UnitPart::BSplineCurve {
                    degree,
                    control_points_list: cps,
                    curve_form: m::BSplineCurveForm::Unspecified,
                    closed_curve: m::Logical::Unknown,
                    self_intersect: m::Logical::Unknown,
                },
                m::UnitPart::BSplineCurveWithKnots {
                    knot_multiplicities,
                    knots,
                    knot_spec: m::KnotType::Unspecified,
                },
                m::UnitPart::Curve,
                m::UnitPart::GeometricRepresentationItem,
                m::UnitPart::RationalBSplineCurve {
                    weights_data: c.weights.clone(),
                },
                m::UnitPart::RepresentationItem { name: no_name },
            ])?;
            Ok(m::CurveRef::Complex(id))
        }
    }

    /// NURBS surface geometry; see [`Self::nurbs_curve_geometry`].
    #[allow(clippy::float_cmp)] // exact 1.0 check, as in nurbs_curve_geometry
    fn nurbs_surface_geometry(&mut self, s: &NurbsSurface) -> Result<m::SurfaceRef, AuthorError> {
        let n_u = s.control_points.len();
        let n_v = s.control_points.first().map_or(0, Vec::len);
        if n_v == 0 || s.control_points.iter().any(|row| row.len() != n_v) {
            return Err(AuthorError::InvalidInput {
                reason: "B-spline surface control points are not a full grid",
            });
        }
        check_bspline(s.degree_u, n_u, &s.knots_u)?;
        check_bspline(s.degree_v, n_v, &s.knots_v)?;
        if s.weights.len() != n_u {
            return Err(AuthorError::InvalidInput {
                reason: "B-spline weights do not match its control points",
            });
        }
        for row in &s.weights {
            check_weights(n_v, row)?;
        }
        let mut cps = Vec::with_capacity(s.control_points.len());
        for row in &s.control_points {
            cps.push(self.control_points(row)?);
        }
        let (u_knots, u_multiplicities) = compress_knots(&s.knots_u);
        let (v_knots, v_multiplicities) = compress_knots(&s.knots_v);
        let u_degree = i64::try_from(s.degree_u).unwrap_or(i64::MAX);
        let v_degree = i64::try_from(s.degree_v).unwrap_or(i64::MAX);
        if s.weights.iter().flatten().all(|w| *w == 1.0) {
            let id = self.author.add_b_spline_surface_with_knots(
                String::new(),
                u_degree,
                v_degree,
                cps,
                m::BSplineSurfaceForm::Unspecified,
                m::Logical::Unknown,
                m::Logical::Unknown,
                m::Logical::Unknown,
                u_multiplicities,
                v_multiplicities,
                u_knots,
                v_knots,
                m::KnotType::Unspecified,
            )?;
            Ok(m::SurfaceRef::BSplineSurfaceWithKnots(id))
        } else {
            let no_name = self.author.text("");
            let id = self.author.add_complex(vec![
                m::UnitPart::BoundedSurface,
                m::UnitPart::BSplineSurface {
                    u_degree,
                    v_degree,
                    control_points_list: cps,
                    surface_form: m::BSplineSurfaceForm::Unspecified,
                    u_closed: m::Logical::Unknown,
                    v_closed: m::Logical::Unknown,
                    self_intersect: m::Logical::Unknown,
                },
                m::UnitPart::BSplineSurfaceWithKnots {
                    u_multiplicities,
                    v_multiplicities,
                    u_knots,
                    v_knots,
                    knot_spec: m::KnotType::Unspecified,
                },
                m::UnitPart::GeometricRepresentationItem,
                m::UnitPart::RationalBSplineSurface {
                    weights_data: s.weights.clone(),
                },
                m::UnitPart::RepresentationItem { name: no_name },
                m::UnitPart::Surface,
            ])?;
            Ok(m::SurfaceRef::Complex(id))
        }
    }

    /// Add a topological vertex at `p`.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn vertex(&mut self, p: [f64; 3]) -> Result<Vertex, AuthorError> {
        let point = self.author.add_cartesian_point(String::new(), p.into())?;
        let vp = self
            .author
            .add_vertex_point(String::new(), m::PointRef::CartesianPoint(point))?;
        self.vertices.push((vp, p));
        Ok(Vertex(self.vertices.len() - 1))
    }

    /// Add an edge between two vertices over the given curve. A straight
    /// edge derives its line from the vertex positions (a zero-length
    /// segment gets a placeholder direction — degenerate geometry is the
    /// caller's responsibility) or takes the caller's exact direction
    /// ([`CurveInput::LineAlong`]), written with a unit-magnitude `VECTOR`; a
    /// circle with `from == to` is a closed full-circle edge.
    ///
    /// # Errors
    /// [`AuthorError::InvalidInput`] for a [`CurveInput::Nurbs`] with a
    /// B-spline degree below 1, no more control points than the degree, a knot
    /// count other than control points + degree + 1, decreasing knots, a knot
    /// repeated more than the degree allows (degree + 1 at an end), or weights
    /// that are not one positive value per control point; for a
    /// [`CurveInput::Parabola`] with a zero focal distance or a
    /// [`CurveInput::Hyperbola`] with a semi-axis not positive; and for either
    /// when the two vertices sit at the same parameter, so the edge has no
    /// span. See also [the builder's errors](Self#errors).
    pub fn edge(
        &mut self,
        from: Vertex,
        to: Vertex,
        curve: CurveInput,
    ) -> Result<m::EdgeCurveId, AuthorError> {
        let (start_vertex, start) = self.vertices[from.0];
        let (end_vertex, end) = self.vertices[to.0];
        let mut same_sense = true;
        let geometry = match curve {
            CurveInput::Line => {
                let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
                let len = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                let dir = if len < 1e-12 {
                    [0.0, 0.0, 1.0]
                } else {
                    [delta[0] / len, delta[1] / len, delta[2] / len]
                };
                self.profile_geometry(&ProfileInput::Line(start, dir))?
            }
            CurveInput::LineAlong(dir) => {
                let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
                let along = dir[0] * delta[0] + dir[1] * delta[1] + dir[2] * delta[2];
                let dir = if along < 0.0 { dir.map(|c| -c) } else { dir };
                self.profile_geometry(&ProfileInput::Line(start, dir))?
            }
            CurveInput::Circle(frame, radius) => {
                let position = self.placement(&frame)?;
                let circle = self.author.add_circle(
                    String::new(),
                    m::Axis2PlacementRef::Axis2Placement3d(position),
                    radius,
                )?;
                m::CurveRef::Circle(circle)
            }
            CurveInput::Ellipse(frame, semi_1, semi_2) => {
                self.profile_geometry(&ProfileInput::Ellipse(frame, semi_1, semi_2))?
            }
            CurveInput::Parabola(frame, focal) => {
                if focal == 0.0 {
                    return Err(AuthorError::InvalidInput {
                        reason: "parabola with a zero focal distance",
                    });
                }
                // The edge runs with the curve when its parameter
                // y / (2·focal) grows from start to end.
                same_sense = open_conic_ahead(&frame, start, end, focal)?;
                let position = self.placement(&frame)?;
                let parabola = self.author.add_parabola(
                    String::new(),
                    m::Axis2PlacementRef::Axis2Placement3d(position),
                    focal,
                )?;
                m::CurveRef::Parabola(parabola)
            }
            CurveInput::Hyperbola(frame, semi_axis, semi_imag_axis) => {
                if !(semi_axis > 0.0 && semi_imag_axis > 0.0) {
                    return Err(AuthorError::InvalidInput {
                        reason: "hyperbola semi-axes not positive",
                    });
                }
                // Its parameter asinh(y / semi_imag_axis) grows with y.
                same_sense = open_conic_ahead(&frame, start, end, 1.0)?;
                let position = self.placement(&frame)?;
                let hyperbola = self.author.add_hyperbola(
                    String::new(),
                    m::Axis2PlacementRef::Axis2Placement3d(position),
                    semi_axis,
                    semi_imag_axis,
                )?;
                m::CurveRef::Hyperbola(hyperbola)
            }
            CurveInput::Polyline(points) => {
                let refs = self.control_points(&points)?;
                m::CurveRef::Polyline(self.author.add_polyline(String::new(), refs)?)
            }
            CurveInput::Nurbs(curve) => self.nurbs_curve_geometry(&curve)?,
        };
        self.author.add_edge_curve(
            String::new(),
            m::VertexRef::VertexPoint(start_vertex),
            m::VertexRef::VertexPoint(end_vertex),
            geometry,
            same_sense,
        )
    }

    /// Emit the geometry entity for a [`SurfaceInput`].
    fn surface_geometry(&mut self, surface: SurfaceInput) -> Result<m::SurfaceRef, AuthorError> {
        Ok(match surface {
            SurfaceInput::Plane(frame) => {
                let position = self.placement(&frame)?;
                let plane = self.author.add_plane(
                    String::new(),
                    m::Axis2Placement3dRef::Axis2Placement3d(position),
                )?;
                m::SurfaceRef::Plane(plane)
            }
            SurfaceInput::Cylinder(frame, radius) => {
                let position = self.placement(&frame)?;
                let cyl = self.author.add_cylindrical_surface(
                    String::new(),
                    m::Axis2Placement3dRef::Axis2Placement3d(position),
                    radius,
                )?;
                m::SurfaceRef::CylindricalSurface(cyl)
            }
            SurfaceInput::Sphere(frame, radius) => {
                let position = self.placement(&frame)?;
                let sphere = self.author.add_spherical_surface(
                    String::new(),
                    m::Axis2Placement3dRef::Axis2Placement3d(position),
                    radius,
                )?;
                m::SurfaceRef::SphericalSurface(sphere)
            }
            SurfaceInput::Torus(frame, major_radius, minor_radius) => {
                let position = self.placement(&frame)?;
                let torus = self.author.add_toroidal_surface(
                    String::new(),
                    m::Axis2Placement3dRef::Axis2Placement3d(position),
                    major_radius,
                    minor_radius,
                )?;
                m::SurfaceRef::ToroidalSurface(torus)
            }
            SurfaceInput::Cone(frame, radius, semi_angle) => {
                let position = self.placement(&frame)?;
                let cone = self.author.add_conical_surface(
                    String::new(),
                    m::Axis2Placement3dRef::Axis2Placement3d(position),
                    radius,
                    semi_angle,
                )?;
                m::SurfaceRef::ConicalSurface(cone)
            }
            SurfaceInput::LinearExtrusion(profile, sweep) => {
                let swept_curve = self.profile_geometry(&profile)?;
                let len = (sweep[0] * sweep[0] + sweep[1] * sweep[1] + sweep[2] * sweep[2]).sqrt();
                let dir = if len < 1e-12 {
                    [0.0, 0.0, 1.0]
                } else {
                    [sweep[0] / len, sweep[1] / len, sweep[2] / len]
                };
                let a = &mut self.author;
                let orientation = a.add_direction(String::new(), dir.into())?;
                let vector =
                    a.add_vector(String::new(), m::DirectionRef::Direction(orientation), len)?;
                let surf = a.add_surface_of_linear_extrusion(
                    String::new(),
                    swept_curve,
                    m::VectorRef::Vector(vector),
                )?;
                m::SurfaceRef::SurfaceOfLinearExtrusion(surf)
            }
            SurfaceInput::Revolution(profile, origin, axis) => {
                let swept_curve = self.profile_geometry(&profile)?;
                let a = &mut self.author;
                let location = a.add_cartesian_point(String::new(), origin.into())?;
                let axis_dir = a.add_direction(String::new(), axis.into())?;
                let position = a.add_axis1_placement(
                    String::new(),
                    m::CartesianPointRef::CartesianPoint(location),
                    Some(m::DirectionRef::Direction(axis_dir)),
                )?;
                let surf = a.add_surface_of_revolution(
                    String::new(),
                    swept_curve,
                    m::Axis1PlacementRef::Axis1Placement(position),
                )?;
                m::SurfaceRef::SurfaceOfRevolution(surf)
            }
            SurfaceInput::Nurbs(surface) => self.nurbs_surface_geometry(&surface)?,
        })
    }

    /// Add a face over an analytic surface, bounded by the given loops;
    /// `orientation` says whether the face's normal is its surface's normal
    /// ([`Forward`](Orientation::Forward)) or against it.
    ///
    /// # Errors
    /// [`AuthorError::InvalidInput`] for a NURBS surface or profile with a
    /// B-spline degree below 1, no more control points than the degree, a knot
    /// count other than control points + degree + 1, decreasing knots, a knot
    /// repeated more than the degree allows (degree + 1 at an end), or weights
    /// that are not one positive value per control point, or a surface whose control points are not a full grid.
    /// See also [the builder's errors](Self#errors).
    pub fn face(
        &mut self,
        surface: SurfaceInput,
        orientation: Orientation,
        bounds: Vec<FaceBoundInput>,
    ) -> Result<m::AdvancedFaceId, AuthorError> {
        let face_geometry = self.surface_geometry(surface)?;
        let mut face_bounds = Vec::with_capacity(bounds.len());
        for bound in bounds {
            let mut edge_list = Vec::with_capacity(bound.edges.len());
            for (edge, along) in bound.edges {
                let oe = self.author.add_oriented_edge(
                    String::new(),
                    m::EdgeRef::EdgeCurve(edge),
                    along.flag(),
                )?;
                edge_list.push(m::OrientedEdgeRef::OrientedEdge(oe));
            }
            let el = self.author.add_edge_loop(String::new(), edge_list)?;
            let fb = if bound.outer {
                m::FaceBoundRef::FaceOuterBound(self.author.add_face_outer_bound(
                    String::new(),
                    m::LoopRef::EdgeLoop(el),
                    bound.orientation.flag(),
                )?)
            } else {
                m::FaceBoundRef::FaceBound(self.author.add_face_bound(
                    String::new(),
                    m::LoopRef::EdgeLoop(el),
                    bound.orientation.flag(),
                )?)
            };
            face_bounds.push(fb);
        }
        self.author.add_advanced_face(
            String::new(),
            face_bounds,
            face_geometry,
            orientation.flag(),
        )
    }

    /// Close the faces into a shell and add the solid to `part`; the solid
    /// lands in the part's shape representation on [`finish`](Self::finish)
    /// (as an `ADVANCED_BREP_SHAPE_REPRESENTATION`).
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn solid(
        &mut self,
        part: Part,
        name: &str,
        faces: Vec<m::AdvancedFaceId>,
    ) -> Result<m::ManifoldSolidBrepId, AuthorError> {
        let shell = self.author.add_closed_shell(
            String::new(),
            faces.into_iter().map(m::FaceRef::AdvancedFace).collect(),
        )?;
        let solid = self
            .author
            .add_manifold_solid_brep(name.to_owned(), m::ClosedShellRef::ClosedShell(shell))?;
        self.parts[part.0]
            .items
            .push(m::RepresentationItemRef::ManifoldSolidBrep(solid));
        Ok(solid)
    }

    /// Close `outer_faces` into the outer shell and each group in `voids` into
    /// an internal cavity shell, then add a solid with internal voids
    /// (`BREP_WITH_VOIDS`) to `part` — a cavity-bearing body such as a hollow
    /// casting. The solid lands in the part's shape representation on
    /// [`finish`](Self::finish) (as an `ADVANCED_BREP_SHAPE_REPRESENTATION`,
    /// like [`solid`](Self::solid)).
    ///
    /// `voids` must be non-empty (a solid with no voids is a plain
    /// [`solid`](Self::solid)); each inner group is one closed cavity shell.
    ///
    /// # Void shell orientation (`normals`)
    ///
    /// STEP orients every bounding shell so its face normals point away from
    /// the solid material: outward into free space for the outer shell, and
    /// into the empty cavity for a void. Each void group is wrapped in an
    /// [`ORIENTED_CLOSED_SHELL`](m::OrientedClosedShell); `normals` tells
    /// step-io whether the faces you pass already follow that rule or must be
    /// flipped to it.
    ///
    /// - [`AwayFromMaterial`](VoidShellNormals::AwayFromMaterial): the faces
    ///   already point out of the solid (into the cavity) — the same rule your
    ///   outer shell follows. Natural, and what most kernels produce when they
    ///   emit a cavity as a reversed shell; kept as authored (`.T.`). Use this
    ///   if your kernel orients cavities consistently with the outer shell.
    ///   When in doubt, this is the right choice.
    /// - [`TowardMaterial`](VoidShellNormals::TowardMaterial): the faces point
    ///   into the material — you built the cavity like an ordinary
    ///   outward-facing box (normals facing outward from the cavity volume,
    ///   toward the surrounding solid). step-io reverses them (`.F.`).
    ///
    /// Quick test: look along a cavity wall's normal. Into the empty hole →
    /// `AwayFromMaterial`; into the surrounding solid → `TowardMaterial`.
    /// step-io does not evaluate geometry — it trusts this declaration, and the
    /// wrong value inverts the void (validators and mass-property sign
    /// conventions will reject it).
    ///
    /// # Errors
    /// Empty `voids`, or empty face lists, fail with
    /// [`AuthorError::Cardinality`] (the strict `CLOSED_SHELL` /
    /// `BREP_WITH_VOIDS` cardinality checks). See also
    /// [the builder's errors](Self#errors).
    pub fn solid_with_voids(
        &mut self,
        part: Part,
        name: &str,
        outer_faces: Vec<m::AdvancedFaceId>,
        voids: Vec<Vec<m::AdvancedFaceId>>,
        normals: VoidShellNormals,
    ) -> Result<m::BrepWithVoidsId, AuthorError> {
        let outer = self.author.add_closed_shell(
            String::new(),
            outer_faces
                .into_iter()
                .map(m::FaceRef::AdvancedFace)
                .collect(),
        )?;
        // `.T.` keeps the faces as authored, `.F.` reverses them; see
        // `conditional_reverse` in the schema.
        let orientation = match normals {
            VoidShellNormals::AwayFromMaterial => true, // .T. — as authored (natural)
            VoidShellNormals::TowardMaterial => false,  // .F. — topology_reversed
        };
        let mut void_refs = Vec::with_capacity(voids.len());
        for void_faces in voids {
            let shell = self.author.add_closed_shell(
                String::new(),
                void_faces
                    .into_iter()
                    .map(m::FaceRef::AdvancedFace)
                    .collect(),
            )?;
            let oriented = self.author.add_oriented_closed_shell(
                String::new(),
                m::ClosedShellRef::ClosedShell(shell),
                orientation,
            )?;
            void_refs.push(m::OrientedClosedShellRef::OrientedClosedShell(oriented));
        }
        let solid = self.author.add_brep_with_voids(
            name.to_owned(),
            m::ClosedShellRef::ClosedShell(outer),
            void_refs,
        )?;
        self.parts[part.0]
            .items
            .push(m::RepresentationItemRef::BrepWithVoids(solid));
        Ok(solid)
    }

    /// Add one part: the full product chain (product → formation →
    /// definition → definition shape) plus its origin placement. Shape items
    /// collected for the part are bound into its shape representation by
    /// [`finish`](Self::finish).
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn part(&mut self, name: &str) -> Result<Part, AuthorError> {
        self.part_with(name, &PartOptions::default())
    }

    /// [`part`](Self::part) with metadata overrides: part number, version
    /// label, and the description fields.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn part_with(&mut self, name: &str, opts: &PartOptions) -> Result<Part, AuthorError> {
        let origin = self.placement(&Frame {
            origin: [0.0; 3],
            axis: [0.0, 0.0, 1.0],
            ref_dir: [1.0, 0.0, 0.0],
        })?;
        let a = &mut self.author;
        let product = a.add_product(
            opts.id.clone().unwrap_or_else(|| name.to_owned()),
            name.to_owned(),
            opts.description.clone(),
            vec![m::ProductContextRef::ProductContext(self.product_context)],
        )?;
        let formation = a.add_product_definition_formation(
            opts.version.clone().unwrap_or_default(),
            opts.version_description.clone(),
            m::ProductRef::Product(product),
        )?;
        let definition = a.add_product_definition(
            "design".to_owned(),
            opts.definition_description.clone(),
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(formation),
            m::ProductDefinitionContextRef::ProductDefinitionContext(self.pd_context),
        )?;
        let shape = a.add_product_definition_shape(
            String::new(),
            None,
            m::CharacterizedDefinitionRef::ProductDefinition(definition),
        )?;

        self.parts.push(PendingPart {
            product,
            formation,
            definition,
            shape,
            origin,
            items: Vec::new(),
            meshes: Vec::new(),
            placements: Vec::new(),
        });
        Ok(Part(self.parts.len() - 1))
    }

    /// Attach a display mesh to `part`, optionally linked to the b-rep solid
    /// it tessellates (the link is what `MeshGroup::solid()` resolves on the
    /// read side). Triangle indices are 0-based into `points`; the builder
    /// converts to STEP's 1-based strip encoding. The mesh lands in its own
    /// `TESSELLATED_SHAPE_REPRESENTATION` on [`finish`](Self::finish).
    ///
    /// # Errors
    /// [`AuthorError::InvalidInput`] for a mesh without triangles, with a
    /// triangle index not below the number of points, or with per-vertex
    /// normals that do not number the points.
    /// See also [the builder's errors](Self#errors).
    pub fn mesh(
        &mut self,
        part: Part,
        name: &str,
        input: &MeshInput,
        of_solid: Option<SolidRef>,
    ) -> Result<m::TessellatedSolidId, AuthorError> {
        let n = input.points.len();
        let bad = |reason| Err(AuthorError::InvalidInput { reason });
        if input.triangles.is_empty() {
            return bad("mesh without triangles");
        }
        if input.triangles.iter().flatten().any(|&i| i >= n) {
            return bad("mesh triangle index past its points");
        }
        if matches!(&input.normals, MeshNormalsInput::PerVertex(rows) if rows.len() != n) {
            return bad("mesh per-vertex normals do not match its points");
        }
        let a = &mut self.author;
        let npoints = i64::try_from(n).unwrap_or(i64::MAX);
        let coordinates = a.add_coordinates_list(
            String::new(),
            npoints,
            input.points.iter().map(|&p| p.into()).collect(),
        )?;
        let normals: Vec<m::Reals3> = match &input.normals {
            MeshNormalsInput::None => Vec::new(),
            MeshNormalsInput::Uniform(n) => vec![(*n).into()],
            MeshNormalsInput::PerVertex(rows) => rows.iter().map(|&n| n.into()).collect(),
        };
        // One 3-index strip per triangle, 1-based. The read side's strip
        // decode swaps the last two indices of the first window, so encode
        // (p, q, r) as [p, r, q] to get the original winding back.
        let to_ix = |v: usize| i64::try_from(v + 1).unwrap_or(i64::MAX);
        let triangle_strips: Vec<Vec<i64>> = input
            .triangles
            .iter()
            .map(|&[p, q, r]| vec![to_ix(p), to_ix(r), to_ix(q)])
            .collect();
        let face = a.add_complex_triangulated_face(
            name.to_owned(),
            m::CoordinatesListRef::CoordinatesList(coordinates),
            npoints,
            normals,
            None,
            Vec::new(),
            triangle_strips,
            Vec::new(),
        )?;
        let solid = a.add_tessellated_solid(
            name.to_owned(),
            vec![m::TessellatedStructuredItemRef::ComplexTriangulatedFace(
                face,
            )],
            of_solid.map(|s| match s {
                SolidRef::Solid(id) => m::ManifoldSolidBrepRef::ManifoldSolidBrep(id),
                SolidRef::VoidSolid(id) => m::ManifoldSolidBrepRef::BrepWithVoids(id),
            }),
        )?;
        self.parts[part.0].meshes.push(solid);
        Ok(solid)
    }

    /// A person at an organization — reusable across
    /// [`contributor`](Self::contributor) and approver roles.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn person_and_org(
        &mut self,
        p: &PersonOrg,
    ) -> Result<m::PersonAndOrganizationId, AuthorError> {
        let a = &mut self.author;
        let person = a.add_person(
            p.person_id.clone(),
            p.last_name.clone(),
            p.first_name.clone(),
            None,
            None,
            None,
        )?;
        let organization = a.add_organization(None, p.organization.clone(), None)?;
        a.add_person_and_organization(
            m::PersonRef::Person(person),
            m::OrganizationRef::Organization(organization),
        )
    }

    /// Record `who` as a contributor to the part's version under the given
    /// role (customary roles: `"creator"`, `"design_owner"`, `"checker"`).
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn contributor(
        &mut self,
        part: Part,
        role: &str,
        who: m::PersonAndOrganizationId,
    ) -> Result<(), AuthorError> {
        let a = &mut self.author;
        let role = a.add_person_and_organization_role(role.to_owned())?;
        a.add_applied_person_and_organization_assignment(
            m::PersonAndOrganizationRef::PersonAndOrganization(who),
            m::PersonAndOrganizationRoleRef::PersonAndOrganizationRole(role),
            vec![m::PersonAndOrganizationItemRef::ProductDefinitionFormation(
                self.parts[part.0].formation,
            )],
        )?;
        Ok(())
    }

    /// Attach an approval to the part's version: a status word, a level,
    /// any approvers with their roles, and an optional approval date.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn approve(
        &mut self,
        part: Part,
        input: &ApprovalInput,
    ) -> Result<m::ApprovalId, AuthorError> {
        let a = &mut self.author;
        let status = a.add_approval_status(input.status.clone())?;
        let approval = a.add_approval(
            m::ApprovalStatusRef::ApprovalStatus(status),
            input.level.clone(),
        )?;
        a.add_applied_approval_assignment(
            m::ApprovalRef::Approval(approval),
            vec![m::ApprovalItemRef::ProductDefinitionFormation(
                self.parts[part.0].formation,
            )],
        )?;
        for (who, role) in &input.approvers {
            let role = a.add_approval_role(role.clone())?;
            a.add_approval_person_organization(
                m::PersonOrganizationSelectRef::PersonAndOrganization(*who),
                m::ApprovalRef::Approval(approval),
                m::ApprovalRoleRef::ApprovalRole(role),
            )?;
        }
        if let Some(date) = input.date {
            let calendar = a.add_calendar_date(date.year, date.day, date.month)?;
            let zone = a.add_coordinated_universal_time_offset(0, None, m::AheadOrBehind::Exact)?;
            let time = a.add_local_time(
                date.hour,
                date.minute,
                None,
                m::CoordinatedUniversalTimeOffsetRef::CoordinatedUniversalTimeOffset(zone),
            )?;
            let date_time = a.add_date_and_time(
                m::DateRef::CalendarDate(calendar),
                m::LocalTimeRef::LocalTime(time),
            )?;
            a.add_approval_date_time(
                m::DateTimeSelectRef::DateAndTime(date_time),
                m::ApprovalRef::Approval(approval),
            )?;
        }
        Ok(approval)
    }

    /// Attach an external document reference to the part's version.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn document(&mut self, part: Part, d: &DocumentInput) -> Result<(), AuthorError> {
        let a = &mut self.author;
        let kind = a.add_document_type(d.kind.clone())?;
        let doc = a.add_document(
            d.id.clone(),
            d.name.clone(),
            d.description.clone(),
            m::DocumentTypeRef::DocumentType(kind),
        )?;
        a.add_applied_document_reference(
            m::DocumentRef::Document(doc),
            String::new(),
            vec![m::DocumentReferenceItemRef::ProductDefinitionFormation(
                self.parts[part.0].formation,
            )],
        )?;
        Ok(())
    }

    /// Attach a security classification to the part's version.
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn security(
        &mut self,
        part: Part,
        name: &str,
        purpose: &str,
        level: &str,
    ) -> Result<(), AuthorError> {
        let a = &mut self.author;
        let level = a.add_security_classification_level(level.to_owned())?;
        let classification = a.add_security_classification(
            name.to_owned(),
            purpose.to_owned(),
            m::SecurityClassificationLevelRef::SecurityClassificationLevel(level),
        )?;
        a.add_applied_security_classification_assignment(
            m::SecurityClassificationRef::SecurityClassification(classification),
            vec![
                m::SecurityClassificationItemRef::ProductDefinitionFormation(
                    self.parts[part.0].formation,
                ),
            ],
        )?;
        Ok(())
    }

    /// Place `child` inside `parent` at the given frame — one assembly
    /// occurrence (the same child may be placed any number of times; each
    /// call is one instance). The usage occurrence is created immediately;
    /// its shape wiring is bound in [`finish`](Self::finish).
    ///
    /// # Errors
    /// [`AuthorError::CyclicAssembly`] if `child` is `parent` or already
    /// contains it, as an assembly may not contain itself; nothing is added
    /// then. Otherwise see [the builder's errors](Self#errors).
    pub fn place(
        &mut self,
        parent: Part,
        child: Part,
        at: Frame,
    ) -> Result<m::NextAssemblyUsageOccurrenceId, AuthorError> {
        if self.contains(child.0, parent.0) {
            return Err(AuthorError::CyclicAssembly);
        }
        let nauo = self.author.add_next_assembly_usage_occurrence(
            format!("{}", self.placements.len() + 1),
            String::new(),
            None,
            m::ProductDefinitionOrReferenceRef::ProductDefinition(self.parts[parent.0].definition),
            m::ProductDefinitionOrReferenceRef::ProductDefinition(self.parts[child.0].definition),
            None,
        )?;
        let to = self.placement(&at)?;
        self.parts[parent.0].placements.push(self.placements.len());
        self.placements.push(PendingPlacement {
            nauo,
            parent: parent.0,
            child: child.0,
            to,
        });
        Ok(nauo)
    }

    /// Whether part `whole` is part `part` or contains it through the
    /// placements so far.
    fn contains(&self, whole: usize, part: usize) -> bool {
        let mut seen = HashSet::new();
        let mut stack = vec![whole];
        while let Some(i) = stack.pop() {
            if i == part {
                return true;
            }
            if seen.insert(i) {
                stack.extend(
                    self.parts[i]
                        .placements
                        .iter()
                        .map(|&p| self.placements[p].child),
                );
            }
        }
        false
    }

    /// Materialize each part's shape representation (its origin placement
    /// plus the collected items) with its shape-definition link, add the
    /// customary product category, and return the finished model — AP242,
    /// ready for [`write`](crate::write).
    ///
    /// # Errors
    /// See [the builder's errors](Self#errors).
    pub fn finish(mut self) -> Result<m::StepModel, AuthorError> {
        let a = &mut self.author;
        // Pass 1: one shape representation per part, kept by part index for
        // the placement wiring below.
        let mut part_reps: Vec<m::RepresentationOrRepresentationReferenceRef> =
            Vec::with_capacity(self.parts.len());
        // A placement relates representations in different contexts, so a
        // part placed in an assembly gets a context of its own.
        let mut placed = vec![false; self.parts.len()];
        for placement in &self.placements {
            placed[placement.child] = true;
        }
        for (i, part) in self.parts.iter().enumerate() {
            let ctx = if placed[i] {
                geometric_context(a, self.uncertainty, self.units)?
            } else {
                self.ctx
            };
            let children = part.placements.iter().map(|&p| self.placements[p].to);
            part_reps.push(bind_part(a, part, children, ctx)?);
        }
        // Pass 2: assembly placements.
        bind_placements(a, &self.parts, &self.placements, &part_reps)?;
        if !self.parts.is_empty() {
            a.add_product_related_product_category(
                "part".to_owned(),
                None,
                self.parts
                    .iter()
                    .map(|p| m::ProductRef::Product(p.product))
                    .collect(),
            )?;
        }
        // Hidden items (individual styled items and whole layers) collect
        // into one INVISIBILITY.
        if !self.hidden.is_empty() {
            a.add_invisibility(std::mem::take(&mut self.hidden))?;
        }
        // Styles are anchored in the customary presentation representation.
        if !self.styles.is_empty() {
            a.add_mechanical_design_geometric_presentation_representation(
                String::new(),
                self.styles
                    .iter()
                    .map(|&s| m::RepresentationItemRef::StyledItem(s))
                    .collect(),
                m::RepresentationContextRef::Complex(self.ctx),
            )?;
        }
        let h = &self.header;
        let file_header = FileHeader {
            description: h.description.clone().unwrap_or_default(),
            file_name: h.file_name.clone().unwrap_or_default(),
            time_stamp: h.timestamp.clone().unwrap_or_else(now_iso_utc),
            authors: h.authors.clone(),
            organizations: h.organizations.clone(),
            preprocessor_version: concat!("step-io ", env!("CARGO_PKG_VERSION")).to_owned(),
            originating_system: h.originating_system.clone().unwrap_or_default(),
            authorisation: h.authorisation.clone().unwrap_or_default(),
            // The authoring layer stamps the AP242 schema identity in
            // `finish_with_header`; whatever is set here is overwritten.
            schema: crate::parser::SchemaId::default(),
        };
        Ok(self.author.finish_with_header(&file_header))
    }
}

/// One part's shape representation in `ctx`, linked to the part's
/// definition, plus its tessellated representation if it carries display
/// meshes. A part carrying a solid is emitted as the customary
/// `ADVANCED_BREP_SHAPE_REPRESENTATION`. `children` are the frames the part's
/// children are placed at.
fn bind_part(
    a: &mut Ap242Author,
    part: &PendingPart,
    children: impl Iterator<Item = m::Axis2Placement3dId>,
    ctx: m::ComplexUnitId,
) -> Result<m::RepresentationOrRepresentationReferenceRef, AuthorError> {
    // The origin, then where each child is placed, then the shape.
    let mut items = vec![m::RepresentationItemRef::Axis2Placement3d(part.origin)];
    items.extend(children.map(m::RepresentationItemRef::Axis2Placement3d));
    items.extend(part.items.iter().cloned());
    let has_solid = part.items.iter().any(|i| {
        matches!(
            i,
            m::RepresentationItemRef::ManifoldSolidBrep(_)
                | m::RepresentationItemRef::BrepWithVoids(_)
        )
    });
    let (rep, rep_or_ref) = if has_solid {
        let id = a.add_advanced_brep_shape_representation(
            String::new(),
            items,
            m::RepresentationContextRef::Complex(ctx),
        )?;
        (
            m::RepresentationRef::AdvancedBrepShapeRepresentation(id),
            m::RepresentationOrRepresentationReferenceRef::AdvancedBrepShapeRepresentation(id),
        )
    } else {
        let id = a.add_shape_representation(
            String::new(),
            items,
            m::RepresentationContextRef::Complex(ctx),
        )?;
        (
            m::RepresentationRef::ShapeRepresentation(id),
            m::RepresentationOrRepresentationReferenceRef::ShapeRepresentation(id),
        )
    };
    a.add_shape_definition_representation(
        m::RepresentedDefinitionRef::ProductDefinitionShape(part.shape),
        rep,
    )?;

    // A part with display meshes gets its own tessellated
    // representation alongside the shape one (multi-SDR per shape).
    if !part.meshes.is_empty() {
        let tsr = a.add_tessellated_shape_representation(
            String::new(),
            part.meshes
                .iter()
                .map(|&t| m::RepresentationItemRef::TessellatedSolid(t))
                .collect(),
            m::RepresentationContextRef::Complex(ctx),
        )?;
        a.add_shape_definition_representation(
            m::RepresentedDefinitionRef::ProductDefinitionShape(part.shape),
            m::RepresentationRef::TessellatedShapeRepresentation(tsr),
        )?;
    }
    Ok(rep_or_ref)
}

/// A 3D geometric representation context over the shared length
/// uncertainty and units (length, plane angle, solid angle).
fn geometric_context(
    a: &mut Ap242Author,
    uncertainty: m::UncertaintyMeasureWithUnitId,
    [length, angle, solid_angle]: [m::ComplexUnitId; 3],
) -> Result<m::ComplexUnitId, AuthorError> {
    let (no_identifier, context_type) = (a.text(""), a.text("3D"));
    a.add_complex(vec![
        m::UnitPart::GeometricRepresentationContext {
            coordinate_space_dimension: 3,
        },
        m::UnitPart::GlobalUncertaintyAssignedContext {
            uncertainty: vec![
                m::UncertaintyMeasureWithUnitRef::UncertaintyMeasureWithUnit(uncertainty),
            ],
        },
        m::UnitPart::GlobalUnitAssignedContext {
            units: vec![
                m::UnitRef::Complex(length),
                m::UnitRef::Complex(angle),
                m::UnitRef::Complex(solid_angle),
            ],
        },
        m::UnitPart::RepresentationContext {
            context_identifier: no_identifier,
            context_type,
        },
    ])
}

/// Assembly-placement wiring, deferred to `finish`: each occurrence gets its
/// placement shape (PDS over the NAUO) bound to an item-defined
/// transformation (item 1 = the child's origin, item 2 = where it is placed
/// in the parent, both items of those parts' representations) through the
/// customary relationship complex (`rep_1` = child, `rep_2` = parent).
fn bind_placements(
    a: &mut Ap242Author,
    parts: &[PendingPart],
    placements: &[PendingPlacement],
    part_reps: &[m::RepresentationOrRepresentationReferenceRef],
) -> Result<(), AuthorError> {
    for placement in placements {
        let pds = a.add_product_definition_shape(
            "Placement".to_owned(),
            Some("Placement of an item".to_owned()),
            m::CharacterizedDefinitionRef::NextAssemblyUsageOccurrence(placement.nauo),
        )?;
        let idt = a.add_item_defined_transformation(
            String::new(),
            None,
            m::RepresentationItemRef::Axis2Placement3d(parts[placement.child].origin),
            m::RepresentationItemRef::Axis2Placement3d(placement.to),
        )?;
        let no_name = a.text("");
        let rrwt = a.add_complex(vec![
            m::UnitPart::RepresentationRelationship {
                name: no_name,
                description: None,
                rep_1: part_reps[placement.child].clone(),
                rep_2: part_reps[placement.parent].clone(),
            },
            m::UnitPart::RepresentationRelationshipWithTransformation {
                transformation_operator: m::TransformationRef::ItemDefinedTransformation(idt),
            },
            m::UnitPart::ShapeRepresentationRelationship,
        ])?;
        a.add_context_dependent_shape_representation(
            m::ShapeRepresentationRelationshipRef::Complex(rrwt),
            m::ProductDefinitionShapeRef::ProductDefinitionShape(pds),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::iso_utc_from_unix;

    #[test]
    fn iso_utc_from_unix_known_values() {
        assert_eq!(iso_utc_from_unix(0), "1970-01-01T00:00:00Z");
        // Leap-day boundary: 2024-02-29 23:59:59 UTC.
        assert_eq!(iso_utc_from_unix(1_709_251_199), "2024-02-29T23:59:59Z");
        // The day after: 2024-03-01 00:00:00 UTC.
        assert_eq!(iso_utc_from_unix(1_709_251_200), "2024-03-01T00:00:00Z");
    }
}
