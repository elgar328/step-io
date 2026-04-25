//! Assembly IR — product hierarchy reconstructed from STEP AP203/214/242
//! `PRODUCT`, `PRODUCT_DEFINITION`, `SHAPE_DEFINITION_REPRESENTATION` and
//! friends.
//!
//! Each `Product` is either a geometry leaf (`Solid` / `SurfaceBody`) or a
//! group that holds `Instance`s referencing child products. `AssemblyTree`
//! owns the product arena and the resolved root.

use super::arena::Arena;
use super::id::{CurveId, Placement3dId, PointId, ProductId, ShellId, SolidId};

/// Assembly graph. Conventionally called a "tree" but shared instances
/// make it a DAG in general (the same product can be reached through
/// multiple [`Instance`]s).
///
/// Phase A leaves `root` as `None`; Phase B resolves the top-level
/// product by walking the NAUO graph.
#[derive(Debug, Clone, Default)]
pub struct AssemblyTree {
    pub products: Arena<Product>,
    /// Phase A: always `None`. Phase B fills this in.
    pub root: Option<ProductId>,
}

/// A single STEP `PRODUCT` with its resolved content.
#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    /// The `id` attribute of the STEP `PRODUCT` entity — a user-facing
    /// identifier such as `"Cube"`. This is *not* the STEP `#N` entity id.
    pub id: String,
    /// The `name` attribute of the STEP `PRODUCT` entity.
    pub name: String,
    /// The `description` attribute of the STEP `PRODUCT` entity. STEP
    /// producers commonly leave this blank (`''`); an empty string is
    /// normalised to `None` so the presence/absence of user-supplied
    /// description round-trips faithfully.
    pub description: Option<String>,
    pub content: ProductContent,
    /// Coordinate frame referenced by the `ADVANCED_BREP_SHAPE_REPRESENTATION`
    /// (or group `SHAPE_REPRESENTATION`) `items` list. Commercial CAD output
    /// uses an identity placement here almost universally; the reader still
    /// preserves whatever the file held. Kernels that construct an IR from
    /// scratch call [`crate::ir::model::GeometryPool::identity_placement`] to
    /// obtain a shared identity id.
    pub shape_ref_frame: Placement3dId,
    /// Indirect SR pattern marker — `Some(p)` means the source file used
    /// `SDR → plain SHAPE_REPRESENTATION → SHAPE_REPRESENTATION_RELATIONSHIP →
    /// ABSR/MSSR` (Fusion 360, some CATIA outputs). `p` is the plain SR's
    /// `items[0]` axis placement, kept loyal to the source. Writer re-emits
    /// the plain SR + SRR wrapper when `Some`. Default `None` produces the
    /// direct `SDR → ABSR/MSSR` form; kernels building an IR from scratch
    /// should leave this `None` unless they specifically want the indirect
    /// output.
    pub outer_sr_frame: Option<Placement3dId>,
}

/// What a [`Product`] holds.
#[derive(Debug, Clone, PartialEq)]
pub enum ProductContent {
    /// Assembly or wrapper product — references other products as
    /// [`Instance`]s. Phase A always produces an empty `Vec`; Phase B
    /// populates it from `NEXT_ASSEMBLY_USAGE_OCCURRENCE` edges.
    Group(Vec<Instance>),
    /// Geometry leaf — the product itself is a single solid.
    Solid(SolidId),
    /// Surface body leaf — the product is a
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`'s `SHELL_BASED_SURFACE_MODEL`
    /// with one or more shells. Unlike `Solid`, no closed volume is implied;
    /// shells are typically `OPEN_SHELL`, occasionally `CLOSED_SHELL`.
    SurfaceBody(Vec<ShellId>),
    /// Wireframe leaf — the product is a `GEOMETRIC_(CURVE_)SET` of curves
    /// (and optionally loose points) wrapped in a
    /// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` (or its
    /// `..._SURFACE_...` cousin). No surface or solid topology is implied.
    Wireframe(WireframeContent),
}

/// Payload of a [`ProductContent::Wireframe`].
///
/// `curves` are the geometric items (lines, circles, trimmed curves, etc.).
/// `points` are loose `CARTESIAN_POINT` items that some producers (notably
/// CATIA) include in `GEOMETRIC_SET` alongside curves; they stay empty for
/// `GEOMETRIC_CURVE_SET`-style outputs. `repr_kind` records which wrapper
/// the source file used so writers can reproduce it.
#[derive(Debug, Clone, PartialEq)]
pub struct WireframeContent {
    pub curves: Vec<CurveId>,
    pub points: Vec<PointId>,
    pub repr_kind: WireframeReprKind,
}

/// Loyalty flag — which `GEOMETRICALLY_BOUNDED_*_SHAPE_REPRESENTATION`
/// wrapper carried this wireframe in the source file. Default is
/// [`WireframeReprKind::Wireframe`]: kernels building an IR from scratch
/// get the more common `..._WIREFRAME_...` form on output.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WireframeReprKind {
    /// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` —
    /// pure wireframe (no associated surface).
    #[default]
    Wireframe,
    /// `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` — wireframe
    /// expressed as a (degenerate) bounded surface representation; CATIA
    /// uses this form for supplemental geometry.
    Surface,
}

/// One placement of a child product inside a parent.
///
/// Only used from Phase B onward; the type is defined in Phase A so the
/// public IR shape (`Group(Vec<Instance>)`) is stable across both phases.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub child: ProductId,
    pub transform: Transform3d,
    /// STEP NAUO `id` attribute (e.g. "1", "23").
    pub occurrence_id: String,
    /// STEP NAUO `name` attribute (e.g. "Cube", "Part003").
    pub occurrence_name: String,
}

/// A rigid 3D placement expressed as STEP does it: two axis placements
/// describing how `source` maps onto `target`. Kept as the literal
/// `ITEM_DEFINED_TRANSFORMATION` payload so the IR can round-trip
/// without introducing floating-point drift. Kernel adapters compute
/// the 4×4 matrix themselves when needed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3d {
    pub source: Placement3dId,
    pub target: Placement3dId,
}
