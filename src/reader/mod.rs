//! Converts a raw [`EntityGraph`] into a typed [`StepModel`].
//!
//! This module is the boundary between the parser layer and the IR layer.
//! It converts entities eagerly in reference-dependency (topological)
//! order so that referenced objects are always available when needed.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ir::arena::Arena;
use crate::ir::assembly::{AssemblyTree, Product, Transform3d, WireframeContent};
use crate::ir::error::ConvertError;
use crate::ir::geometry::TransitionCode;
use crate::ir::id::{
    CurveId, Direction2dId, DirectionId, EdgeId, FaceId, Placement1dId, Placement3dId, PointId,
    ProductId, ShellId, SolidId, SurfaceId, UnitContextId, VertexId, WireId,
};
use crate::ir::model::{GeometryPool, StepModel, TopologyPool};
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit, UnitContext};
use crate::ir::topology::{Orientation, OrientedEdge};
use crate::ir::units::{DerivedUnit, DerivedUnitElement, MeasureWithUnit, NamedUnit, UnitsPool};
use crate::ir::visualization::{FillAreaStyleColour, VisualizationPool};
// PreDefinedCurveFontId / CurveStyleId / StyledItemId imported above; the map types reference them directly.
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

mod dispatch;
mod header;
mod id_map_cache;
/// Catalogue of non-standard input normalizations + the typed `NsCase` marker.
mod nonstandard;
pub(crate) use nonstandard::NsCase;

#[cfg(test)]
mod tests;

/// The result of converting an [`EntityGraph`] into a [`StepModel`].
///
/// Conversion always succeeds structurally — individual entity failures are
/// recorded as [`warnings`](ConvertResult::warnings) and the corresponding
/// entities are skipped.
#[derive(Debug)]
pub struct ConvertResult {
    pub model: StepModel,
    /// Each warning describes a single entity that could not be converted;
    /// that entity is silently **omitted from the IR** and conversion
    /// continues for the rest. Messages intentionally omit the
    /// "skipped" verb — the type itself already implies skipping.
    pub warnings: Vec<ConvertError>,
    /// Non-fatal issues observed during the parsing stage (forwarded
    /// from [`EntityGraph::warnings`]). Lenient recoveries (missing
    /// semicolons, blank attribute slots) and discarded P21 edition 3
    /// sections land here. The IR carries the spec-conformant form;
    /// these warnings are the only trace of what was repaired.
    pub parse_warnings: Vec<crate::parser::ParseWarning>,
    /// Canonicalizations applied while reading: **valid** legacy/variant forms
    /// normalized to their modern canonical form (e.g. `QUASI_UNIFORM_CURVE` ->
    /// the canonical `B_SPLINE_CURVE_WITH_KNOTS` NURBS form). Distinct from
    /// `warnings`' non-standard-input recovery — the input was standard, just an
    /// older/alternate encoding. Keyed by the source STEP type that was
    /// canonicalized, value = count. Every canonicalization goes through
    /// [`canon_record`](ReaderContext::canon_record) (`rg "CanonCase::"` finds
    /// all sites). Consumed by round-trip accounting to recognize the source
    /// type's absence from the output as modernization, not loss.
    pub canonicalizations: std::collections::BTreeMap<String, usize>,
}

/// Typed marker for a canonicalization — a **valid** legacy/variant input form
/// normalized to its modern canonical form. The canonical counterpart of
/// [`NsCase`] (which marks non-standard *input* recovery). Every canonicalization
/// flows through [`canon_record`](ReaderContext::canon_record); `rg "CanonCase::"`
/// finds every site, and adding a variant is the deliberate, reviewable way to
/// introduce a new canonicalization (drift guard).
#[derive(Debug, Clone, Copy)]
pub(crate) enum CanonCase {
    /// A b-spline curve form (`QUASI_UNIFORM_CURVE`, …) absorbed into the single
    /// canonical `B_SPLINE_CURVE_WITH_KNOTS` NURBS form on output.
    CurveFormToNurbs,
    /// A b-spline surface form absorbed into `B_SPLINE_SURFACE_WITH_KNOTS`.
    SurfaceFormToNurbs,
}

/// A `SHAPE_DEFINITION_REPRESENTATION` whose product is resolved but whose
/// geometry classification is deferred. The classification follows the
/// indirect chain `SDR -> plain SR -> SHAPE_REPRESENTATION_RELATIONSHIP ->
/// ABSR/MSSR/GBSSR`, but the SDR references only the plain SR — neither the
/// SRR nor the geometry representation — so under topological dispatch the
/// maps it reads (`srr_equiv_map`, `wireframe_data_map`, ...) are not
/// guaranteed populated when the SDR is processed. Resolve in a post-pass.
/// See [`ReaderContext::pending_sdr_geometry`].
pub(crate) struct PendingSdrGeometry {
    pub(crate) pid: ProductId,
    pub(crate) shape_rep_ref: u64,
    pub(crate) entity_id: u64,
    /// STEP id of the `definition` `PRODUCT_DEFINITION_SHAPE` — used to resolve
    /// the `property_definitions` arena slot when a second SDR for the same
    /// product is preserved as a `ShapeDefinitionRepresentationLink`.
    pub(crate) pdef_shape_ref: u64,
}

/// A NAUO assembly instance with its parent/child resolved but its transform
/// still pending. See [`ReaderContext::pending_nauo_instances`].
pub(crate) struct PendingNauoInstance {
    pub(crate) parent: ProductId,
    pub(crate) child: ProductId,
    /// `relating_product_definition` / `related_product_definition` resolved to
    /// `product_definitions` arena ids — the canonical NAUO endpoints. (The
    /// `Instance` view keeps `parent`/`child` as `ProductId`.)
    pub(crate) relating_pd: crate::ir::ProductDefinitionId,
    pub(crate) related_pd: crate::ir::ProductDefinitionId,
    pub(crate) occurrence_id: String,
    pub(crate) occurrence_name: String,
    /// NAUO `description` (attr 2) — kept for the canonical
    /// `assembly_component_usage` arena entry built in `resolve_nauo_instances`.
    pub(crate) description: String,
    /// NAUO `reference_designator` (attr 5) — `None` when unset (`$`).
    pub(crate) reference_designator: Option<String>,
    /// STEP id of the NAUO — the key into `nauo_transform_map`, and the
    /// `entity_id` reported if no transform was found.
    pub(crate) nauo_id: u64,
}

/// Assembly placement RR-complex payload, stashed by the CDSR handler until
/// `resolve_nauo_instances` materialises it into the
/// `representation_relationships` arena. `rep_1`/`rep_2` are the parent/child
/// `SHAPE_REPRESENTATION`s (already resolved to `RepresentationId`);
/// `rr_complex_entity` is the source complex's `#N` (the `style_context` target
/// key).
pub(crate) struct AssemblyRrData {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) rep_1: crate::ir::RepresentationId,
    pub(crate) rep_2: crate::ir::RepresentationId,
    pub(crate) rr_complex_entity: u64,
}

/// A NAUO-targeted `PRODUCT_DEFINITION_SHAPE`'s payload, recorded by the PDS
/// handler's NAUO branch: the target NAUO's `#N` plus the source
/// `name`/`description` (so `materialize_nauo_owned_pds` preserves the
/// exporter's placement label, e.g. "Placement #462", instead of the assembly
/// chain's hard-coded "Placement" / "Placement of an item").
pub(crate) struct NauoPdsInfo {
    pub(crate) nauo: u64,
    pub(crate) name: String,
    pub(crate) description: String,
}

/// A `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM` whose `style_context` SELECT
/// list is resolved in a post-pass. The targets are overwhelmingly assembly
/// RR-complexes, which only gain a `RepresentationRelationshipId` after
/// `resolve_nauo_instances` materialises them — so resolution is deferred until
/// then (mirrors [`PendingNauoInstance`]). The styled item is already pushed
/// with an empty `style_context`; the post-pass patches it in place.
pub(crate) struct PendingCdorsiStyleContext {
    pub(crate) styled_item_id: crate::ir::id::StyledItemId,
    pub(crate) context_refs: Vec<u64>,
    pub(crate) entity_id: u64,
}

/// A `GEOMETRIC_ITEM_SPECIFIC_USAGE` whose `used_representation` was the
/// non-standard `$` (CATIA). Its other refs are resolved during dispatch; the
/// `used_representation` is derived post-dispatch from the representation that
/// contains `identified_item` (the schema's WHERE rule). See
/// [`ReaderContext::deferred_gisu_used_repr`].
pub(crate) struct DeferredGisu {
    pub(crate) entity_id: u64,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) definition: crate::ir::ShapeAspectRef,
    pub(crate) identified_item: crate::ir::representation_item::RepresentationItemRef,
}

/// Accumulates converted IR objects and tracks the mapping from STEP entity
/// ids (`#N`) to typed arena Ids.
#[derive(Default)]
// `solid_angle_unit_map` below has a zero-sized value type; keep it as a
// map for symmetry with the other unit maps and to leave room for future
// `SolidAngleUnit` variants.
#[allow(clippy::zero_sized_map_values)]
pub struct ReaderContext {
    /// Type-keyed `file_id (#N) → arena id` cache. Replaces the former
    /// per-type `*_id_map: HashMap<u64, XId>` fields for every simple arena-id
    /// map (see [`id_map_cache`]). Maps with non-id values or a shared arena-id
    /// type keep their own named fields below.
    pub(crate) id_cache: id_map_cache::IdMapCache,
    /// Transient L1 (`EarlyModel`) store for entities migrated to the 2-layer
    /// path (currently `surface_style_fill_area` / `surface_side_style`). Holds
    /// the read-side L1→L2 correspondence consulted during `lower`. See
    /// [`crate::early`].
    pub(crate) early: crate::early::EarlyModel,
    pub(crate) geometry: GeometryPool,
    pub(crate) topology: TopologyPool,
    /// Unit / uncertainty contexts — one entry per `REPRESENTATION_CONTEXT`
    /// complex entity in the source file.
    pub(crate) units: Arena<UnitContext>,
    /// Unit-less `(GRC PRC REP_CONTEXT)` complex MI contexts (phase
    /// unitless-context). Populated by `ParametricRepresentationContextHandler`.
    pub(crate) unitless_contexts: Arena<crate::ir::shape_rep::UnitlessContext>,
    /// `GEOMETRIC_ITEM_SPECIFIC_USAGE` arena (phase gisu). Populated by
    /// `GeometricItemSpecificUsageHandler`.
    pub(crate) geometric_item_specific_usages:
        Arena<crate::ir::shape_rep::GeometricItemSpecificUsage>,

    /// GISUs whose `used_representation` was a non-standard `$`, deferred so
    /// `resolve_deferred_gisu_used_representation` can derive it from the
    /// `identified_item`'s containing representation after all reps are read.
    pub(crate) deferred_gisu_used_repr: Vec<DeferredGisu>,

    /// `REPRESENTATION_CONTEXT #N → UnitContextId`.
    /// Used by representation converters (ABSR, MSSR, plain SR, GBWSR, GBSSR,
    /// MDGPR) to translate their `context_of_items` ref into an `UnitContextId`.
    pub(crate) context_id_map: HashMap<u64, UnitContextId>,

    /// `representation #N → UnitContextId` for the 5 product-bearing
    /// representation entities (ABSR / MSSR / plain SR / GBWSR / GBSSR).
    /// Single map suffices because STEP entity ids are globally unique within
    /// a file. The SDR pass reads this map to attach `Product.geometry_context`.
    /// MDGPR resolves directly into `Mdgpr.context` and does not use this map.
    pub(crate) repr_context_map: HashMap<u64, UnitContextId>,

    /// Entity ids inside any `DEFINITIONAL_REPRESENTATION` subtree (PCURVE
    /// parametric-space geometry). 3D passes skip them so their 2D
    /// `CARTESIAN_POINT` / `DIRECTION` / `LINE` / … don't collide with 3D
    /// conversion. The 2D-curve handlers then walk the same set to populate
    /// the 2D arenas (`points_2d`, `directions_2d`, `curves_2d`).
    pub(crate) pcurve_subtree_ids: HashSet<u64>,

    /// Ids already accounted for by the `NsCase::Pcurve3dInPspace`
    /// normalization (see `surface_curve.rs`). A 3D curve in a 2D pcurve
    /// parameter-space (EXPRESS `pcurve.wr3` violation) drops the whole
    /// `DEFINITIONAL_REPRESENTATION` subtree; this set dedups the per-type
    /// `ns_record` accounting when several pcurves share a subtree id.
    pub(crate) dropped_pcurve_recorded: HashSet<u64>,

    // Unit entity maps: STEP #N → resolved unit variant.
    pub(crate) length_unit_map: HashMap<u64, LengthUnit>,
    pub(crate) angle_unit_map: HashMap<u64, AngleUnit>,
    pub(crate) solid_angle_unit_map: HashMap<u64, SolidAngleUnit>,
    pub(crate) mass_unit_map: HashMap<u64, crate::ir::units::MassUnit>,
    /// Per-instance `NAMED_UNIT` arena (units-1). Populated by the
    /// `LENGTH` / `PLANE_ANGLE` / `SOLID_ANGLE` / `MASS` unit complex
    /// handlers alongside their existing flavour-specific maps.
    pub(crate) named_units_arena: Arena<NamedUnit>,
    /// `MEASURE_WITH_UNIT` arena (units-1). Populated by the `MEASURE_WITH_UNIT` handlers.
    pub(crate) mwu_arena: Arena<MeasureWithUnit>,
    /// `DERIVED_UNIT_ELEMENT` arena (units-1). Populated by the `DERIVED_UNIT_ELEMENT` handlers.
    pub(crate) due_arena: Arena<DerivedUnitElement>,

    /// `DERIVED_UNIT` arena (units-1b). Populated by the `DERIVED_UNIT` handler.
    pub(crate) derived_unit_arena: Arena<DerivedUnit>,
    /// `DIMENSIONAL_EXPONENTS` arena (phase dim-exp-arena-a). Populated by a
    /// dedicated `DIMENSIONAL_EXPONENTS` handler so `NAMED_UNIT` subtypes can
    /// look up the ref in [`Self::dim_exp_id_map`].
    pub(crate) dimensional_exponents: Arena<crate::ir::units::DimensionalExponents>,

    // Geometry maps: STEP #N → typed Id.

    // Geometry intermediate maps.
    pub(crate) placement_map: HashMap<u64, Placement3dId>,
    pub(crate) vector_map: HashMap<u64, (DirectionId, f64)>,

    // 2D geometry (PCURVE parametric space) maps.
    pub(crate) vector_2d_map: HashMap<u64, (Direction2dId, f64)>,

    // Topology maps: STEP #N → typed Id.
    pub(crate) vertex_map: HashMap<u64, VertexId>,

    // Topology intermediate maps.
    pub(crate) oriented_edge_map: HashMap<u64, OrientedEdge>,
    pub(crate) edge_loop_map: HashMap<u64, Vec<OrientedEdge>>,
    /// `VERTEX_LOOP #N → VertexId`. `FACE_BOUND` consults this when the
    /// loop ref is not in `edge_loop_map`.
    pub(crate) vertex_loop_map: HashMap<u64, VertexId>,
    /// `ORIENTED_CLOSED_SHELL #N → (underlying CLOSED_SHELL's ShellId,
    /// wrapper orientation)`. Populated by the `ORIENTED_CLOSED_SHELL` handler,
    /// consumed by `convert_brep_with_voids`.
    pub(crate) oriented_closed_shell_map: HashMap<u64, (ShellId, Orientation)>,

    // Assembly: product arena + lookup maps.
    // `assembly` is filled in `convert()` if any PRODUCT was seen.
    pub(crate) assembly: Option<AssemblyTree>,
    pub(crate) assembly_products: Arena<Product>,
    /// Canonical `assembly_component_usage` (NAUO) arena, accumulated by
    /// `resolve_nauo_instances` and `mem::take`-n into the `AssemblyTree` at
    /// finalize. Each entry is 1:1 with a built `Instance` (via `Instance::acu`).
    pub(crate) assembly_component_usages: Arena<crate::ir::NextAssemblyUsageOccurrence>,
    /// NAUO step id → the standalone placement `SHAPE_REPRESENTATION`s linked by
    /// extra `SHAPE_DEFINITION_REPRESENTATION`s to the instance's NAUO-owned PDS
    /// (a single placement PDS can carry several). Recorded by the SDR handler;
    /// attached to `Instance::placement_representation`.
    pub(crate) nauo_placement_sr: HashMap<u64, Vec<crate::ir::id::RepresentationId>>,
    /// Canonical `product_definition` arena, populated by the `PRODUCT_DEFINITION`
    /// handler and `mem::take`-n into the `AssemblyTree` at finalize. Each
    /// `Product.pdef` indexes here.
    pub(crate) product_definitions: Arena<crate::ir::ProductDefinition>,

    pub(crate) product_contexts: Arena<crate::ir::ProductContext>,
    pub(crate) product_definition_contexts: Arena<crate::ir::ProductDefinitionContext>,

    /// `ProductId → raw STEP entity id of the product's first PC`
    /// (`PRODUCT.frame_of_reference[0]`). `resolve_product_contexts`
    /// converts these refs to `ProductContextId`.
    pub(crate) product_pc_step_refs: HashMap<ProductId, u64>,
    /// Same pattern, populated by `PRODUCT_DEFINITION.frame_of_reference`.
    pub(crate) product_pdc_step_refs: HashMap<ProductId, u64>,
    pub(crate) product_definition_context_roles: Arena<crate::ir::ProductDefinitionContextRole>,
    pub(crate) product_definition_context_associations:
        Arena<crate::ir::ProductDefinitionContextAssociation>,

    pub(crate) product_definition_relationships: Arena<crate::ir::ProductDefinitionRelationship>,

    /// `product_category` `enum_base` arena (phase pc-unify).
    pub(crate) product_categories: Arena<crate::ir::assembly::ProductCategory>,
    /// `PRODUCT_CATEGORY_RELATIONSHIP` arena.
    pub(crate) product_category_relationships:
        Arena<crate::ir::assembly::ProductCategoryRelationship>,
    /// `PRODUCT_CATEGORY` STEP `#N → ProductCategoryId` (`Itself` variant).
    pub(crate) pc_arena_map: HashMap<u64, crate::ir::ProductCategoryId>,
    /// `PRODUCT_RELATED_PRODUCT_CATEGORY` STEP `#N → ProductCategoryId`
    /// (`ProductRelatedProductCategory` variant).
    pub(crate) prpc_arena_map: HashMap<u64, crate::ir::ProductCategoryId>,
    /// STEP ids of `PRODUCT_RELATED_PRODUCT_CATEGORY` entities dropped because
    /// their `products` SET was empty `()` (non-standard — `SET[1:?]`). A
    /// `PRODUCT_CATEGORY_RELATIONSHIP` whose `sub_category` points here is
    /// dropped as a normalization (relates a non-standard empty category).
    pub(crate) empty_prrpc_refs: HashSet<u64>,
    /// STEP ids of entities dropped (by a Simple handler) because a required
    /// reference was *dangling* — it points to an id the file never defines
    /// (e.g. `#0`, or the `#18446744073709551615` sentinel some anonymizers
    /// emit for a scrubbed person). Such a drop is malformed *input*, not a
    /// step-io coverage gap, so the dispatcher records it as a `NonStandardInput`
    /// normalization rather than a `MissingReference` defect, and seeds this set
    /// so anything transitively requiring the dropped entity cascades the same
    /// way. See `reader::nonstandard` (`NS-dangling-reference-drop`).
    pub(crate) nonstandard_dropped_refs: HashSet<u64>,

    /// `PRODUCT_DEFINITION_FORMATION` arena (carrier enum), `mem::take`-n into
    /// `AssemblyTree` at finalize. Source of truth for version metadata.
    pub(crate) product_definition_formations:
        crate::ir::Arena<crate::ir::assembly::ProductDefinitionFormation>,

    pub(crate) absr_solid_map: HashMap<u64, Vec<SolidId>>,
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION #N → Placement3dId` for the first
    /// `AXIS2_PLACEMENT_3D` item in the ABSR's `items` list (its coordinate
    /// reference frame). Consumed by SDR conversion to populate
    /// `Product.shape_ref_frame`.
    pub(crate) absr_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `SHELL_BASED_SURFACE_MODEL #N → resolved shell ids`. Populated by the
    /// `SHELL_BASED_SURFACE_MODEL` handler and consumed by MSSR conversion to
    /// flatten shells.
    pub(crate) sbsm_shells_map: HashMap<u64, Vec<ShellId>>,
    /// `SHELL_BASED_SURFACE_MODEL #N → GeometricRepresentationItemId`.
    /// Same SBSM, dual-indexed so `STYLED_ITEM` can resolve a standalone
    /// SBSM target through `geometric_representation_items` (phase
    /// sbsm-repr-item) while MSSR keeps using `sbsm_shells_map`.
    pub(crate) sbsm_id_map: HashMap<u64, crate::ir::id::GeometricRepresentationItemId>,
    /// `GEOMETRIC_CURVE_SET` / `GEOMETRIC_SET #N → GeometricRepresentationItemId`.
    /// Same role as `sbsm_id_map`: `STYLED_ITEM` resolves a standalone
    /// GCS/GS target through the unified GRI arena while `curve_set_map`
    /// keeps feeding the wireframe flatten path (phase gcs-cluster).
    pub(crate) curve_set_id_map: HashMap<u64, crate::ir::id::GeometricRepresentationItemId>,
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION #N → flattened shell ids`
    /// pulled from the MSSR's referenced SBSM. Consumed by SDR conversion
    /// to populate `Product.content = SurfaceBody(..)`.
    pub(crate) mssr_shells_map: HashMap<u64, Vec<ShellId>>,
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION #N → Placement3dId` — same
    /// role as `absr_ref_frame_map` but for the MSSR path. Optional because
    /// some writers omit the AXIS2 item.
    pub(crate) mssr_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `plain SHAPE_REPRESENTATION #N → target ABSR/MSSR #N` — built from
    /// simple `SHAPE_REPRESENTATION_RELATIONSHIP` entities where exactly one
    /// side resolves to a known ABSR/MSSR. Consumed by SDR conversion to
    /// follow the Fusion 360 / CATIA indirection chain
    /// `SDR → plain SR → SRR → ABSR/MSSR`. Deliberately NOT a typed L1 probe:
    /// both key and value are raw step ids the post-passes chain into the
    /// equally raw-keyed geometry payload maps (no L2 arena id to record).
    pub(crate) srr_equiv_map: HashMap<u64, u64>,
    /// `plain SHAPE_REPRESENTATION #N → items[0] axis Placement3dId`. Captured
    /// by the plain `SHAPE_REPRESENTATION` handler so SDR conversion can attach
    /// the plain SR's reference frame to `Product.outer_sr_frame` when the
    /// indirection chain is taken.
    pub(crate) plain_sr_frame_map: HashMap<u64, Placement3dId>,
    /// `COMPOSITE_CURVE_SEGMENT #N → (transition, same_sense, parent_curve
    /// step id)`. Populated by the `COMPOSITE_CURVE_SEGMENT` handler so
    /// `COMPOSITE_CURVE` conversion can resolve segments by entity ref.
    pub(crate) composite_segment_map: HashMap<u64, (TransitionCode, bool, u64)>,
    /// `GEOMETRIC_(CURVE_)SET #N → (curves, loose points)`. Populated by the
    /// `GEOMETRIC_(CURVE_)SET` handler, just before the GBWSR / GBSSR converters
    /// consume it.
    pub(crate) curve_set_map: HashMap<u64, (Vec<CurveId>, Vec<PointId>)>,
    /// `GBWSR / GBSSR #N → wireframe content payload`. Same role as
    /// `absr_solid_map` / `mssr_shells_map` but for wireframe products.
    pub(crate) wireframe_data_map: HashMap<u64, WireframeContent>,
    /// `GBWSR / GBSSR #N → axis Placement3dId`. Same role as
    /// `absr_ref_frame_map` for wireframe products. Optional because the
    /// wrapper's `items` list may omit an axis (CATIA's GBSSR commonly
    /// does).
    pub(crate) wireframe_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `PRODUCT_DEFINITION_SHAPE #N` → [`NauoPdsInfo`] when the `pdef_shape`
    /// points at a `NAUO` (instance-tagged). Populated by the
    /// `PRODUCT_DEFINITION_SHAPE` handler's NAUO branch and consumed during
    /// NAUO instance wiring. (The product-targeted counterpart is the typed
    /// [`product_of_pds`](Self::product_of_pds) probe.)
    pub(crate) nauo_pds_info: HashMap<u64, NauoPdsInfo>,

    /// `PROPERTY_DEFINITION`s whose `definition` targets a NAUO-owned PDS,
    /// deferred from the PD handler (the NAUO-PDS is not in the arena during
    /// dispatch) and replayed by `materialize_nauo_owned_pds`. Each entry:
    /// `(pd #N, name, description, pds_target #N)`.
    pub(crate) deferred_nauo_pds_pd: Vec<(u64, String, Option<String>, u64)>,
    /// PD `#N`s recorded in `deferred_nauo_pds_pd`, so the PDR handler knows to
    /// stash its descriptive `Property` (rather than drop it) for replay.
    pub(crate) nauo_pds_pd_refs: HashSet<u64>,
    /// Descriptive `Property` payloads (REPRESENTATION items already resolved)
    /// for deferred NAUO-PDS PDs, stashed by the PDR handler and pushed by
    /// `materialize_nauo_owned_pds` once the PD arena entry exists. Each entry:
    /// `(pd #N, representation_name, items, context)`.
    pub(crate) deferred_nauo_property: Vec<(
        u64,
        String,
        Vec<crate::ir::property::PropertyItem>,
        crate::ir::shape_rep::RepresentationContextRef,
    )>,
    /// Raw `(definition #N, used_representation #N)` of `SHAPE_DEFINITION_REPRESENTATION`s
    /// whose `definition` is not a product PDS (stashed by the SDR handler,
    /// resolved to a `ShapeDefinitionRepresentationLink` after all entities once
    /// the `PROPERTY_DEFINITION`s they point at have been read).
    pub(crate) sdr_link_refs: Vec<(u64, u64)>,
    /// `PROPERTY_DEFINITION_REPRESENTATION` `(definition #N, used_representation #N)`
    /// pairs whose `used_representation` is a modelled representation (e.g.
    /// `SHAPE_REPRESENTATION`). Resolved to `PropertyDefinitionRepresentationLink`
    /// by `resolve_pdr_links` after all entities are read, mirroring `sdr_link_refs`.
    pub(crate) pdr_link_refs: Vec<(u64, u64)>,
    pub(crate) transform_map: HashMap<u64, Transform3d>,
    pub(crate) nauo_transform_map: HashMap<u64, Transform3d>,
    /// Assembly placement RR-complex data stashed by the CDSR handler, keyed by
    /// NAUO #N. Materialised into the `representation_relationships` arena in
    /// canonical (product, instance-index) order by `resolve_nauo_instances` so
    /// the resulting `RepresentationRelationshipId` is round-trip stable.
    pub(crate) nauo_assembly_rr: HashMap<u64, AssemblyRrData>,

    /// `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM`s awaiting `style_context`
    /// resolution (post-pass, after `rrcomplex_to_rrid` is populated).
    pub(crate) pending_cdorsi: Vec<PendingCdorsiStyleContext>,
    /// SDRs whose product-geometry classification is deferred to
    /// [`Self::resolve_sdr_product_geometry`] (dispatch-order independent).
    pub(crate) pending_sdr_geometry: Vec<PendingSdrGeometry>,
    /// NAUO instances awaiting their transform. The transform is produced by
    /// the CDSR handler, but the reference graph runs NAUO -> PDS -> CDSR, so
    /// under topological dispatch the NAUO is read before its CDSR. Stash the
    /// resolved parent/child here and attach the transform in a post-pass,
    /// once every CDSR has populated `nauo_transform_map`.
    pub(crate) pending_nauo_instances: Vec<PendingNauoInstance>,

    /// Lazily-built `VisualizationPool` — the MDGPR converter pushes
    /// `Mdgpr` records here. `None` if no visualization entities were seen.
    pub(crate) visualization: Option<VisualizationPool>,

    /// `FILL_AREA_STYLE_COLOUR #N → FillAreaStyleColour`. A value-cache (not a
    /// founded-item map) populated by the FASC handler and read by
    /// `lower_fill_area_style` to inline the colours.
    pub(crate) viz_fasc_map: HashMap<u64, FillAreaStyleColour>,
    // `viz_fas_id_map` removed: `fill_area_style` migrated to the 2-layer path,
    // so `surface_style_fill_area` resolves its `fill_area` via the typed
    // `EarlyFillAreaStyleId` cache bucket. See `crate::early`.
    // `viz_ssfa_id_map` removed: `surface_style_fill_area` / `surface_side_style`
    // migrated to the 2-layer (`EarlyModel`) path, so the `FillArea` member is
    // disambiguated by the typed `EarlySurfaceStyleFillAreaId` cache bucket
    // instead of a bespoke `FoundedItemId`-valued named map. See `crate::early`.
    /// `SURFACE_STYLE_TRANSPARENT #N → transparency value`.
    pub(crate) viz_transparent_map: HashMap<u64, f64>,
    // `viz_sss_id_map` / `viz_ssu_id_map` / `viz_point_style_id_map` /
    // `viz_view_volume_id_map` removed: `surface_side_style` / `surface_style_usage`
    // / `point_style` / `view_volume` migrated to the 2-layer path, so their
    // consumers (`surface_style_usage`, `presentation_style_assignment`,
    // camera-model handlers) resolve via the typed `Early*Id` cache buckets. See
    // `crate::early`.
    // `viz_ssb_id_map` removed: it was write-only — `surface_style_boundary`
    // round-trips through the founded-item arena and nothing resolves it via a
    // named map, so the field served no purpose.
    /// Lazily-built property pool — populated by the PDR converter.
    /// `None` if the source had no `PROPERTY_DEFINITION_REPRESENTATION`.
    pub(crate) properties: Option<crate::ir::property::PropertyPool>,
    /// `DESCRIPTIVE_REPRESENTATION_ITEM #N → DescriptiveItem`.
    /// Temp; discarded after the property handlers run. PDR handler consults
    /// both this map and `repr_item_id_map` when resolving `REPRESENTATION.items` refs.
    pub(crate) descriptive_item_map: HashMap<u64, crate::ir::shape_rep::DescriptiveItem>,
    /// `SHAPE_DIMENSION_REPRESENTATION` repr id → its raw `items` step refs
    /// (phase measure-arena-1). SDR is read before the complex
    /// `MEASURE_REPRESENTATION_ITEM` arena push, so item resolution is deferred:
    /// `resolve_deferred_sdr_items` re-resolves these once the arena is
    /// populated. Temp.
    pub(crate) sdr_raw_items: HashMap<crate::ir::id::RepresentationId, Vec<u64>>,
    /// `PROPERTY_DEFINITION #N → (name, description, target ProductId)`.
    /// PDs whose target ref does not resolve to a Product
    /// (e.g. `SHAPE_ASPECT`) are silently dropped — no map entry.
    pub(crate) property_def_map: HashMap<u64, (String, Option<String>)>,

    /// `pmi` pool — populated by the PMI handlers. `None` until the
    /// first PMI entity is seen.
    pub(crate) pmi: Option<crate::ir::pmi::PmiPool>,

    /// `SHAPE_ASPECT` arena — the `SHAPE_ASPECT` converter pushes
    /// here. Empty when no PMI entities were seen.
    pub(crate) shape_aspects: crate::ir::Arena<crate::ir::ShapeAspect>,
    /// `SHAPE_ASPECT` subtype arenas. No STEP-id map (no consumer
    /// yet; the entities round-trip as standalone arena entries).
    pub(crate) composite_group_shape_aspects:
        crate::ir::Arena<crate::ir::CompositeGroupShapeAspect>,
    pub(crate) centre_of_symmetries: crate::ir::Arena<crate::ir::CentreOfSymmetry>,
    pub(crate) all_around_shape_aspects: crate::ir::Arena<crate::ir::AllAroundShapeAspect>,
    pub(crate) default_model_geometric_views:
        crate::ir::Arena<crate::ir::DefaultModelGeometricView>,
    /// `DATUM_SYSTEM` arena + `#N → DatumSystemId` map (phase datum-system).
    /// Populated by the `DATUM_SYSTEM` handler; the map lets `resolve_shape_aspect_ref`
    /// resolve a `shape_aspect` ref onto a datum system.
    pub(crate) datum_systems: crate::ir::Arena<crate::ir::DatumSystem>,

    pub(crate) tolerance_zones: crate::ir::Arena<crate::ir::ToleranceZone>,
    /// `DATUM_TARGET` arena + step entity id → `DatumTargetId` map.
    /// Phase datum-target.
    pub(crate) datum_targets: crate::ir::Arena<crate::ir::shape_rep::DatumTarget>,
    /// `PLACED_DATUM_TARGET_FEATURE` arena + id map.
    pub(crate) placed_datum_target_features:
        crate::ir::Arena<crate::ir::shape_rep::PlacedDatumTargetFeature>,
    /// `SHAPE_ASPECT_RELATIONSHIP` arena (phase shape-aspect-ref) — orphan.
    pub(crate) shape_aspect_relationships:
        crate::ir::Arena<crate::ir::shape_rep::ShapeAspectRelationship>,

    /// Unified `REPRESENTATION` arena (representation-refactor expand phase).
    /// 6 representation handlers dual-write here alongside the legacy maps.
    pub(crate) representations: crate::ir::Arena<crate::ir::shape_rep::Representation>,
    /// `representation_item` arena (phase repr-item-arena-1). QRI / VRI
    /// handlers push entries; consumers ref via
    /// `RepresentationItemRef::RepresentationItem(_)`.
    pub(crate) representation_items:
        crate::ir::Arena<crate::ir::representation_item::RepresentationItem>,
    /// `characterized_object` arena (phase characterized-object-ciwr).
    pub(crate) characterized_objects: crate::ir::Arena<crate::ir::shape_rep::CharacterizedObject>,

    /// `REPRESENTATION_MAP` arena + `#N → RepresentationMapId` (phase
    /// mapped-item). The map lets the `MAPPED_ITEM` handler resolve its
    /// `mapping_source` ref.
    pub(crate) representation_maps: crate::ir::Arena<crate::ir::shape_rep::RepresentationMap>,

    /// `MAPPED_ITEM` arena (phase mapped-item) — orphan round-trip.
    pub(crate) mapped_items: crate::ir::Arena<crate::ir::shape_rep::MappedItem>,

    /// `INTEGER`/`REAL_REPRESENTATION_ITEM` value-item arena (phase
    /// numeric-representation-item) — orphan round-trip.
    pub(crate) numeric_representation_items:
        crate::ir::Arena<crate::ir::shape_rep::NumericRepresentationItem>,
    /// `tessellated_item` arena (`COORDINATES_LIST`) + `#N → TessellatedItemId`
    /// (phase tessellation). The map lets the `COMPLEX_TRIANGULATED_FACE`
    /// handler resolve its `coordinates` ref.
    pub(crate) tessellated_items: crate::ir::Arena<crate::ir::tessellation::TessellatedItem>,

    /// `VALUE_FORMAT_TYPE_QUALIFIER` step entity id →
    /// `ValueFormatTypeQualifierId` (phase measure-qualification). Same
    /// `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` step entity id →
    /// `DimensionalCharacteristicRepresentationId` (phase sdr-dcr).
    /// Populated by `DimensionalCharacteristicRepresentationHandler`;
    /// `COMPLEX_TRIANGULATED_FACE` arena (phase tessellation).
    pub(crate) tessellated_faces:
        crate::ir::Arena<crate::ir::tessellation::ComplexTriangulatedFace>,
    /// `COMPLEX_TRIANGULATED_SURFACE_SET` arena (phase tessellation-2).
    pub(crate) tessellated_surface_sets:
        crate::ir::Arena<crate::ir::tessellation::ComplexTriangulatedSurfaceSet>,

    /// `representation_relationship` enum arena (phase cgrr).
    pub(crate) representation_relationships:
        crate::ir::Arena<crate::ir::shape_rep::RepresentationRelationship>,
    /// `compound_representation_item` arena (phase cri).
    pub(crate) compound_representation_items:
        crate::ir::Arena<crate::ir::shape_rep::CompoundRepresentationItem>,
    /// `item_identified_representation_usage` arena (phase iiru).
    pub(crate) item_identified_representation_usages:
        crate::ir::Arena<crate::ir::shape_rep::ItemIdentifiedRepresentationUsage>,
    /// P21 edition 3 `REFERENCE` section, materialized into the model arena.
    pub(crate) external_references: crate::ir::Arena<crate::ir::ExternalReference>,
    /// P21 edition 3 `ANCHOR` section.
    pub(crate) anchors: Vec<crate::ir::ExternalAnchor>,

    /// `SYMBOL_TARGET` step entity id → `GeometricRepresentationItemId`.
    /// Populated by the `SymbolTarget` reader so `DEFINED_SYMBOL.target`
    /// can resolve.
    pub(crate) symbol_target_id_map: HashMap<u64, crate::ir::id::GeometricRepresentationItemId>,
    /// `DEFINED_SYMBOL` step entity id → `GeometricRepresentationItemId`. Lets a
    /// styled `LEADER_TERMINATOR` (or any `STYLED_ITEM` consumer) resolve a
    /// `DEFINED_SYMBOL` item through the GRI arena.
    pub(crate) defined_symbol_id_map: HashMap<u64, crate::ir::id::GeometricRepresentationItemId>,

    /// Lazily-built plm pool — populated by the plm reader chain
    /// (`CalendarDate` / `LocalTime` / UTC / `DateAndTime` / `DateTimeRole`
    /// in Phase plm-1a; Person/Approval/Security clusters later).
    pub(crate) plm: Option<crate::ir::PlmPool>,

    /// Per-file tally of non-standard required fields the reader normalized
    /// to a standard default. Key = (type-qualified field description,
    /// normalized-to description), value = count. Flushed into `warnings` as
    /// one [`ConvertError::NonStandardInput`] per key at the end of `convert`.
    pub(crate) nonstandard_normalizations: BTreeMap<(String, &'static str), usize>,

    /// Per-file tally of canonicalizations (valid legacy/variant form -> modern
    /// canonical form), keyed by the source STEP type. Flushed into
    /// `ConvertResult.canonicalizations` at the end of `convert`. Separate from
    /// `nonstandard_normalizations` (this is not a defect, the input was valid).
    pub(crate) canonicalizations: BTreeMap<&'static str, usize>,

    /// Part-sets already reported via `ConvertError::UnhandledComplex` this
    /// file, so a shape recurring across many instances warns once.
    pub(crate) unhandled_complex_seen: std::collections::BTreeSet<String>,

    /// Largest real file-id in the DATA section (computed once at `convert`
    /// entry). Base for [`Self::alloc_synthetic_entity_id`].
    pub(crate) max_file_id: u64,
    /// Monotonic counter for reader-injected placeholder entity ids.
    pub(crate) synthetic_id_counter: u64,

    pub(crate) warnings: Vec<ConvertError>,
}

/// Reader seam for `#[derive(StepSelect)]`-generated `resolve_select`: probe
/// the type-keyed id cache for a member arena id.
impl crate::ir::select::IdResolver for ReaderContext {
    fn resolve_arena_id<K: crate::ir::arena::ArenaId>(&self, file_id: u64) -> Option<K> {
        self.id_cache.get::<K>(file_id)
    }
}

impl ReaderContext {
    /// Record a non-standard input normalization, aggregated per file so a
    /// 980k-instance defect surfaces as a single summary warning. `case` is the
    /// typed marker (`rg "NsCase::"` finds every site); it does not affect the
    /// aggregation key, so output stays byte-identical. The *only* aggregating
    /// path — see [`NsCase`](crate::reader::NsCase).
    pub(crate) fn ns_record(&mut self, _case: NsCase, field: String, normalized_to: &'static str) {
        *self
            .nonstandard_normalizations
            .entry((field, normalized_to))
            .or_default() += 1;
    }

    /// Record a canonicalization: a **valid** legacy/variant form (`from_type`,
    /// the source STEP entity/part name) normalized to its modern canonical form.
    /// `case` is the typed marker (`rg "CanonCase::"` finds every site); the
    /// aggregation key is `from_type`. Aggregated per file, flushed into
    /// `ConvertResult.canonicalizations`. The canonical counterpart of
    /// [`ns_record`](Self::ns_record) — distinct because the input was valid.
    pub(crate) fn canon_record(&mut self, _case: CanonCase, from_type: &'static str) {
        *self.canonicalizations.entry(from_type).or_default() += 1;
    }

    /// Allocate a collision-free synthetic file-id for a reader-injected
    /// placeholder entity (non-standard normalization). The result exceeds every
    /// real file-id (`max_file_id` base) and stays well below the dangling
    /// sentinel (`u64::MAX`), so it never aliases a real, dangling, or other
    /// synthetic id.
    pub(crate) fn alloc_synthetic_entity_id(&mut self) -> u64 {
        self.synthetic_id_counter += 1;
        self.max_file_id + self.synthetic_id_counter
    }

    /// Record a non-standard input normalization immediately (one warning with
    /// the given `count`, not aggregated). `case` is the typed marker. The only
    /// site (besides this module's per-file flush) that constructs a
    /// `NonStandardInput` warning — see [`NsCase`](crate::reader::NsCase).
    pub(crate) fn ns_push(
        &mut self,
        _case: NsCase,
        field: String,
        count: usize,
        normalized_to: String,
    ) {
        self.warnings.push(ConvertError::NonStandardInput {
            field,
            count,
            normalized_to,
        });
    }

    /// `true` when `id` was registered into any geometry arena (so it survives
    /// round-trip). Used by the `NsCase::Pcurve3dInPspace` accounting to record
    /// only the genuinely-dropped subtree members, not survivors like
    /// `CARTESIAN_POINT` / `DIRECTION` / `VECTOR` (which self-discriminate and
    /// land in their 2D/3D arenas even inside a pcurve subtree).
    fn is_geometry_registered(&self, id: u64) -> bool {
        self.id_cache.contains::<crate::ir::id::PointId>(id)
            || self.id_cache.contains::<crate::ir::id::DirectionId>(id)
            || self.vector_map.contains_key(&id)
            || self.id_cache.contains::<crate::ir::id::SurfaceId>(id)
            || self.id_cache.contains::<crate::ir::id::CurveId>(id)
            || self.placement_map.contains_key(&id)
            || self.id_cache.contains::<crate::ir::id::Point2dId>(id)
            || self.id_cache.contains::<crate::ir::id::Direction2dId>(id)
            || self.vector_2d_map.contains_key(&id)
            || self.id_cache.contains::<crate::ir::id::Curve2dId>(id)
            || self.id_cache.contains::<crate::ir::id::Placement2dId>(id)
    }

    /// Classify a dropped `PCURVE` (the `resolve_pcurve` returned `None` path).
    /// Returns `true` and records `NsCase::Pcurve3dInPspace` normalizations for
    /// the dropped `DEFINITIONAL_REPRESENTATION` subtree when the drop is an
    /// EXPRESS `pcurve.wr3` violation (a 3D curve sitting in a 2D parameter
    /// space); returns `false` for any other drop cause (missing
    /// `basis_surface`, malformed structure) so the caller emits its defect
    /// warning instead.
    pub(crate) fn record_pcurve_wr3_drop(
        &mut self,
        pcurve_ref: u64,
        early: crate::early::EarlyGraph<'_>,
    ) -> bool {
        // Reader-internal raw walk (generic ref-subtree collection the facade
        // can't express) — unwrap the facade here, inside `reader/`.
        let graph = early.raw();
        // PCURVE(name, basis_surface, reference_to_curve).
        let Some(RawEntity::Simple { attributes, .. }) = graph.get(pcurve_ref) else {
            return false;
        };
        let Some(Attribute::EntityRef(basis_surface_ref)) = attributes.get(1) else {
            return false;
        };
        // wr3 signature requires the basis_surface to resolve; a missing
        // surface is a different defect (keep the warning).
        if !self
            .id_cache
            .contains::<crate::ir::id::SurfaceId>(*basis_surface_ref)
        {
            return false;
        }
        let Some(Attribute::EntityRef(def_repr_ref)) = attributes.get(2) else {
            return false;
        };
        let Some(RawEntity::Simple {
            name: def_name,
            attributes: def_attrs,
            ..
        }) = graph.get(*def_repr_ref)
        else {
            return false;
        };
        if def_name != "DEFINITIONAL_REPRESENTATION" {
            return false;
        }
        let Some(Attribute::List(items)) = def_attrs.get(1) else {
            return false;
        };
        let Some(Attribute::EntityRef(first_item_ref)) = items.first() else {
            return false;
        };
        // The wr3 violation: the parameter-space curve is NOT 2D — it never
        // landed in `curve_2d_map`, yet it is a curve entity in the graph.
        if self
            .id_cache
            .contains::<crate::ir::id::Curve2dId>(*first_item_ref)
        {
            return false; // genuinely 2D; not a wr3 violation
        }
        let is_curve = matches!(
            graph.get(*first_item_ref),
            Some(RawEntity::Simple { name, .. }) if is_curve_type(name)
        );
        if !is_curve {
            return false;
        }

        // Walk the DEFINITIONAL_REPRESENTATION subtree and record one
        // normalization per genuinely-dropped (unregistered) member, deduped
        // across pcurves that share a subtree id.
        let mut subtree = HashSet::new();
        collect_refs_transitive(*def_repr_ref, graph, &mut subtree);
        subtree.insert(pcurve_ref);
        for id in subtree {
            if self.dropped_pcurve_recorded.contains(&id) || self.is_geometry_registered(id) {
                continue;
            }
            let Some(RawEntity::Simple { name, .. }) = graph.get(id) else {
                continue;
            };
            self.dropped_pcurve_recorded.insert(id);
            self.ns_record(
                NsCase::Pcurve3dInPspace,
                name.clone(),
                "dropped (3D curve in 2D pcurve parameter-space — EXPRESS pcurve.wr3)",
            );
        }
        true
    }

    /// Convert an entire [`EntityGraph`] into a [`StepModel`].
    ///
    /// Each instance is converted exactly once in reference-dependency
    /// (topological) order: an entity is processed only after every entity it
    /// references. Unrecognised entities are silently skipped — only entities
    /// that the reader *attempts* to convert but fails produce warnings.
    #[must_use]
    pub fn convert(graph: &EntityGraph) -> ConvertResult {
        let mut ctx = Self {
            pcurve_subtree_ids: collect_pcurve_subtree_ids(graph),
            max_file_id: graph.entities.last_key_value().map_or(0, |(&k, _)| k),
            ..Self::default()
        };
        // Materialize P21 edition 3 external references before dispatch so
        // entities (e.g. CIRCULAR_AREA.centre) can resolve an external centre.
        for (&src_id, anchor) in &graph.external_references {
            let id = ctx.external_references.push(crate::ir::ExternalReference {
                anchor: anchor.clone(),
            });
            ctx.id_cache.insert(src_id, id);
        }
        for (name, src_id) in &graph.anchors {
            if let Some(target) = ctx.id_cache.get::<crate::ir::id::ExternalRefId>(*src_id) {
                ctx.anchors.push(crate::ir::ExternalAnchor {
                    name: name.clone(),
                    target,
                });
            }
        }
        ctx.run_topo(graph);
        ctx.resolve_deferred_gisu_used_representation();
        ctx.resolve_product_contexts();
        ctx.resolve_sdr_product_geometry();
        ctx.ensure_product_ref_frames();
        ctx.resolve_nauo_instances();
        ctx.materialize_nauo_owned_pds();
        ctx.resolve_cdorsi_style_contexts();
        ctx.finalize_assembly();
        ctx.resolve_sdr_links();
        ctx.resolve_pdr_links();
        let header = header::extract_file_header(&graph.header, &mut ctx);
        for ((field, normalized_to), count) in std::mem::take(&mut ctx.nonstandard_normalizations) {
            ctx.warnings.push(ConvertError::NonStandardInput {
                field,
                count,
                normalized_to: normalized_to.to_string(),
            });
        }
        ConvertResult {
            model: StepModel {
                geometry: ctx.geometry,
                topology: ctx.topology,
                shape_rep: crate::ir::ShapeRepPool {
                    unit_contexts: ctx.units,
                    unitless_contexts: ctx.unitless_contexts,
                    geometric_item_specific_usages: ctx.geometric_item_specific_usages,
                    shape_aspects: ctx.shape_aspects,
                    composite_group_shape_aspects: ctx.composite_group_shape_aspects,
                    centre_of_symmetries: ctx.centre_of_symmetries,
                    all_around_shape_aspects: ctx.all_around_shape_aspects,
                    default_model_geometric_views: ctx.default_model_geometric_views,
                    datum_systems: ctx.datum_systems,
                    tolerance_zones: ctx.tolerance_zones,
                    datum_targets: ctx.datum_targets,
                    placed_datum_target_features: ctx.placed_datum_target_features,
                    shape_aspect_relationships: ctx.shape_aspect_relationships,
                    representations: ctx.representations,
                    representation_items: ctx.representation_items,
                    characterized_objects: ctx.characterized_objects,
                    representation_maps: ctx.representation_maps,
                    mapped_items: ctx.mapped_items,
                    numeric_representation_items: ctx.numeric_representation_items,
                    tessellated_items: ctx.tessellated_items,
                    tessellated_faces: ctx.tessellated_faces,
                    tessellated_surface_sets: ctx.tessellated_surface_sets,
                    representation_relationships: ctx.representation_relationships,
                    compound_representation_items: ctx.compound_representation_items,
                    item_identified_representation_usages: ctx
                        .item_identified_representation_usages,
                },
                assembly: ctx.assembly,
                visualization: ctx.visualization,
                properties: ctx.properties,
                pmi: ctx.pmi,
                plm: ctx.plm,
                units_pool: build_units_pool(
                    ctx.named_units_arena,
                    ctx.mwu_arena,
                    ctx.due_arena,
                    ctx.derived_unit_arena,
                    ctx.dimensional_exponents,
                ),
                metadata: crate::ir::FileMetadata {
                    schema: graph.schema.clone(),
                    header,
                    external_references: ctx.external_references,
                    anchors: ctx.anchors,
                },
            },
            warnings: ctx.warnings,
            parse_warnings: graph.warnings.clone(),
            canonicalizations: ctx
                .canonicalizations
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Resolve `product_pc_step_refs` / `product_pdc_step_refs` (raw STEP
    /// entity ids captured by the `PRODUCT` / `PRODUCT_DEFINITION` handlers) into
    /// typed `ProductContextId` / `ProductDefinitionContextId` and write them
    /// back onto each `Product`. Run after the assembly-context handlers
    /// populate the id maps.
    fn resolve_product_contexts(&mut self) {
        for (pid, pc_step_id) in &self.product_pc_step_refs {
            if let Some(pcid) = self
                .id_cache
                .get::<crate::ir::id::ProductContextId>(*pc_step_id)
            {
                self.assembly_products[*pid].product_context = Some(pcid);
            }
        }
        for (pid, pdc_step_id) in &self.product_pdc_step_refs {
            if let Some(pdcid) = self
                .id_cache
                .get::<crate::ir::id::ProductDefinitionContextId>(*pdc_step_id)
            {
                self.assembly_products[*pid].pdef_context = Some(pdcid);
                // Mirror onto the canonical PD arena entry (the context id map
                // only fills after the PD pass, so resolve it here).
                if let Some(pd_id) = self.assembly_products[*pid].pdef {
                    self.product_definitions[pd_id].context = Some(pdcid);
                }
            }
        }
    }

    /// Resolve each product's `shape_ref_frame` for the degenerate case where
    /// the file contains no `AXIS2_PLACEMENT_3D` at all: synthesize a single
    /// identity placement and point every product at it. The PRODUCT handler
    /// leaves `shape_ref_frame = Placement3dId(0)` as a tentative default;
    /// SDR conversion overwrites it with the real frame when a shape
    /// representation exists. Run after the full dispatch so the
    /// `placements.is_empty()` test is dispatch-order independent. When any
    /// placement exists no product was left dangling — an SDR frame derives
    /// from a real placement, and the `Placement3dId(0)` default is valid — so
    /// this is a no-op.
    fn ensure_product_ref_frames(&mut self) {
        if self.assembly_products.is_empty() || !self.geometry.placements.is_empty() {
            return;
        }
        let identity = self.geometry.identity_placement();
        let pids: Vec<_> = self.assembly_products.iter_ids().collect();
        for pid in pids {
            self.assembly_products[pid].shape_ref_frame = identity;
        }
    }

    /// Attach the deferred NAUO transforms and push each instance onto its
    /// parent product. Runs after the full dispatch (every CDSR has populated
    /// `nauo_transform_map`) and before `finalize_assembly` consumes the
    /// instances. A NAUO with no transform warns and drops, as before.
    ///
    /// Second pass materialises each placement's RR-complex into the
    /// `representation_relationships` arena in **canonical (product-arena ×
    /// instance-index) order** — not the flat NAUO dispatch order — so the
    /// resulting `RepresentationRelationshipId`s (referenced by
    /// `Instance.transform_rr` and `style_context`, both deep-diffed) are
    /// round-trip stable. See [`crate::ir::shape_rep::RrwtData`].
    fn resolve_nauo_instances(&mut self) {
        use crate::ir::id::ProductId;
        use crate::ir::shape_rep::{RepresentationRelationship, RrwtData};
        // Pass 1 — build instances (flat NAUO order). Remember which NAUO each
        // (parent, index) slot came from so pass 2 can look up its RR payload.
        let mut slot_nauo: HashMap<(ProductId, usize), u64> = HashMap::new();
        for pending in std::mem::take(&mut self.pending_nauo_instances) {
            let Some(&transform) = self.nauo_transform_map.get(&pending.nauo_id) else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id: pending.nauo_id,
                    detail: String::from("NEXT_ASSEMBLY_USAGE_OCCURRENCE with no transform found"),
                });
                continue;
            };
            let parent = pending.parent;
            let idx = self.assembly_products[parent].instances.len();
            slot_nauo.insert((parent, idx), pending.nauo_id);
            // Push the canonical `assembly_component_usage` arena entry here —
            // co-located with the Instance build — so a transform-less NAUO
            // (the `else continue` above) leaves neither an arena entry nor an
            // Instance, keeping the two strictly 1:1 (no orphan NAUO on write).
            let acu_id = self.assembly_component_usages.push(
                crate::ir::assembly::NextAssemblyUsageOccurrence {
                    id: pending.occurrence_id,
                    name: pending.occurrence_name,
                    description: pending.description,
                    relating: pending.relating_pd,
                    related: pending.related_pd,
                    reference_designator: pending.reference_designator,
                },
            );
            // Record the NAUO → ACU id so `materialize_nauo_owned_pds` can resolve
            // a NAUO-targeted PDS's `definition` to this canonical arena entry.
            self.id_cache.insert(pending.nauo_id, acu_id);
            // The Instance is a denormalized view of the arena entry. `child` is
            // the resolved child ProductId (the arena's `related` is the
            // PRODUCT_DEFINITION ref); occurrence id/name mirror the arena.
            let (occurrence_id, occurrence_name) = {
                let acu = &self.assembly_component_usages[acu_id];
                (acu.id.clone(), acu.name.clone())
            };
            let placement_representation = self
                .nauo_placement_sr
                .get(&pending.nauo_id)
                .cloned()
                .unwrap_or_default();
            self.assembly_products[parent]
                .instances
                .push(crate::ir::assembly::Instance {
                    child: pending.child,
                    transform,
                    occurrence_id,
                    occurrence_name,
                    transform_rr: None,
                    acu: Some(acu_id),
                    placement_representation,
                });
        }
        // Pass 2 — canonical-order RR materialisation.
        let pids: Vec<ProductId> = self.assembly_products.iter_ids().collect();
        for pid in pids {
            let count = self.assembly_products[pid].instances.len();
            for idx in 0..count {
                let Some(&nauo_id) = slot_nauo.get(&(pid, idx)) else {
                    continue;
                };
                let Some(data) = self.nauo_assembly_rr.remove(&nauo_id) else {
                    continue;
                };
                let transform = self.assembly_products[pid].instances[idx].transform;
                let rrid = self.representation_relationships.push(
                    RepresentationRelationship::RepresentationRelationshipWithTransformation(
                        RrwtData {
                            name: data.name,
                            description: data.description,
                            rep_1: data.rep_1,
                            rep_2: data.rep_2,
                            transform,
                        },
                    ),
                );
                self.assembly_products[pid].instances[idx].transform_rr = Some(rrid);
                self.id_cache.insert(data.rr_complex_entity, rrid);
            }
        }
    }

    /// Materialise NAUO-owned `PRODUCT_DEFINITION_SHAPE`s into the
    /// `property_definitions` arena and replay the `PROPERTY_DEFINITION`s (and
    /// descriptive `Property`s) that target them. Deferred from dispatch because
    /// a NAUO-PDS's `definition` resolves to an `AssemblyComponentUsageId`, which
    /// only exists after `resolve_nauo_instances`. Arena pushes are ordered by
    /// source `#N` so the round-trip-diffed arenas reproduce identically on
    /// re-read. A NAUO-PDS whose NAUO has no instance (transform-less, dropped)
    /// has no ACU id and is skipped — keeping reader/writer symmetric (the
    /// assembly chain emits a body only for instances).
    ///
    /// Scoped to assemblies that actually attach a property to a NAUO-PDS (the
    /// `deferred_nauo_pds_pd` early-out): when nothing targets it, the NAUO-PDS
    /// stays on the assembly chain's synthesis path, leaving the common
    /// assembly's output untouched.
    fn materialize_nauo_owned_pds(&mut self) {
        use crate::ir::property::{
            CharacterizedDefinition, ProductDefinitionShape, Property, PropertyDefinition,
            PropertyDefinitionData, PropertyPool,
        };

        if self.deferred_nauo_pds_pd.is_empty() {
            return;
        }

        // 1. NAUO-targeted PDS → arena entry (source-#N order).
        let mut nauo_pds: Vec<(u64, u64, String, String)> = self
            .nauo_pds_info
            .iter()
            .map(|(&pds, info)| (pds, info.nauo, info.name.clone(), info.description.clone()))
            .collect();
        nauo_pds.sort_unstable_by_key(|&(pds, ..)| pds);
        for (pds_step, nauo_step, name, description) in nauo_pds {
            let Some(acu_id) = self
                .id_cache
                .get::<crate::ir::id::AssemblyComponentUsageId>(nauo_step)
            else {
                continue; // NAUO without an instance — no ACU, skip (orphan-safe).
            };
            let pd_id = self
                .properties
                .get_or_insert_with(PropertyPool::default)
                .property_definitions
                .push(PropertyDefinition::ProductDefinitionShape(
                    ProductDefinitionShape {
                        inherited: PropertyDefinitionData {
                            name,
                            description,
                            definition: CharacterizedDefinition::ProductDefinitionRelationship(
                                acu_id,
                            ),
                        },
                    },
                ));
            self.id_cache.insert(pds_step, pd_id);
        }

        // 2. Deferred centroid PDs → arena entry (source-#N order).
        let mut deferred_pd = std::mem::take(&mut self.deferred_nauo_pds_pd);
        deferred_pd.sort_unstable_by_key(|&(pd_step, ..)| pd_step);
        for (pd_step, name, description, pds_target) in deferred_pd {
            let Some(pds_pd_id) = self
                .id_cache
                .get::<crate::ir::id::PropertyDefinitionId>(pds_target)
            else {
                continue; // NAUO-PDS was skipped above (no ACU) — drop this PD too.
            };
            let pd_id = self
                .properties
                .get_or_insert_with(PropertyPool::default)
                .property_definitions
                .push(PropertyDefinition::Itself(PropertyDefinitionData {
                    name: name.clone(),
                    description: description.clone().unwrap_or_default(),
                    definition: CharacterizedDefinition::ProductDefinitionShape(pds_pd_id),
                }));
            self.id_cache.insert(pd_step, pd_id);
            self.property_def_map.insert(pd_step, (name, description));
        }

        // 3. Deferred descriptive Properties → arena entry (source-#N order).
        let mut deferred_prop = std::mem::take(&mut self.deferred_nauo_property);
        deferred_prop.sort_unstable_by_key(|&(pd_step, ..)| pd_step);
        for (pd_step, representation_name, items, context) in deferred_prop {
            let Some(definition) = self
                .id_cache
                .get::<crate::ir::id::PropertyDefinitionId>(pd_step)
            else {
                continue;
            };
            let (name, description) = self
                .property_def_map
                .get(&pd_step)
                .cloned()
                .unwrap_or_default();
            let prop_id = self
                .properties
                .get_or_insert_with(PropertyPool::default)
                .properties
                .push(Property {
                    name,
                    description,
                    definition: crate::ir::property::PropertyDefinitionRef::PropertyDefinition(
                        definition,
                    ),
                    representation_name,
                    context,
                    items,
                });
            self.id_cache.insert(pd_step, prop_id);
        }
    }

    /// Derive the `used_representation` of GISUs whose source value was the
    /// non-standard `$` (CATIA). The schema requires it and its WHERE rule ties
    /// it to the representation that contains `identified_item`; step-io recovers
    /// that container instead of dropping the GISU. Deferred to here (post
    /// `run_topo`) because the container is not referenced by the GISU, so
    /// dispatch order gives no guarantee it was read first. Pushes in source-`#N`
    /// order so the round-trip-diffed arena reproduces identically on re-read
    /// (where the now-explicit ref resolves inline).
    fn resolve_deferred_gisu_used_representation(&mut self) {
        use crate::ir::representation_item::RepresentationItemRef;
        use crate::ir::shape_rep::{GeometricItemSpecificUsage, Representation};

        let mut deferred = std::mem::take(&mut self.deferred_gisu_used_repr);
        if deferred.is_empty() {
            return;
        }

        // Typed reverse indices: an item belongs to exactly one shape
        // representation in well-formed STEP, so first-writer-wins is safe.
        let mut solid_to_rep: HashMap<crate::ir::id::SolidId, crate::ir::RepresentationId> =
            HashMap::new();
        let mut curve_to_rep: HashMap<crate::ir::id::CurveId, crate::ir::RepresentationId> =
            HashMap::new();
        for (rid, repr) in self.representations.iter_with_ids() {
            match repr {
                Representation::AdvancedBrep(r) => {
                    for s in r.solids() {
                        solid_to_rep.entry(s).or_insert(rid);
                    }
                }
                Representation::Wireframe(r) => {
                    for c in &r.content.curves {
                        curve_to_rep.entry(*c).or_insert(rid);
                    }
                }
                _ => {}
            }
        }

        deferred.sort_by_key(|d| d.entity_id);
        let mut recovered = 0usize;
        for d in deferred {
            let used = match d.identified_item {
                RepresentationItemRef::Solid(sid) => solid_to_rep.get(&sid).copied(),
                RepresentationItemRef::Curve(cid) => curve_to_rep.get(&cid).copied(),
                _ => None,
            };
            let Some(used_representation) = used else {
                // No container found — drop with a visibility warning (symmetric
                // on re-read, where the GISU was never emitted).
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id: d.entity_id,
                    detail: String::from(
                        "GEOMETRIC_ITEM_SPECIFIC_USAGE used_representation Unset and no containing \
                         representation found for identified_item — dropping",
                    ),
                });
                continue;
            };
            let id = self
                .geometric_item_specific_usages
                .push(GeometricItemSpecificUsage {
                    name: d.name,
                    description: d.description,
                    definition: d.definition,
                    used_representation,
                    identified_item: d.identified_item,
                });
            self.id_cache.insert(d.entity_id, id);
            recovered += 1;
        }
        if recovered > 0 {
            // NsCase::GisuUnsetUsedRep recovery side (detect/defer is in
            // geometric_item_specific_usage.rs). Surfaced as a normalization,
            // not a defect — the GISU is preserved. See reader::nonstandard.
            self.ns_push(
                NsCase::GisuUnsetUsedRep,
                "GEOMETRIC_ITEM_SPECIFIC_USAGE.used_representation (Unset)".into(),
                recovered,
                "containing representation (derived from identified_item)".into(),
            );
        }
    }

    /// Resolve each deferred `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM`'s
    /// `style_context` SELECT list and patch the already-pushed styled item.
    /// Runs after `resolve_nauo_instances` so assembly RR-complex targets have
    /// a `RepresentationRelationshipId`. A `representation_item` target resolves
    /// to [`StyleContextRef::RepresentationItem`]; an assembly RR-complex to
    /// [`StyleContextRef::RepresentationRelationship`]; anything else warns and
    /// drops (e.g. a `shape` / `shape_aspect` member, or a cascade from a
    /// dropped placement).
    fn resolve_cdorsi_style_contexts(&mut self) {
        use crate::entities::visualization::styled_item::resolve_representation_item_ref;
        use crate::ir::visualization::{StyleContextRef, StyledItem};
        let pendings = std::mem::take(&mut self.pending_cdorsi);
        // Resolve against `self` immutably first, then apply the mutations.
        let mut patches: Vec<(crate::ir::id::StyledItemId, Vec<StyleContextRef>)> =
            Vec::with_capacity(pendings.len());
        let mut warnings: Vec<ConvertError> = Vec::new();
        for pending in &pendings {
            let mut resolved = Vec::with_capacity(pending.context_refs.len());
            for &r in &pending.context_refs {
                if let Some(target) = resolve_representation_item_ref(self, r) {
                    resolved.push(StyleContextRef::RepresentationItem(target));
                } else if let Some(rrid) = self
                    .id_cache
                    .get::<crate::ir::id::RepresentationRelationshipId>(r)
                {
                    resolved.push(StyleContextRef::RepresentationRelationship(rrid));
                } else {
                    warnings.push(ConvertError::UnexpectedEntityForm {
                        entity_id: pending.entity_id,
                        detail: format!(
                            "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM.style_context #{r} target \
                             type unsupported"
                        ),
                    });
                }
            }
            patches.push((pending.styled_item_id, resolved));
        }
        self.warnings.extend(warnings);
        if let Some(pool) = self.visualization.as_mut() {
            for (id, resolved) in patches {
                if let StyledItem::ContextDependent(cd) = &mut pool.styled_items[id] {
                    cd.style_context = resolved;
                }
            }
        }
    }

    /// Wrap the collected products into an `AssemblyTree` if any PRODUCT
    /// entities were seen. `roots` lists every top-level product (a forest
    /// for multi-part files).
    fn finalize_assembly(&mut self) {
        if self.id_cache.is_empty::<crate::ir::ProductId>() {
            return;
        }
        // Collect every ProductId that appears as an Instance.child. The
        // remaining products are root candidates. (The canonical NAUO arena's
        // `related` is now a ProductDefinitionId, so the root scan walks the
        // 1:1 Instance view, which carries the resolved child ProductId.)
        let mut is_child: HashSet<ProductId> = HashSet::new();
        for product in self.assembly_products.iter() {
            for inst in &product.instances {
                is_child.insert(inst.child);
            }
        }
        let roots: Vec<ProductId> = self
            .assembly_products
            .iter_ids()
            .filter(|pid| !is_child.contains(pid))
            .collect();
        // A multi-part file legitimately holds several independent
        // top-level products — a normal forest, not an anomaly. Only a
        // fully cyclic graph (no product free of a parent) is malformed.
        if roots.is_empty() {
            self.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id: 0,
                detail: String::from(
                    "assembly has no root candidate (every product appears as an instance child)",
                ),
            });
        }
        let products = std::mem::take(&mut self.assembly_products);
        let product_contexts = std::mem::take(&mut self.product_contexts);
        let product_definition_contexts = std::mem::take(&mut self.product_definition_contexts);
        let product_definition_context_roles =
            std::mem::take(&mut self.product_definition_context_roles);
        let product_definition_context_associations =
            std::mem::take(&mut self.product_definition_context_associations);
        let product_definition_relationships =
            std::mem::take(&mut self.product_definition_relationships);
        let product_categories = std::mem::take(&mut self.product_categories);
        let product_category_relationships =
            std::mem::take(&mut self.product_category_relationships);
        let product_definition_formations = std::mem::take(&mut self.product_definition_formations);
        let assembly_component_usages = std::mem::take(&mut self.assembly_component_usages);
        let product_definitions = std::mem::take(&mut self.product_definitions);
        self.assembly = Some(AssemblyTree {
            products,
            roots,
            product_contexts,
            product_definition_contexts,
            product_definition_context_roles,
            product_definition_context_associations,
            product_definition_relationships,
            product_categories,
            product_category_relationships,
            product_definition_formations,
            assembly_component_usages,
            product_definitions,
        });
    }

    /// Resolve the raw `SHAPE_DEFINITION_REPRESENTATION` links stashed by the
    /// SDR handler (definition = a `PROPERTY_DEFINITION`). Runs after all entities
    /// are read, so `property_def_step_to_id` and `repr_id_map` are
    /// complete. Unresolved links (PD or representation not modelled) drop —
    /// FAIL-safe.
    fn resolve_sdr_links(&mut self) {
        use crate::ir::property::SdrDefinition;
        if self.sdr_link_refs.is_empty() {
            return;
        }
        let mut links = Vec::new();
        for (def_ref, rep_ref) in std::mem::take(&mut self.sdr_link_refs) {
            let effective = self.srr_equiv_map.get(&rep_ref).copied().unwrap_or(rep_ref);
            // `definition` is a `represented_definition` SELECT: a
            // property_definition, or a SHAPE_ASPECT (C3D / grabcad define a
            // shape aspect's shape directly).
            let definition = if let Some(pd) = self
                .id_cache
                .get::<crate::ir::id::PropertyDefinitionId>(def_ref)
            {
                SdrDefinition::PropertyDefinition(pd)
            } else if let Some(sa) = self.id_cache.get::<crate::ir::ShapeAspectId>(def_ref) {
                SdrDefinition::ShapeAspect(sa)
            } else {
                continue;
            };
            let Some(used_representation) = self
                .id_cache
                .get::<crate::ir::id::RepresentationId>(effective)
            else {
                continue;
            };
            links.push(crate::ir::property::ShapeDefinitionRepresentationLink {
                definition,
                used_representation,
            });
        }
        if links.is_empty() {
            return;
        }
        let pool = self
            .properties
            .get_or_insert_with(crate::ir::property::PropertyPool::default);
        for link in links {
            pool.shape_definition_representations.push(link);
        }
    }

    /// Resolve stashed `PROPERTY_DEFINITION_REPRESENTATION` links whose
    /// `used_representation` is a modelled representation (e.g.
    /// `SHAPE_REPRESENTATION`). Runs after all passes so the `PROPERTY_DEFINITION`
    /// and the representation are both read. Unresolved links drop — FAIL-safe.
    /// Mirrors [`Self::resolve_sdr_links`].
    fn resolve_pdr_links(&mut self) {
        if self.pdr_link_refs.is_empty() {
            return;
        }
        let mut links = Vec::new();
        for (def_ref, rep_ref) in std::mem::take(&mut self.pdr_link_refs) {
            let effective = self.srr_equiv_map.get(&rep_ref).copied().unwrap_or(rep_ref);
            if let (Some(definition), Some(used_representation)) = (
                self.id_cache
                    .get::<crate::ir::id::PropertyDefinitionId>(def_ref),
                self.id_cache
                    .get::<crate::ir::id::RepresentationId>(effective),
            ) {
                links.push(crate::ir::property::PropertyDefinitionRepresentationLink {
                    definition,
                    used_representation,
                });
            }
        }
        if links.is_empty() {
            return;
        }
        let pool = self
            .properties
            .get_or_insert_with(crate::ir::property::PropertyPool::default);
        for link in links {
            pool.property_definition_representations.push(link);
        }
    }

    // ---------------------------------------------------------------------
    // Resolver helpers: look up a STEP entity id in one of the internal
    // maps and return the stored IR id (or a `MissingReference` error).
    // Each converter can collapse four lines of boilerplate into one call.
    // ---------------------------------------------------------------------

    /// Resolve a `context_of_items` STEP id to a
    /// `RepresentationContextRef`. Looks up the unit-bearing
    /// `context_id_map` first, then falls back to the GRC+PRC
    /// `unitless_context_id_map` (phase unitless-context). Returns
    /// `None` when the id matches neither map (e.g. unmodelled context
    /// complex such as GRC+GUNCAC without a GUAC part).
    pub(crate) fn resolve_repr_context(
        &self,
        from: u64,
    ) -> Option<crate::ir::shape_rep::RepresentationContextRef> {
        use crate::ir::shape_rep::RepresentationContextRef;
        if let Some(&id) = self.context_id_map.get(&from) {
            return Some(RepresentationContextRef::Unitful(id));
        }
        if let Some(id) = self.id_cache.get::<crate::ir::id::UnitlessContextId>(from) {
            return Some(RepresentationContextRef::Unitless(id));
        }
        None
    }

    pub(crate) fn resolve_point(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<PointId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::PointId>(from, to, field_name)
    }

    pub(crate) fn resolve_direction(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<DirectionId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::DirectionId>(from, to, field_name)
    }

    pub(crate) fn resolve_curve(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<CurveId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::CurveId>(from, to, field_name)
    }

    pub(crate) fn resolve_surface(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<SurfaceId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::SurfaceId>(from, to, field_name)
    }

    pub(crate) fn resolve_vertex(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<VertexId, ConvertError> {
        resolve_in_map(&self.vertex_map, from, to, field_name)
    }

    pub(crate) fn resolve_edge(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<EdgeId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::EdgeId>(from, to, field_name)
    }

    pub(crate) fn resolve_face_bound(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<WireId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::WireId>(from, to, field_name)
    }

    pub(crate) fn resolve_face(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<FaceId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::FaceId>(from, to, field_name)
    }

    pub(crate) fn resolve_shell(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<ShellId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::ShellId>(from, to, field_name)
    }

    /// One typed probe `PRODUCT_DEFINITION #N → ProductId`. The migrated
    /// pdef's `lower` recorded the correspondence (only when the product
    /// itself resolved — exactly when the former raw 2-hop
    /// `pdef_to_product` → `id_cache` chain succeeded).
    pub(crate) fn product_of_pdef(&self, pdef_ref: u64) -> Option<ProductId> {
        self.id_cache
            .get::<crate::early::model::EarlyProductDefinitionId>(pdef_ref)
            .map(|e| self.early.lookup_lowered(e))
    }

    /// One typed probe `PRODUCT_DEFINITION_FORMATION #N → ProductId`
    /// (formation counterpart of [`product_of_pdef`](Self::product_of_pdef)).
    pub(crate) fn product_of_formation(&self, formation_ref: u64) -> Option<ProductId> {
        self.id_cache
            .get::<crate::early::model::EarlyProductDefinitionFormationId>(formation_ref)
            .map(|e| self.early.lookup_lowered(e))
    }

    /// One typed probe `PRODUCT_DEFINITION_SHAPE #N → ProductId` (PDS
    /// counterpart of [`product_of_pdef`](Self::product_of_pdef) — what the
    /// former `pdef_shape_to_pdef` → `product_of_pdef` chain resolved to).
    pub(crate) fn product_of_pds(&self, pds_ref: u64) -> Option<ProductId> {
        self.id_cache
            .get::<crate::early::model::EarlyProductDefinitionShapeId>(pds_ref)
            .map(|e| self.early.lookup_lowered(e))
    }

    /// Lookup `PRODUCT_DEFINITION #N → ProductId` shared by the NAUO
    /// tree-wiring handlers (error-carrying form of
    /// [`product_of_pdef`](Self::product_of_pdef)).
    pub(crate) fn resolve_product_by_pdef(
        &self,
        from: u64,
        pdef_ref: u64,
        field_name: &'static str,
    ) -> Result<ProductId, ConvertError> {
        self.product_of_pdef(pdef_ref)
            .ok_or(ConvertError::MissingReference {
                from,
                to: pdef_ref,
                field_name,
            })
    }

    pub(crate) fn resolve_placement(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Placement3dId, ConvertError> {
        resolve_in_map(&self.placement_map, from, to, field_name)
    }

    pub(crate) fn resolve_vector(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<(DirectionId, f64), ConvertError> {
        resolve_in_map(&self.vector_map, from, to, field_name)
    }

    pub(crate) fn resolve_axis1(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Placement1dId, ConvertError> {
        self.id_cache
            .resolve::<crate::ir::id::Placement1dId>(from, to, field_name)
    }

    pub(crate) fn resolve_oriented_edge(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<OrientedEdge, ConvertError> {
        resolve_in_map(&self.oriented_edge_map, from, to, field_name)
    }

    pub(crate) fn resolve_edge_loop(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Vec<OrientedEdge>, ConvertError> {
        resolve_in_map_cloned(&self.edge_loop_map, from, to, field_name)
    }
}

// ---------------------------------------------------------------------------
// Free helpers (used by multiple submodules)
// ---------------------------------------------------------------------------

/// Look up `to` in `map` and return the stored value, raising
/// `MissingReference` keyed by `(from, to, field_name)` when absent.
pub(crate) fn resolve_in_map<V: Copy>(
    map: &HashMap<u64, V>,
    from: u64,
    to: u64,
    field_name: &'static str,
) -> Result<V, ConvertError> {
    map.get(&to).copied().ok_or(ConvertError::MissingReference {
        from,
        to,
        field_name,
    })
}

/// `Clone` variant of [`resolve_in_map`] for non-`Copy` map values
/// (e.g. `Vec<OrientedEdge>`).
pub(crate) fn resolve_in_map_cloned<V: Clone>(
    map: &HashMap<u64, V>,
    from: u64,
    to: u64,
    field_name: &'static str,
) -> Result<V, ConvertError> {
    map.get(&to).cloned().ok_or(ConvertError::MissingReference {
        from,
        to,
        field_name,
    })
}

/// Wrap the units arenas into a [`UnitsPool`], returning `None` when all
/// are empty (the common case for fixtures without MWU / DUE / DU / MASS
/// content). Called by [`ReaderContext::convert`] when assembling the
/// final [`StepModel`].
fn build_units_pool(
    named_units: Arena<NamedUnit>,
    measure_with_units: Arena<MeasureWithUnit>,
    derived_unit_elements: Arena<DerivedUnitElement>,
    derived_units: Arena<DerivedUnit>,
    dimensional_exponents: Arena<crate::ir::units::DimensionalExponents>,
) -> Option<UnitsPool> {
    if named_units.is_empty()
        && measure_with_units.is_empty()
        && derived_unit_elements.is_empty()
        && derived_units.is_empty()
        && dimensional_exponents.is_empty()
    {
        None
    } else {
        Some(UnitsPool {
            named_units,
            measure_with_units,
            derived_unit_elements,
            derived_units,
            dimensional_exponents,
        })
    }
}

pub(crate) fn bool_to_orientation(same_sense: bool) -> Orientation {
    if same_sense {
        Orientation::Forward
    } else {
        Orientation::Reversed
    }
}

/// Find a part by name in a complex entity's part list.
pub(crate) fn find_part_attrs<'a>(
    parts: &'a [RawEntityPart],
    name: &str,
) -> Option<&'a [Attribute]> {
    parts
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.attributes.as_slice())
}

/// Find a required part by name. Returns an error if missing.
pub(crate) fn require_part_attrs<'a>(
    parts: &'a [RawEntityPart],
    name: &'static str,
    entity_id: u64,
) -> Result<&'a [Attribute], ConvertError> {
    find_part_attrs(parts, name).ok_or(ConvertError::UnexpectedEntityForm {
        entity_id,
        detail: format!("missing required part '{name}'"),
    })
}

/// Check whether a complex entity contains all required parts (subset test).
/// Used by handler bodies that sniff optional companion parts after an
/// exact-case match has selected them.
pub(crate) fn has_all_parts(parts: &[RawEntityPart], required: &[&str]) -> bool {
    required
        .iter()
        .all(|name| parts.iter().any(|p| p.name == *name))
}

/// True when a complex entity's DISTINCT part-name set EQUALS `case`
/// (order-independent; duplicate part names — which STEP AND-instances do not
/// have — collapse). This is the exact-case dispatch predicate.
pub(crate) fn matches_exact_case(parts: &[RawEntityPart], case: &[&str]) -> bool {
    let actual: std::collections::BTreeSet<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    let expected: std::collections::BTreeSet<&str> = case.iter().copied().collect();
    actual == expected
}

/// True when the entity's part-set exactly matches any of the handler's
/// declared `cases`.
pub(crate) fn matches_any_case(parts: &[RawEntityPart], cases: &[&[&str]]) -> bool {
    cases.iter().any(|c| matches_exact_case(parts, c))
}

/// How a complex (multi-part) instance lines up with the complex handler
/// registry. Complex matching is a *subset* test (`required ⊆ parts`), so an
/// instance can be claimed by a handler that does not account for all of its
/// parts — silently dropping the rest. This classifies that looseness for an
/// audit; it has no effect on [`ReaderContext::convert`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexMatchClass {
    /// Two or more complex handlers match the same instance — dispatch order
    /// decides the winner and the losing handler's parts drop.
    Ambiguous,
    /// Exactly one handler matches, but the instance carries parts beyond that
    /// handler's required set — those extra parts may be silently dropped.
    Loose,
    /// No complex handler matches — the instance is skipped entirely.
    Unhandled,
}

/// One complex instance's audit result. See [`audit_complex_matching`].
#[derive(Debug, Clone)]
pub struct ComplexAuditFinding {
    pub class: ComplexMatchClass,
    /// Sorted, de-duplicated part type names of the instance.
    pub parts: Vec<String>,
    /// Matching handler names — the competitors (`Ambiguous`) or the single
    /// claimant (`Loose`); empty for `Unhandled`.
    pub handlers: Vec<String>,
    /// `Loose` only: parts present on the instance but not in the matched
    /// handler's required set (the silent-drop candidates).
    pub extra_parts: Vec<String>,
}

/// Every `(handler_name, sorted distinct part-set)` pair where a complex
/// handler's declared cases exactly match an instance in `graph`. Run over the
/// corpus and de-duplicated, this verifies the declared `cases` cover the
/// observed shapes. Diagnostic only.
#[must_use]
pub fn complex_handler_matches(graph: &EntityGraph) -> Vec<(String, Vec<String>)> {
    use crate::entities::{ENTITY_HANDLERS, ReadKind};
    let mut out = Vec::new();
    for ent in graph.entities.values() {
        let RawEntity::Complex { parts, .. } = ent else {
            continue;
        };
        let mut part_names: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
        part_names.sort();
        part_names.dedup();
        for e in ENTITY_HANDLERS {
            if let ReadKind::Complex { cases, .. } = &e.kind
                && matches_any_case(parts, cases)
            {
                out.push((e.name.to_string(), part_names.clone()));
            }
        }
    }
    out
}

/// Audit every complex instance in `graph` against the complex handler
/// registry under exact-case matching. Diagnostic — does NOT run during
/// [`ReaderContext::convert`]. `Ambiguous` = two **distinct-name** handlers
/// match (2D/3D sisters share a name and are excluded). `Unhandled` = no
/// handler matches. A single exact match is healthy and omitted; `Loose` is
/// structurally impossible under exact-case (retained for API stability).
#[must_use]
pub fn audit_complex_matching(graph: &EntityGraph) -> Vec<ComplexAuditFinding> {
    use crate::entities::{ENTITY_HANDLERS, ReadKind};
    let mut out = Vec::new();
    for ent in graph.entities.values() {
        let RawEntity::Complex { parts, .. } = ent else {
            continue;
        };
        let mut part_names: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
        part_names.sort();
        part_names.dedup();

        let mut names: Vec<&'static str> = Vec::new();
        for e in ENTITY_HANDLERS {
            if let ReadKind::Complex { cases, .. } = &e.kind
                && matches_any_case(parts, cases)
            {
                names.push(e.name);
            }
        }
        let distinct: std::collections::BTreeSet<&str> = names.iter().copied().collect();

        let finding = if distinct.len() >= 2 {
            ComplexAuditFinding {
                class: ComplexMatchClass::Ambiguous,
                parts: part_names,
                handlers: distinct.iter().map(|n| (*n).to_string()).collect(),
                extra_parts: Vec::new(),
            }
        } else if distinct.len() == 1 {
            continue; // healthy exact match (sisters share the name).
        } else {
            ComplexAuditFinding {
                class: ComplexMatchClass::Unhandled,
                parts: part_names,
                handlers: Vec::new(),
                extra_parts: Vec::new(),
            }
        };
        out.push(finding);
    }
    out
}

/// Collect entity ids that belong to any `DEFINITIONAL_REPRESENTATION`
/// subtree. These represent PCURVE parametric-space geometry (2D points,
/// 2D curves, `AXIS2_PLACEMENT_2D`, `PARAMETRIC_REPRESENTATION_CONTEXT`).
/// 3D handlers skip them so their 2D `CARTESIAN_POINT` / `DIRECTION` / etc.
/// don't collide with the 3D converters; the 2D-curve handlers then walk
/// the same set to populate the 2D arenas.
///
/// Only the exact entity type name `DEFINITIONAL_REPRESENTATION` is treated
/// as a root — other `REPRESENTATION` subtypes (e.g. `SHAPE_REPRESENTATION`,
/// `ADVANCED_BREP_SHAPE_REPRESENTATION`) reference 3D top-level entities
/// and must remain visible.
pub(crate) fn collect_pcurve_subtree_ids(graph: &EntityGraph) -> HashSet<u64> {
    let mut ids = HashSet::new();
    for (&id, entity) in &graph.entities {
        if let RawEntity::Simple { name, .. } = entity
            && name == "DEFINITIONAL_REPRESENTATION"
        {
            collect_refs_transitive(id, graph, &mut ids);
        }
    }
    ids
}

/// Whether a STEP entity type name denotes a geometric curve — used to confirm
/// the `pcurve.wr3` violation (a curve, not e.g. a point, sitting unresolved in
/// the 2D parameter space). Covers the `bounded_curve` / `conic` / spline
/// families plus the `*_CURVE` suffix.
fn is_curve_type(name: &str) -> bool {
    name.ends_with("CURVE")
        || matches!(
            name,
            "CIRCLE" | "ELLIPSE" | "LINE" | "CONIC" | "PARABOLA" | "HYPERBOLA"
        )
}

fn collect_refs_transitive(id: u64, graph: &EntityGraph, skip: &mut HashSet<u64>) {
    if !skip.insert(id) {
        return;
    }
    let Some(entity) = graph.get(id) else {
        return;
    };
    match entity {
        RawEntity::Simple { attributes, .. } => {
            for attr in attributes {
                walk_refs_in_attr(attr, graph, skip);
            }
        }
        RawEntity::Complex { parts, .. } => {
            for part in parts {
                for attr in &part.attributes {
                    walk_refs_in_attr(attr, graph, skip);
                }
            }
        }
    }
}

fn walk_refs_in_attr(attr: &Attribute, graph: &EntityGraph, skip: &mut HashSet<u64>) {
    match attr {
        Attribute::EntityRef(n) => collect_refs_transitive(*n, graph, skip),
        Attribute::List(items) => {
            for item in items {
                walk_refs_in_attr(item, graph, skip);
            }
        }
        Attribute::Typed { value, .. } => walk_refs_in_attr(value, graph, skip),
        _ => {}
    }
}
