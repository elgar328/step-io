use super::id::{
    CurveId, EdgeId, FaceId, ShellId, SurfaceCurveSubtypeId, SurfaceId, VertexId, WireId,
};

/// An edge's `edge_geometry` (EXPRESS `edge_curve.edge_geometry : curve`): a
/// plain 3D curve, or a `surface_curve`-family entity (`SURFACE_CURVE` /
/// `SEAM_CURVE` / `BOUNDED_SURFACE_CURVE` / `INTERSECTION_CURVE`) carried as an
/// arena id so its `associated_geometry` / `master_representation` round-trip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeGeometry {
    Curve3d(CurveId),
    SurfaceCurve(SurfaceCurveSubtypeId),
}

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
    /// `edge_geometry` — a plain 3D curve, or a `surface_curve`-family node
    /// (carried as an arena id so its `associated_geometry` round-trips).
    pub edge_geometry: EdgeGeometry,
    pub vertices: (VertexId, VertexId),
    /// Curve parameter range. Placeholder `(0.0, 0.0)` — trim computation
    /// requires projecting vertex positions onto the curve parameterization,
    /// which is a geometric operation deferred to the kernel adapter.
    pub trim: (f64, f64),
    pub orientation: Orientation,
}

/// A reference to an edge with an orientation flag.
///
/// Not stored in an arena — embedded directly in [`Wire::edges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrientedEdge {
    pub edge: EdgeId,
    pub orientation: Orientation,
}

/// The shared payload of a face boundary wire.
///
/// Created from STEP `FACE_BOUND` / `FACE_OUTER_BOUND` whose loop is an
/// `EDGE_LOOP` (normal case) or a `VERTEX_LOOP` (degenerate — a single
/// vertex, as used by spheres and some revolutions). For the vertex-loop
/// case `edges` is empty and [`vertex`](Self::vertex) carries the degenerate
/// point; for edge-loop case the opposite holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireData {
    pub edges: Vec<OrientedEdge>,
    /// Set when the boundary came from a STEP `VERTEX_LOOP`. `None` for
    /// the common `EDGE_LOOP` case.
    pub vertex: Option<VertexId>,
    /// Orientation from the bound entity — indicates whether this wire's
    /// traversal direction agrees with the face's surface normal.
    pub orientation: Orientation,
}

/// A closed or open loop of oriented edges, forming a face boundary.
///
/// The variant records the source STEP entity (`FACE_BOUND` /
/// `FACE_OUTER_BOUND`) so the writer emits it back verbatim; the payload is
/// identical between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wire {
    FaceBound(WireData),
    FaceOuterBound(WireData),
}

impl Wire {
    /// The shared boundary payload, regardless of bound kind.
    #[must_use]
    pub fn data(&self) -> &WireData {
        match self {
            Wire::FaceBound(d) | Wire::FaceOuterBound(d) => d,
        }
    }
}

/// A face — a bounded portion of a surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceData {
    pub surface: SurfaceId,
    pub bounds: Vec<WireId>,
    pub orientation: Orientation,
}

/// A face, tagged with the source STEP entity.
///
/// `AdvancedFace` is a constrained subtype of `FACE_SURFACE` (pcurve
/// required per edge, exactly one outer bound, etc.), but the data fields
/// are identical. step-io does not enforce the `ADVANCED_FACE` WHERE
/// clauses — it preserves whichever entity type the source file used so the
/// writer emits it back verbatim. `FaceSurface` is the less constrained
/// supertype, observed in parts of the ABC dataset and some legacy AP203
/// files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Face {
    FaceSurface(FaceData),
    AdvancedFace(FaceData),
}

impl Face {
    /// The shared face payload, regardless of source entity.
    #[must_use]
    pub fn data(&self) -> &FaceData {
        match self {
            Face::FaceSurface(d) | Face::AdvancedFace(d) => d,
        }
    }
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

/// A solid bounded by one or more shells, tagged with the source STEP
/// entity. `ManifoldSolidBrep` has a single outer shell; `BrepWithVoids`
/// adds inner void shells (each carrying `Orientation::Reversed` when
/// imported from an `ORIENTED_CLOSED_SHELL('', *, cs, .F.)` wrapper) — so
/// the "voids only on `BREP_WITH_VOIDS`" invariant is encoded in the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Solid {
    ManifoldSolidBrep {
        outer: ShellId,
        name: Option<String>,
    },
    BrepWithVoids {
        outer: ShellId,
        voids: Vec<ShellId>,
        name: Option<String>,
    },
}

impl Solid {
    /// The outer bounding shell (`Orientation::Forward`).
    #[must_use]
    pub fn outer(&self) -> ShellId {
        match self {
            Solid::ManifoldSolidBrep { outer, .. } | Solid::BrepWithVoids { outer, .. } => *outer,
        }
    }

    /// The inner void shells — empty for a `ManifoldSolidBrep`.
    #[must_use]
    pub fn voids(&self) -> &[ShellId] {
        match self {
            Solid::ManifoldSolidBrep { .. } => &[],
            Solid::BrepWithVoids { voids, .. } => voids,
        }
    }

    /// The optional solid name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Solid::ManifoldSolidBrep { name, .. } | Solid::BrepWithVoids { name, .. } => {
                name.as_deref()
            }
        }
    }
}
