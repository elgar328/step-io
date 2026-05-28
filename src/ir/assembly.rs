//! Assembly IR — product hierarchy reconstructed from STEP AP203/214/242
//! `PRODUCT`, `PRODUCT_DEFINITION`, `SHAPE_DEFINITION_REPRESENTATION` and
//! friends.
//!
//! Each `Product` is either a geometry leaf (`Solid` / `SurfaceBody`) or a
//! group that holds `Instance`s referencing child products. `AssemblyTree`
//! owns the product arena and the resolved root.

use super::arena::Arena;
use super::id::{
    ApplicationContextId, CurveId, MeasureWithUnitId, Placement3dId, PointId, ProductCategoryId,
    ProductContextId, ProductDefinitionContextId, ProductDefinitionContextRoleId, ProductId,
    RepresentationId, ShellId, SolidId, UnitContextId,
};

/// Assembly graph. Conventionally called a "tree" but shared instances
/// make it a DAG in general (the same product can be reached through
/// multiple [`Instance`]s). A single STEP file may also carry several
/// independent top-level products (no NAUO between them), so the graph is
/// a forest in general — hence `roots` is a list.
#[derive(Debug, Clone, Default)]
pub struct AssemblyTree {
    pub products: Arena<Product>,
    /// Every top-level product — one not referenced as any
    /// [`Instance::child`]. One entry for a single assembly, several for a
    /// multi-part file; empty only for a (malformed) fully cyclic graph.
    pub roots: Vec<ProductId>,
    /// `PRODUCT_CONTEXT` / `MECHANICAL_CONTEXT` arena. The writer
    /// currently emits the first entry; additional entries drop
    /// (single-context emit pattern shared with the AC chain).
    pub product_contexts: Arena<ProductContext>,
    /// `PRODUCT_DEFINITION_CONTEXT` / `DESIGN_CONTEXT` arena. Same
    /// single-emit constraint as `product_contexts`.
    pub product_definition_contexts: Arena<ProductDefinitionContext>,
    /// `PRODUCT_DEFINITION_CONTEXT_ROLE` arena. Leaf entries referenced
    /// by `ProductDefinitionContextAssociation`.
    pub product_definition_context_roles: Arena<ProductDefinitionContextRole>,
    /// `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION` arena. Top-level (no
    /// current IR consumer); links a `PRODUCT_DEFINITION` to a
    /// `ProductDefinitionContext` via a role tag.
    pub product_definition_context_associations: Arena<ProductDefinitionContextAssociation>,
    /// `PRODUCT_DEFINITION_RELATIONSHIP` arena, carrying both the plain
    /// supertype form and the `MAKE_FROM_USAGE_OPTION` in-enum subtype.
    pub product_definition_relationships: Arena<ProductDefinitionRelationship>,
    /// `product_category` `enum_base` arena — `PC` `Itself` + `PRPC` variants.
    /// Source-of-truth for the PC cluster (phase pc-unify); the per-product
    /// `Product.category` field is a deprecated mirror kept for kernel API
    /// compatibility.
    pub product_categories: Arena<ProductCategory>,
    /// `PRODUCT_CATEGORY_RELATIONSHIP` arena — pairs a PC `Itself` entry
    /// with a PRPC entry (`sub_category`) per the AP203 / AP242 schema.
    pub product_category_relationships: Arena<ProductCategoryRelationship>,
}

/// `PRODUCT_DEFINITION_RELATIONSHIP` arena entry. Carries the plain base
/// form and the `MAKE_FROM_USAGE_OPTION` subtype as flat enum variants —
/// mirrors the [`crate::ir::visualization::StyledItem`] carrier pattern
/// (inline fields rather than a nested base struct).
#[derive(Debug, Clone, PartialEq)]
pub enum ProductDefinitionRelationship {
    Plain(PlainProductDefinitionRelationship),
    MakeFrom(MakeFromUsageOption),
}

/// Plain `PRODUCT_DEFINITION_RELATIONSHIP(id, name, description, relating, related)`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainProductDefinitionRelationship {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub relating: ProductId,
    pub related: ProductId,
}

/// `MAKE_FROM_USAGE_OPTION` — SUBTYPE OF `PRODUCT_DEFINITION_RELATIONSHIP`
/// adding `ranking`, `ranking_rationale`, `quantity`.
#[derive(Debug, Clone, PartialEq)]
pub struct MakeFromUsageOption {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub relating: ProductId,
    pub related: ProductId,
    pub ranking: i64,
    pub ranking_rationale: String,
    pub quantity: MeasureWithUnitId,
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
    /// Geometry leaf — `Some` when the product carries an `ABSR` / `MSSR` /
    /// `GBWSR` representation. `None` for pure assembly groups (instances
    /// only) and for metadata-only products.
    pub geometry: Option<GeometryLeaf>,
    /// Child instances attached to this product via `NEXT_ASSEMBLY_USAGE_OCCURRENCE`.
    /// Empty for leaf products that aren't also assembly parents. A product can
    /// hold both geometry and instances when the same `PRODUCT` is referenced
    /// as both a part with its own shape and as the parent of further sub-parts.
    pub instances: Vec<Instance>,
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
    /// `PRODUCT_CATEGORY` chain attached to this product. `Some` when the
    /// source file emitted at least a `PRODUCT_RELATED_PRODUCT_CATEGORY`
    /// pointing at this product (the common case in every CAD output).
    /// `None` for the rare minimal fixtures (e.g. AP214 CD) that omit the
    /// chain — writers built from scratch can leave this `None` and the
    /// emitter will skip the chain.
    pub category: Option<ProductCategoryChain>,
    /// `true` when the source file used the `_WITH_SPECIFIED_SOURCE`
    /// subtype of `PRODUCT_DEFINITION_FORMATION`. AP203 always uses this
    /// form (mandatory by schema); AP214/242 use it occasionally — notably
    /// CATIA AP214 IS exports. Writer emits the subtype iff this flag is
    /// `true`. Default `false`.
    pub formation_with_source: bool,
    /// Unit / uncertainty context referenced by this product's shape
    /// representation (`ABSR`, `MSSR`, plain `SHAPE_REPRESENTATION`, `GBWSR`,
    /// `GBSSR`). `Some(id)` indexes into [`crate::ir::model::StepModel::units`].
    /// `None` for kernel-built IR; the writer falls back to the first arena
    /// entry (synthesizing a default if needed).
    pub geometry_context: Option<UnitContextId>,
    /// `PRODUCT_CONTEXT` (or `AP203` `MECHANICAL_CONTEXT`) referenced by
    /// this product's `frame_of_reference`. `None` for kernel-built IR
    /// or files without an explicit PC chain — writer falls back to
    /// IR[0] or synthesised context.
    pub product_context: Option<ProductContextId>,
    /// `PRODUCT_DEFINITION_CONTEXT` (or `AP203` `DESIGN_CONTEXT`)
    /// referenced by this product's `PRODUCT_DEFINITION.frame_of_reference`.
    /// Same fallback semantics as `product_context`.
    pub pdef_context: Option<ProductDefinitionContextId>,
    /// Unified `Representation` arena index for this product's geometry
    /// (the resolved `ABSR` / `MSSR` / wireframe / plain `SHAPE_REPRESENTATION`).
    /// `None` for metadata-only products or kernel-built IR that only sets
    /// `content`. representation-refactor: the writer pre-emits the arena and
    /// dispatches geometry off this id.
    pub representation_id: Option<RepresentationId>,
    /// When the source used the indirect `SDR → plain SR → SRR → ABSR/MSSR`
    /// pattern, the arena index of the outer plain `SHAPE_REPRESENTATION`
    /// wrapper. `None` for the direct form. Pairs with `outer_sr_frame`.
    pub outer_representation_id: Option<RepresentationId>,
}

/// `PRODUCT_CATEGORY` chain attached to a [`Product`] — preserves the source
/// file's category metadata so round-trips stay loyal across CAD vendors.
///
/// The chain is `PRODUCT_RELATED_PRODUCT_CATEGORY` (always), optionally
/// joined to a `PRODUCT_CATEGORY` supertype via a
/// `PRODUCT_CATEGORY_RELATIONSHIP`. `FreeCAD` typically emits PRPC alone;
/// AP203 and CATIA exports include the full triplet.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCategoryChain {
    /// `PRPC.name` — usually `"part"`, occasionally `"detail"` (AP203).
    pub kind: String,
    /// `PRPC.description` — almost always `None` (`$`).
    pub kind_description: Option<String>,
    /// `PCR` + supertype `PC`. `Some` iff the source file emitted both a
    /// `PRODUCT_CATEGORY_RELATIONSHIP` and a `PRODUCT_CATEGORY` pointing
    /// at this PRPC.
    pub root: Option<ProductCategoryRoot>,
}

/// Supertype `PRODUCT_CATEGORY` of a [`ProductCategoryChain`]. Only present
/// when the source file emits a `PRODUCT_CATEGORY_RELATIONSHIP` linking
/// the PRPC to a PC.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCategoryRoot {
    /// `PC.name`. May differ from `ProductCategoryChain.kind` (PRPC.name)
    /// — e.g. AP203 fixtures pair `kind = "detail"` with `name = "part"`.
    pub name: String,
    /// `PC.description`. `Some("specification")` is the most common
    /// non-empty form; FreeCAD-style outputs use `None` (`$`).
    pub description: Option<String>,
}

/// `product_category` `enum_base` (blueprint `instance_count`: `PC` 18833 +
/// `PRPC` 215544). `PC` itself (`Itself`) and `PRPC` are two variants of the
/// same arena — schema-faithful counterpart of the SRR / CGRR / MDDR
/// pattern in [`crate::ir::shape_rep::RepresentationRelationship`].
#[derive(Debug, Clone, PartialEq)]
pub enum ProductCategory {
    Itself(ProductCategoryData),
    ProductRelatedProductCategory(ProductRelatedProductCategoryData),
}

/// `PRODUCT_CATEGORY(name, description)` carrier — blueprint `shape =
/// "carrier"`. Standalone PCs (those not connected to a PRPC via PCR)
/// arrive only through this variant.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCategoryData {
    pub name: String,
    pub description: Option<String>,
}

/// `PRODUCT_RELATED_PRODUCT_CATEGORY(name, description, products)` —
/// EXPRESS SUBTYPE of `PRODUCT_CATEGORY` that adds the `products` field.
/// `name` / `description` are inherited from the PC supertype.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductRelatedProductCategoryData {
    pub name: String,
    pub description: Option<String>,
    pub products: Vec<ProductId>,
}

/// `PRODUCT_CATEGORY_RELATIONSHIP(name, description, category, sub_category)`
/// — blueprint `single_struct`. Both `name` / `description` are `string`
/// (NOT `opt_string` — different from the `PC` / `PRPC` description);
/// empty strings normalise the source's `$` form.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCategoryRelationship {
    pub name: String,
    pub description: String,
    pub category: ProductCategoryId,
    pub sub_category: ProductCategoryId,
}

/// Payload of [`GeometryLeaf::Solid`] — one or more `MANIFOLD_SOLID_BREP`
/// items wrapped in a single `ADVANCED_BREP_SHAPE_REPRESENTATION`. Almost
/// always a single solid; multi-body STEP files (rare) carry more than one.
/// Invariant: non-empty.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidContent {
    pub ids: Vec<SolidId>,
}

/// Payload of [`GeometryLeaf::SurfaceBody`] — the product is a
/// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`'s `SHELL_BASED_SURFACE_MODEL`
/// with one or more shells. Unlike [`SolidContent`], no closed volume is
/// implied; shells are typically `OPEN_SHELL`, occasionally `CLOSED_SHELL`.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceBodyContent {
    pub ids: Vec<ShellId>,
}

/// Geometry payload attached to a [`Product`]. `None` on the product means
/// the product is a pure assembly group (or metadata-only). The three
/// variants mirror the three representation wrappers in STEP.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryLeaf {
    Solid(SolidContent),
    SurfaceBody(SurfaceBodyContent),
    /// Wireframe leaf — the product is a `GEOMETRIC_(CURVE_)SET` of curves
    /// (and optionally loose points) wrapped in a
    /// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` (or its
    /// `..._SURFACE_...` cousin). No surface or solid topology is implied.
    Wireframe(WireframeContent),
}

/// Payload of [`GeometryLeaf::Wireframe`].
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

/// `PRODUCT_CONTEXT` vs `MECHANICAL_CONTEXT` discriminator. The two
/// `AP203` / `AP214e3` entities share identical fields; only the STEP
/// entity name differs at write time (`MECHANICAL_CONTEXT` is an
/// `AP203` subtype with a `discipline_type='mechanical'` `WHERE`
/// constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProductContextKind {
    #[default]
    Plain,
    Mechanical,
}

/// `PRODUCT_CONTEXT(name, frame_of_reference, discipline_type)` per
/// `AP214e3`. `MECHANICAL_CONTEXT` reuses this struct with
/// `kind = Mechanical`; the writer picks the STEP entity name based
/// on `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductContext {
    pub name: String,
    pub frame_of_reference: ApplicationContextId,
    pub discipline_type: String,
    pub kind: ProductContextKind,
}

/// `PRODUCT_DEFINITION_CONTEXT` vs `DESIGN_CONTEXT` discriminator.
/// Same `base_parallel` pattern as `ProductContextKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProductDefinitionContextKind {
    #[default]
    Plain,
    Design,
}

/// `PRODUCT_DEFINITION_CONTEXT(name, frame_of_reference,
/// life_cycle_stage)` per `AP214e3`. `DESIGN_CONTEXT` reuses this
/// struct with `kind = Design`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDefinitionContext {
    pub name: String,
    pub frame_of_reference: ApplicationContextId,
    pub life_cycle_stage: String,
    pub kind: ProductDefinitionContextKind,
}

/// `PRODUCT_DEFINITION_CONTEXT_ROLE(name, description)` per `AP214e3`.
/// Leaf entity referenced by `ProductDefinitionContextAssociation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductDefinitionContextRole {
    pub name: String,
    pub description: Option<String>,
}

/// `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION(definition, frame_of_reference,
/// role)` per `AP214e3`. `definition` references a `PRODUCT_DEFINITION` —
/// step-io maps this to the parent `ProductId` since `PRODUCT_DEFINITION`
/// data is conflated into the `Product` struct (no separate PDEF arena).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductDefinitionContextAssociation {
    pub definition: ProductId,
    pub frame_of_reference: ProductDefinitionContextId,
    pub role: ProductDefinitionContextRoleId,
}
