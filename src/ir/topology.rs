use super::geometry::Pcurve;
use super::id::{CurveId, EdgeId, FaceId, ShellId, SurfaceId, VertexId, WireId};

/// Direction agreement flag used throughout B-Rep topology.
///
/// Maps to STEP's `same_sense` and `orientation` boolean attributes:
/// `.T.` → `Forward`, `.F.` → `Reversed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    Forward,
    Reversed,
}

/// A topological edge — a bounded piece of a curve between two vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub curve: CurveId,
    pub vertices: (VertexId, VertexId),
    /// Curve parameter range. Placeholder `(0.0, 0.0)` — trim computation
    /// requires projecting vertex positions onto the curve parameterization,
    /// which is a geometric operation deferred to the kernel adapter.
    pub trim: (f64, f64),
    pub orientation: Orientation,
    /// Pcurves from the source `SURFACE_CURVE` / `SEAM_CURVE` wrapper.
    /// Empty when the edge's `edge_geometry` pointed directly at a 3D curve.
    pub pcurves: Vec<Pcurve>,
}

/// A reference to an edge with an orientation flag.
///
/// Not stored in an arena — embedded directly in [`Wire::edges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrientedEdge {
    pub edge: EdgeId,
    pub orientation: Orientation,
}

/// A closed or open loop of oriented edges, forming a face boundary.
///
/// Created from STEP `FACE_BOUND` / `FACE_OUTER_BOUND` whose loop is an
/// `EDGE_LOOP` (normal case) or a `VERTEX_LOOP` (degenerate — a single
/// vertex, as used by spheres and some revolutions). For the vertex-loop
/// case `edges` is empty and [`vertex`](Self::vertex) carries the degenerate
/// point; for edge-loop case the opposite holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub edges: Vec<OrientedEdge>,
    /// Set when the boundary came from a STEP `VERTEX_LOOP`. `None` for
    /// the common `EDGE_LOOP` case.
    pub vertex: Option<VertexId>,
    /// `true` when the source entity was `FACE_OUTER_BOUND`.
    pub is_outer: bool,
    /// Orientation from the `FACE_BOUND` entity — indicates whether this
    /// wire's traversal direction agrees with the face's surface normal.
    pub orientation: Orientation,
}

/// Source STEP entity type for a face.
///
/// `ADVANCED_FACE` is a constrained subtype of `FACE_SURFACE` (pcurve
/// required per edge, exactly one outer bound, etc.), but the data
/// fields are identical. step-io does not enforce the `ADVANCED_FACE`
/// WHERE clauses — it preserves whichever entity type the source file
/// used so the writer can emit it back verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FaceKind {
    /// `ADVANCED_FACE` — common output from modern CAD (`FreeCAD`, `CATIA`,
    /// Fusion 360).
    #[default]
    Advanced,
    /// `FACE_SURFACE` — less constrained supertype, observed in parts of
    /// the ABC dataset and some legacy AP203 files.
    General,
}

/// A face — a bounded portion of a surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub surface: SurfaceId,
    pub bounds: Vec<WireId>,
    pub orientation: Orientation,
    pub kind: FaceKind,
}

/// A connected set of faces forming a closed or open shell.
///
/// `is_open` selects between `CLOSED_SHELL` and `OPEN_SHELL` at write time.
/// Solid BREPs only hold closed shells; surface bodies may hold either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    pub faces: Vec<FaceId>,
    pub orientation: Orientation,
    pub is_open: bool,
}

/// A solid bounded by one or more shells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solid {
    /// `shells[0]` is the outer shell with `Orientation::Forward`.
    /// `shells[1..]` are inner void shells from `BREP_WITH_VOIDS`; each
    /// carries `Orientation::Reversed` when imported from an
    /// `ORIENTED_CLOSED_SHELL('', *, cs, .F.)` wrapper.
    pub shells: Vec<ShellId>,
    pub name: Option<String>,
}
