//! Converts a raw [`EntityGraph`] into a typed [`StepModel`].
//!
//! This module is the boundary between the parser layer and the IR layer.
//! It uses multi-pass eager conversion: entities are processed in dependency
//! order so that referenced objects are always available when needed.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ir::arena::Arena;
use crate::ir::assembly::{AssemblyTree, Product, Transform3d, WireframeContent};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{SurfaceCurveWrapper, TransitionCode};
use crate::ir::id::{
    ColourId, Curve2dId, CurveId, CurveStyleId, Direction2dId, DirectionId, EdgeId, FaceId,
    FoundedItemId, Placement1dId, Placement2dId, Placement3dId, Point2dId, PointId,
    PreDefinedCurveFontId, PreDefinedSymbolId, PresentationStyleAssignmentId, ProductId, ShellId,
    SolidId, StyledItemId, SurfaceId, SurfaceStyleRenderingId, UnitContextId, VertexId, WireId,
};
use crate::ir::model::{GeometryPool, StepModel, TopologyPool};
use crate::ir::shape_rep::{AngleUnit, LengthUncertainty, LengthUnit, SolidAngleUnit, UnitContext};
use crate::ir::topology::{Orientation, OrientedEdge};
use crate::ir::units::{DerivedUnit, DerivedUnitElement, MeasureWithUnit, NamedUnit, UnitsPool};
use crate::ir::visualization::{FillAreaStyleColour, VisualizationPool};
// PreDefinedCurveFontId / CurveStyleId / StyledItemId imported above; the map types reference them directly.
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

mod dispatch;
mod header;

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
}

/// A NAUO assembly instance with its parent/child resolved but its transform
/// still pending. See [`ReaderContext::pending_nauo_instances`].
pub(crate) struct PendingNauoInstance {
    pub(crate) parent: ProductId,
    pub(crate) child: ProductId,
    pub(crate) occurrence_id: String,
    pub(crate) occurrence_name: String,
    /// STEP id of the NAUO — the key into `nauo_transform_map`, and the
    /// `entity_id` reported if no transform was found.
    pub(crate) nauo_id: u64,
}

/// Accumulates converted IR objects and tracks the mapping from STEP entity
/// ids (`#N`) to typed arena Ids.
#[derive(Default)]
// `solid_angle_unit_map` below has a zero-sized value type; keep it as a
// map for symmetry with the other unit maps and to leave room for future
// `SolidAngleUnit` variants.
#[allow(clippy::zero_sized_map_values)]
pub struct ReaderContext {
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
    /// `GEOMETRIC_ITEM_SPECIFIC_USAGE` step id → arena id (phase gisu).
    /// No step-io entity references GISU today; the map is retained for
    /// symmetry with sibling handlers.
    pub(crate) gisu_id_map: HashMap<u64, crate::ir::id::GeometricItemSpecificUsageId>,
    /// `INVISIBILITY` step id → arena id (phase invisibility).
    pub(crate) invisibility_id_map: HashMap<u64, crate::ir::id::InvisibilityId>,
    /// `PRESENTATION_VIEW` / `PRESENTATION_AREA` step id → arena id
    /// (phase pr-core). Both variants share the same `PresentationRepresentationId`
    /// space — `area_in_set.area` / `presentation_size.unit` resolve here.
    pub(crate) presentation_representation_id_map:
        HashMap<u64, crate::ir::id::PresentationRepresentationId>,
    /// `PRESENTATION_SET` step id → arena id (phase pr-core). Consumed by
    /// `area_in_set.in_set`.
    pub(crate) presentation_set_id_map: HashMap<u64, crate::ir::id::PresentationSetId>,
    /// `AREA_IN_SET` step id → arena id (phase pr-size). Consumed by
    /// `presentation_size.unit` SELECT.
    pub(crate) area_in_set_id_map: HashMap<u64, crate::ir::id::AreaInSetId>,
    /// `REPRESENTATION_CONTEXT #N → UnitContextId`.
    /// Used by representation converters (ABSR, MSSR, plain SR, GBWSR, GBSSR,
    /// MDGPR) to translate their `context_of_items` ref into an `UnitContextId`.
    pub(crate) context_id_map: HashMap<u64, UnitContextId>,
    /// `(GRC PRC REP_CONTEXT) #N → UnitlessContextId` populated by the
    /// `ParametricRepresentationContextHandler`. Representation handlers
    /// look up here after `context_id_map` misses.
    pub(crate) unitless_context_id_map: HashMap<u64, crate::ir::id::UnitlessContextId>,
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
    /// `NAMED_UNIT` complex `#N → NamedUnitId` (units-1). Used by MWU
    /// readers (`unit_component` ref) and the DUE reader (`unit` ref) to
    /// resolve their named-unit reference.
    pub(crate) named_unit_id_map: HashMap<u64, crate::ir::id::NamedUnitId>,
    /// `MEASURE_WITH_UNIT` `#N → MeasureWithUnitId` (units-1).
    pub(crate) mwu_id_map: HashMap<u64, crate::ir::id::MeasureWithUnitId>,
    /// `DERIVED_UNIT_ELEMENT` `#N → DerivedUnitElementId` (units-1).
    pub(crate) due_id_map: HashMap<u64, crate::ir::id::DerivedUnitElementId>,
    /// `DERIVED_UNIT` arena (units-1b). Populated by the `DERIVED_UNIT` handler.
    pub(crate) derived_unit_arena: Arena<DerivedUnit>,
    /// `DIMENSIONAL_EXPONENTS` arena (phase dim-exp-arena-a). Populated by a
    /// dedicated `DIMENSIONAL_EXPONENTS` handler so `NAMED_UNIT` subtypes can
    /// look up the ref in [`Self::dim_exp_id_map`].
    pub(crate) dimensional_exponents: Arena<crate::ir::units::DimensionalExponents>,
    /// `DIMENSIONAL_EXPONENTS` `#N → DimensionalExponentsId`.
    pub(crate) dim_exp_id_map: HashMap<u64, crate::ir::DimensionalExponentsId>,
    /// `DERIVED_UNIT` `#N → DerivedUnitId` (units-1b). Reserved for future
    /// consumers (no entity references it in the current IR).
    pub(crate) derived_unit_id_map: HashMap<u64, crate::ir::id::DerivedUnitId>,
    /// MWU step ids that appear as the `conversion_factor` of a
    /// `CONVERSION_BASED_UNIT` complex (units-1). Populated by the unit
    /// complex handlers and consulted by the `MEASURE_WITH_UNIT` readers —
    /// those MWUs are re-emitted inline by the CBU writer
    /// chain, so adding them to `mwu_arena` would cause double-emit on
    /// round-trip.
    pub(crate) cbu_internal_mwu_refs: HashSet<u64>,
    /// units-2: `CBU outer entity_id → conversion_factor MWU entity_id`.
    /// Populated by `read_conversion_based_unit_body` (`LENGTH` / `PLANE_ANGLE`
    /// / `MASS` branches) and consumed by the `backfill_cbu_base` post-pass
    /// to set each outer's `LengthFlavor.cbu_base` (etc.) to the
    /// `NamedUnitId` of its base SI entry.
    pub(crate) cbu_outer_to_mwu: HashMap<u64, u64>,
    /// `UNCERTAINTY_MEASURE_WITH_UNIT #N → value+metadata` for uncertainty
    /// entities whose `unit_component` resolved to a length unit. Populated
    /// when `UNCERTAINTY_MEASURE_WITH_UNIT` entities are read.
    pub(crate) length_uncertainty_map: HashMap<u64, LengthUncertainty>,
    /// Same shape as `length_uncertainty_map` but keyed on UMUs whose
    /// `unit_component` is a plane-angle unit.
    pub(crate) plane_angle_uncertainty_map: HashMap<u64, LengthUncertainty>,
    /// Same shape but for solid-angle uncertainty UMUs.
    pub(crate) solid_angle_uncertainty_map: HashMap<u64, LengthUncertainty>,

    // Geometry maps: STEP #N → typed Id.
    pub(crate) point_map: HashMap<u64, PointId>,
    pub(crate) direction_map: HashMap<u64, DirectionId>,
    pub(crate) surface_map: HashMap<u64, SurfaceId>,
    pub(crate) curve_map: HashMap<u64, CurveId>,

    // Geometry intermediate maps.
    pub(crate) placement_map: HashMap<u64, Placement3dId>,
    pub(crate) vector_map: HashMap<u64, (DirectionId, f64)>,
    pub(crate) axis1_map: HashMap<u64, Placement1dId>,
    /// `PLANAR_EXTENT` / `PLANAR_BOX` `#N → PlanarExtentId` (phase view-volume).
    /// Lets `VIEW_VOLUME` resolve its `view_window` ref.
    pub(crate) planar_extent_id_map: HashMap<u64, crate::ir::PlanarExtentId>,

    // 2D geometry (PCURVE parametric space) maps.
    pub(crate) point_2d_map: HashMap<u64, Point2dId>,
    pub(crate) direction_2d_map: HashMap<u64, Direction2dId>,
    pub(crate) curve_2d_map: HashMap<u64, Curve2dId>,
    pub(crate) vector_2d_map: HashMap<u64, (Direction2dId, f64)>,
    pub(crate) placement_2d_map: HashMap<u64, Placement2dId>,
    /// `SURFACE_CURVE / SEAM_CURVE #N → SurfaceCurveWrapper`. Populated by the
    /// `SURFACE_CURVE` / `SEAM_CURVE` handler and consumed by `convert_edge_curve`
    /// to attach the wrapper (entity kind, associated geometry,
    /// `master_representation`) to each edge.
    pub(crate) surface_curve_map: HashMap<u64, SurfaceCurveWrapper>,

    // Topology maps: STEP #N → typed Id.
    pub(crate) vertex_map: HashMap<u64, VertexId>,
    pub(crate) edge_map: HashMap<u64, EdgeId>,
    pub(crate) face_bound_map: HashMap<u64, WireId>,
    pub(crate) face_map: HashMap<u64, FaceId>,
    pub(crate) shell_map: HashMap<u64, ShellId>,
    pub(crate) solid_map: HashMap<u64, SolidId>,

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
    pub(crate) product_contexts: Arena<crate::ir::ProductContext>,
    pub(crate) product_definition_contexts: Arena<crate::ir::ProductDefinitionContext>,
    pub(crate) product_context_id_map: HashMap<u64, crate::ir::ProductContextId>,
    pub(crate) product_definition_context_id_map:
        HashMap<u64, crate::ir::ProductDefinitionContextId>,
    /// `ProductId → raw STEP entity id of the product's first PC`
    /// (`PRODUCT.frame_of_reference[0]`). `resolve_product_contexts`
    /// converts these refs to `ProductContextId`.
    pub(crate) product_pc_step_refs: HashMap<ProductId, u64>,
    /// Same pattern, populated by `PRODUCT_DEFINITION.frame_of_reference`.
    pub(crate) product_pdc_step_refs: HashMap<ProductId, u64>,
    pub(crate) product_definition_context_roles: Arena<crate::ir::ProductDefinitionContextRole>,
    pub(crate) product_definition_context_associations:
        Arena<crate::ir::ProductDefinitionContextAssociation>,
    pub(crate) pdc_role_id_map: HashMap<u64, crate::ir::ProductDefinitionContextRoleId>,
    pub(crate) pdca_id_map: HashMap<u64, crate::ir::ProductDefinitionContextAssociationId>,
    pub(crate) product_definition_relationships: Arena<crate::ir::ProductDefinitionRelationship>,
    pub(crate) pdr_id_map: HashMap<u64, crate::ir::ProductDefinitionRelationshipId>,
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
    /// `form_features` pool staging. Moved into `StepModel.form_features` at `into_model` time.
    pub(crate) form_features: Option<crate::ir::FormFeaturesPool>,
    /// `STEP feature` `#N → FeatureDefinitionId`. Reserved for future PMI consumers.
    pub(crate) feature_definition_id_map: HashMap<u64, crate::ir::FeatureDefinitionId>,
    pub(crate) product_arena_map: HashMap<u64, ProductId>,
    pub(crate) formation_to_product: HashMap<u64, u64>,
    pub(crate) pdef_to_product: HashMap<u64, u64>,
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
    /// `SDR → plain SR → SRR → ABSR/MSSR`.
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
    /// `PRODUCT_DEFINITION_SHAPE #N → PRODUCT_DEFINITION #N` when the
    /// `pdef_shape` points at a product definition (not a `NAUO`).
    /// Populated by the `PRODUCT_DEFINITION_SHAPE` handler.
    pub(crate) pdef_shape_to_pdef: HashMap<u64, u64>,
    /// `PRODUCT_DEFINITION_SHAPE #N → NEXT_ASSEMBLY_USAGE_OCCURRENCE #N` when
    /// the `pdef_shape` points at a `NAUO` (instance-tagged). Populated
    /// alongside `pdef_shape_to_pdef` and consumed during NAUO instance wiring.
    pub(crate) pdef_shape_to_nauo: HashMap<u64, u64>,
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
    /// `COLOUR_RGB` / `DRAUGHTING_PRE_DEFINED_COLOUR` step entity id →
    /// `ColourId`. Both colour-family readers push into the
    /// `Arena<Colour>` on `ctx.visualization` and record the resulting id
    /// here so downstream consumers (`FILL_AREA_STYLE_COLOUR`,
    /// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) can resolve a colour ref
    /// to an arena index without copying the colour data.
    pub(crate) viz_colour_id_map: HashMap<u64, ColourId>,
    /// `PRE_DEFINED_CURVE_FONT` / `DRAUGHTING_PRE_DEFINED_CURVE_FONT` step
    /// entity id → `PreDefinedCurveFontId`. Populated by the curve-font
    /// leaf readers; consumed by the `CURVE_STYLE` reader
    /// to resolve a font ref to an arena index.
    pub(crate) viz_pre_defined_curve_font_id_map: HashMap<u64, PreDefinedCurveFontId>,
    /// `PRE_DEFINED_SYMBOL` / `PRE_DEFINED_TERMINATOR_SYMBOL` step entity id
    /// → `PreDefinedSymbolId`. Populated by the symbol leaf readers.
    /// No step-io consumer reads this map yet; entries exist so
    /// the writer round-trips the source symbols unchanged.
    pub(crate) viz_pre_defined_symbol_id_map: HashMap<u64, PreDefinedSymbolId>,
    /// `CURVE_STYLE` step entity id → `CurveStyleId`. Populated by the
    /// `CURVE_STYLE` handler; consumed by the PSA reader to dispatch styling
    /// list entries to the `PsaStyle::Curve(...)` variant.
    pub(crate) viz_curve_style_id_map: HashMap<u64, CurveStyleId>,
    /// `FILL_AREA_STYLE_COLOUR #N → FillAreaStyleColour`.
    pub(crate) viz_fasc_map: HashMap<u64, FillAreaStyleColour>,
    /// `FILL_AREA_STYLE` step entity id → `FoundedItemId`. Populated by the
    /// FAS handler after pushing the `FoundedItem::FillAreaStyle` variant
    /// into `VisualizationPool::founded_items`; consumed by the
    /// `SURFACE_STYLE_FILL_AREA` reader to resolve its `fill_area` ref.
    pub(crate) viz_fas_id_map: HashMap<u64, FoundedItemId>,
    /// `SURFACE_STYLE_FILL_AREA` step entity id → `FoundedItemId`.
    /// Populated by the SSFA handler after pushing the
    /// `FoundedItem::SurfaceStyleFillArea` variant; consumed by the
    /// `SURFACE_SIDE_STYLE` reader for `SurfaceSideStyleEntry::FillArea`.
    pub(crate) viz_ssfa_id_map: HashMap<u64, FoundedItemId>,
    /// `SURFACE_STYLE_RENDERING #N | SURFACE_STYLE_RENDERING_WITH_PROPERTIES #N
    /// → SurfaceStyleRenderingId`. Populated by the SSR handlers after
    /// pushing the resolved enum variant into
    /// `VisualizationPool::surface_style_renderings`; consumed by the
    /// `SURFACE_SIDE_STYLE` reader to build `SurfaceSideStyleEntry::Rendering(...)`.
    pub(crate) viz_ssr_id_map: HashMap<u64, SurfaceStyleRenderingId>,
    /// `SURFACE_STYLE_TRANSPARENT #N → transparency value`.
    pub(crate) viz_transparent_map: HashMap<u64, f64>,
    /// `SURFACE_SIDE_STYLE` step entity id → `FoundedItemId`. Populated by
    /// the SSS handler after pushing the `FoundedItem::SurfaceSideStyle`
    /// variant; consumed by the `SURFACE_STYLE_USAGE` reader for its
    /// `style` ref.
    pub(crate) viz_sss_id_map: HashMap<u64, FoundedItemId>,
    /// `SURFACE_STYLE_USAGE` step entity id → `FoundedItemId`. Populated by
    /// the SSU handler after pushing the `FoundedItem::SurfaceStyleUsage`
    /// variant; consumed by the PSA reader for `PsaStyle::Surface(...)`.
    pub(crate) viz_ssu_id_map: HashMap<u64, FoundedItemId>,
    /// `VIEW_VOLUME` step entity id → `FoundedItemId` (phase camera-model-d3).
    /// Populated by the `VIEW_VOLUME` handler; consumed by `CAMERA_MODEL_D3`
    /// for its `perspective_of_volume` ref.
    pub(crate) viz_view_volume_id_map: HashMap<u64, FoundedItemId>,
    /// `SURFACE_STYLE_BOUNDARY` step entity id → `FoundedItemId` (phase ssb).
    pub(crate) viz_ssb_id_map: HashMap<u64, FoundedItemId>,
    /// `CAMERA_MODEL*` step entity id → `CameraModelId`. Populated by the
    /// `CAMERA_MODEL*` handlers; consumed by the camera-usage handler to
    /// resolve `mapping_origin`.
    pub(crate) viz_camera_model_id_map: HashMap<u64, crate::ir::id::CameraModelId>,
    /// `PRESENTATION_STYLE_ASSIGNMENT` step entity id →
    /// `PresentationStyleAssignmentId`. Populated by the PSA handler after
    /// pushing the resolved variant into
    /// `VisualizationPool::presentation_style_assignments`; consumed by the
    /// `STYLED_ITEM` / `OVER_RIDING_STYLED_ITEM` readers to convert their
    /// `styles` ref lists into arena ids.
    pub(crate) viz_psa_id_map: HashMap<u64, PresentationStyleAssignmentId>,
    /// `STYLED_ITEM` step entity id → `StyledItemId`. Populated by the
    /// `STYLED_ITEM` reader after pushing the resolved `StyledItem`
    /// variant into `VisualizationPool::styled_items`. Consumed by the
    /// MDGPR reader to convert its `items` list into arena references.
    pub(crate) viz_styled_item_id_map: HashMap<u64, StyledItemId>,

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
    /// `PROPERTY_DEFINITION #N → PropertyId`. Recorded by the
    /// PDR reader when it pushes a `Property`; consumed by the GPA reader
    /// to resolve `derived_definition`. Temp; discarded after the property handlers run.
    pub(crate) property_step_to_id: HashMap<u64, crate::ir::PropertyId>,
    /// `PROPERTY_DEFINITION` / `PRODUCT_DEFINITION_SHAPE` #N →
    /// `PropertyDefinitionId`. Recorded by the PD /
    /// PDS handlers when they push into the schema-faithful
    /// `property_definitions` arena. Consumed by:
    /// - the PDR handler — fills the new `Property.definition` field;
    /// - the GPA handler — resolves `derived_definition` to a
    ///   `PropertyDefinitionId` (replacing the legacy `property_step_to_id`
    ///   lookup for that purpose). Phase property-definition-2.
    pub(crate) property_def_step_to_id: HashMap<u64, crate::ir::PropertyDefinitionId>,
    /// `GENERAL_PROPERTY #N → GeneralPropertyId`. Consumed by
    /// the GPA reader to resolve `base_definition`. Temp; discarded after
    /// the property handlers run.
    pub(crate) general_property_id_map: HashMap<u64, crate::ir::GeneralPropertyId>,

    /// `pmi` pool — populated by the PMI handlers. `None` until the
    /// first PMI entity is seen.
    pub(crate) pmi: Option<crate::ir::pmi::PmiPool>,

    /// `SHAPE_ASPECT` arena — the `SHAPE_ASPECT` converter pushes
    /// here. Empty when no PMI entities were seen.
    pub(crate) shape_aspects: crate::ir::Arena<crate::ir::ShapeAspect>,
    /// `SHAPE_ASPECT` `#N → ShapeAspectId`. Populated by the `SHAPE_ASPECT`
    /// handler; consumed by `id_attribute` and future PMI
    /// handlers that resolve shape-aspect refs.
    pub(crate) shape_aspect_id_map: HashMap<u64, crate::ir::ShapeAspectId>,
    /// `SHAPE_ASPECT` subtype arenas. No STEP-id map (no consumer
    /// yet; the entities round-trip as standalone arena entries).
    pub(crate) composite_group_shape_aspects:
        crate::ir::Arena<crate::ir::CompositeGroupShapeAspect>,
    pub(crate) centre_of_symmetries: crate::ir::Arena<crate::ir::CentreOfSymmetry>,
    pub(crate) all_around_shape_aspects: crate::ir::Arena<crate::ir::AllAroundShapeAspect>,
    /// `SHAPE_ASPECT` subtype `#N → …Id` maps (phase shape-aspect-ref).
    /// Populated by the `SHAPE_ASPECT` subtype handlers; consumed by
    /// `resolve_shape_aspect_ref` for `ShapeAspectRef` resolution.
    pub(crate) composite_shape_aspect_id_map: HashMap<u64, crate::ir::CompositeShapeAspectId>,
    pub(crate) centre_of_symmetry_id_map: HashMap<u64, crate::ir::DerivedShapeAspectId>,
    pub(crate) all_around_shape_aspect_id_map: HashMap<u64, crate::ir::ContinuousShapeAspectId>,
    /// `DATUM` / `DATUM_FEATURE` `#N → …Id` maps (phase datum-feature). The
    /// `Datum` / `DatumFeature` arenas live in [`PmiPool`]; these maps let
    /// `resolve_shape_aspect_ref` resolve a `shape_aspect` ref onto them.
    pub(crate) datum_id_map: HashMap<u64, crate::ir::DatumId>,
    pub(crate) datum_feature_id_map: HashMap<u64, crate::ir::DatumFeatureId>,
    /// `DATUM_SYSTEM` arena + `#N → DatumSystemId` map (phase datum-system).
    /// Populated by the `DATUM_SYSTEM` handler; the map lets `resolve_shape_aspect_ref`
    /// resolve a `shape_aspect` ref onto a datum system.
    pub(crate) datum_systems: crate::ir::Arena<crate::ir::DatumSystem>,
    pub(crate) datum_system_id_map: HashMap<u64, crate::ir::DatumSystemId>,
    /// `general_datum_reference` `#N → GeneralDatumReferenceId` map (phase
    /// datum-system). Populated by the `DATUM_REFERENCE_*` handlers; consumed
    /// by `DATUM_SYSTEM`'s `constituents` resolution.
    pub(crate) general_datum_reference_id_map: HashMap<u64, crate::ir::GeneralDatumReferenceId>,
    /// `#N → …Id` maps for the plus-minus-tolerance cluster. `tolerance_value`
    /// / `limits_and_fits` feed `PLUS_MINUS_TOLERANCE.range`; the dimensional
    /// maps feed its `toleranced_dimension` (the `dimensional_location` /
    /// `dimensional_size` handlers populate those — added this phase).
    pub(crate) tolerance_value_id_map: HashMap<u64, crate::ir::ToleranceValueId>,
    pub(crate) limits_and_fits_id_map: HashMap<u64, crate::ir::LimitsAndFitsId>,
    pub(crate) dimensional_location_id_map: HashMap<u64, crate::ir::DimensionalLocationId>,
    pub(crate) dimensional_size_id_map: HashMap<u64, crate::ir::DimensionalSizeId>,
    /// `TOLERANCE_ZONE` arena + `#N → …Id` maps it depends on (phase
    /// tolerance-zone). The two geometric-tolerance maps feed
    /// `TOLERANCE_ZONE.defining_tolerance`, `tolerance_zone_form_id_map` its
    /// `form`. The handlers for those entities populate the maps this phase.
    pub(crate) geometric_tolerance_id_map: HashMap<u64, crate::ir::GeometricToleranceId>,
    pub(crate) geometric_tolerance_with_datum_reference_id_map:
        HashMap<u64, crate::ir::GeometricToleranceWithDatumReferenceId>,
    pub(crate) tolerance_zone_form_id_map: HashMap<u64, crate::ir::ToleranceZoneFormId>,
    pub(crate) tolerance_zones: crate::ir::Arena<crate::ir::ToleranceZone>,
    /// `DATUM_TARGET` arena + step entity id → `DatumTargetId` map.
    /// Phase datum-target.
    pub(crate) datum_targets: crate::ir::Arena<crate::ir::shape_rep::DatumTarget>,
    pub(crate) datum_target_id_map: HashMap<u64, crate::ir::DatumTargetId>,
    /// `PLACED_DATUM_TARGET_FEATURE` arena + id map.
    pub(crate) placed_datum_target_features:
        crate::ir::Arena<crate::ir::shape_rep::PlacedDatumTargetFeature>,
    pub(crate) placed_datum_target_feature_id_map:
        HashMap<u64, crate::ir::PlacedDatumTargetFeatureId>,
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
    /// `SYMBOL_COLOUR` step entity id → `SymbolColourId` (phase symbol-colour).
    pub(crate) symbol_colour_id_map: HashMap<u64, crate::ir::id::SymbolColourId>,
    /// `TEXT_STYLE_FOR_DEFINED_FONT` step entity id →
    /// `TextStyleForDefinedFontId` (phase text-style-font).
    pub(crate) text_style_for_defined_font_id_map:
        HashMap<u64, crate::ir::id::TextStyleForDefinedFontId>,
    /// `PRE_DEFINED_MARKER` step entity id → `PreDefinedMarkerId` (phase
    /// pre-defined-marker).
    pub(crate) viz_pre_defined_marker_id_map: HashMap<u64, crate::ir::id::PreDefinedMarkerId>,
    /// `text_style` enum-arena step id → `TextStyleId` (phase text-style-box).
    /// Populated by the `TEXT_STYLE_WITH_BOX_CHARACTERISTICS` handler.
    pub(crate) text_style_id_map: HashMap<u64, crate::ir::id::TextStyleId>,
    /// `DRAUGHTING_PRE_DEFINED_TEXT_FONT` step id → arena id (phase
    /// text-literal — `id_map` populated retroactively now that `TEXT_LITERAL`
    /// references it through the `font_select` SELECT).
    pub(crate) dptf_id_map: HashMap<u64, crate::ir::id::DraughtingPreDefinedTextFontId>,
    /// `TEXT_LITERAL` step id → `TextLiteralId` (phase text-literal).
    /// Consumed by `COMPOSITE_TEXT` for the `text_or_character` SELECT.
    pub(crate) text_literal_id_map: HashMap<u64, crate::ir::id::TextLiteralId>,
    /// `DRAUGHTING_MODEL_ITEM_ASSOCIATION` step id → arena id (phase dmia).
    /// No other entity references DMIA in the modelled corpus; cache kept
    /// for symmetry / future ref-receiving entities.
    pub(crate) dmia_id_map: HashMap<u64, crate::ir::id::DraughtingModelItemAssociationId>,
    /// `REPRESENTATION_ITEM` step entity id → `RepresentationItemId`
    /// (phase repr-item-arena-1). Populated by QRI / VRI handlers;
    /// consumed by `resolve_representation_item_ref` as last-resort
    /// fallback (per-type arena lookups take priority).
    pub(crate) repr_item_id_map: HashMap<u64, crate::ir::id::RepresentationItemId>,
    /// `REPRESENTATION` `#N → RepresentationId`. Populated by the six
    /// representation handlers; consumed by the SDR reader to link each
    /// product to its `representation_id` / `outer_representation_id`.
    pub(crate) repr_id_map: HashMap<u64, crate::ir::RepresentationId>,
    /// `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` `#N → CharacterizedObjectId`
    /// for standalone CIWRs. Lets `PROPERTY_DEFINITION.definition` resolve a
    /// CIWR target (geometric-validation property shapes).
    pub(crate) characterized_object_id_map: HashMap<u64, crate::ir::CharacterizedObjectId>,
    /// `REPRESENTATION_MAP` arena + `#N → RepresentationMapId` (phase
    /// mapped-item). The map lets the `MAPPED_ITEM` handler resolve its
    /// `mapping_source` ref.
    pub(crate) representation_maps: crate::ir::Arena<crate::ir::shape_rep::RepresentationMap>,
    pub(crate) representation_map_id_map: HashMap<u64, crate::ir::RepresentationMapId>,
    /// `MAPPED_ITEM` arena (phase mapped-item) — orphan round-trip.
    pub(crate) mapped_items: crate::ir::Arena<crate::ir::shape_rep::MappedItem>,
    /// `MAPPED_ITEM` STEP `#N → MappedItemId`. Populated by the `MAPPED_ITEM`
    /// and `CAMERA_IMAGE` handlers so `STYLED_ITEM` / `CDORSI` resolvers can
    /// surface a `RepresentationItemRef::MappedItem` (phase si-mapped-item).
    pub(crate) mapped_item_id_map: HashMap<u64, crate::ir::MappedItemId>,
    /// `INTEGER`/`REAL_REPRESENTATION_ITEM` value-item arena (phase
    /// numeric-representation-item) — orphan round-trip.
    pub(crate) numeric_representation_items:
        crate::ir::Arena<crate::ir::shape_rep::NumericRepresentationItem>,
    /// `tessellated_item` arena (`COORDINATES_LIST`) + `#N → TessellatedItemId`
    /// (phase tessellation). The map lets the `COMPLEX_TRIANGULATED_FACE`
    /// handler resolve its `coordinates` ref.
    pub(crate) tessellated_items: crate::ir::Arena<crate::ir::tessellation::TessellatedItem>,
    pub(crate) tessellated_item_id_map: HashMap<u64, crate::ir::TessellatedItemId>,
    /// `LEADER_CURVE` step entity id → `AnnotationCurveOccurrenceId`
    /// (phase annotation-curve-leader). Populated by `LeaderCurveHandler`;
    /// consumed by `TerminatorSymbolHandler` / `LeaderTerminatorHandler` to
    /// resolve `annotated_curve`.
    pub(crate) annotation_curve_occurrence_id_map:
        HashMap<u64, crate::ir::id::AnnotationCurveOccurrenceId>,
    /// `annotation_occurrence` step entity id → `AnnotationOccurrenceId`
    /// (phase draughting-callout). Populated by every
    /// `annotation_occurrence` enum handler after pushing into the
    /// `annotation_occurrences` arena; consumed by `DraughtingCalloutHandler`
    /// / `LeaderDirectedCalloutHandler` to resolve `contents` SELECT
    /// members.
    pub(crate) annotation_occurrence_id_map: HashMap<u64, crate::ir::id::AnnotationOccurrenceId>,
    /// `DRAUGHTING_CALLOUT` step entity id → `DraughtingCalloutId` (phase
    /// draughting-callout). Populated by `DraughtingCalloutHandler` /
    /// `LeaderDirectedCalloutHandler`; consumed by
    /// `DraughtingCalloutRelationshipHandler` to resolve relating / related
    /// refs.
    pub(crate) draughting_callout_id_map: HashMap<u64, crate::ir::id::DraughtingCalloutId>,
    /// `TOLERANCE_ZONE` step entity id → `ToleranceZoneId` (phase
    /// projected-zone). Populated by `ToleranceZoneHandler`; consumed by
    /// `ProjectedZoneDefinitionHandler` for the `zone` ref.
    pub(crate) tolerance_zone_id_map: HashMap<u64, crate::ir::id::ToleranceZoneId>,
    /// `TYPE_QUALIFIER` step entity id → `TypeQualifierId` (phase
    /// measure-qualification). Consumed by `MeasureQualificationHandler`
    /// to resolve a `qualifiers` SET member.
    pub(crate) type_qualifier_id_map: HashMap<u64, crate::ir::id::TypeQualifierId>,
    /// `VALUE_FORMAT_TYPE_QUALIFIER` step entity id →
    /// `ValueFormatTypeQualifierId` (phase measure-qualification). Same
    /// role as `type_qualifier_id_map`.
    pub(crate) value_format_type_qualifier_id_map:
        HashMap<u64, crate::ir::id::ValueFormatTypeQualifierId>,
    /// `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` step entity id →
    /// `DimensionalCharacteristicRepresentationId` (phase sdr-dcr).
    /// Populated by `DimensionalCharacteristicRepresentationHandler`;
    /// currently no consumer.
    pub(crate) dimensional_characteristic_representation_id_map:
        HashMap<u64, crate::ir::id::DimensionalCharacteristicRepresentationId>,
    /// `COMPLEX_TRIANGULATED_FACE` arena (phase tessellation).
    pub(crate) tessellated_faces:
        crate::ir::Arena<crate::ir::tessellation::ComplexTriangulatedFace>,
    /// `COMPLEX_TRIANGULATED_SURFACE_SET` arena (phase tessellation-2).
    pub(crate) tessellated_surface_sets:
        crate::ir::Arena<crate::ir::tessellation::ComplexTriangulatedSurfaceSet>,
    /// `#N → TessellatedFaceId` / `#N → TessellatedSurfaceSetId` (phase
    /// tessellated-item-ref). Let `resolve_tessellated_item_ref` resolve a
    /// `set_ref_tessellated_item` member into a [`crate::ir::TessellatedItemRef`].
    pub(crate) tessellated_face_id_map: HashMap<u64, crate::ir::TessellatedFaceId>,
    pub(crate) tessellated_surface_set_id_map: HashMap<u64, crate::ir::TessellatedSurfaceSetId>,

    /// `geometric_representation_item` enum arena (phase ds-st).
    pub(crate) geometric_representation_items:
        crate::ir::Arena<crate::ir::visualization::GeometricRepresentationItem>,
    /// `representation_relationship` enum arena (phase cgrr).
    pub(crate) representation_relationships:
        crate::ir::Arena<crate::ir::shape_rep::RepresentationRelationship>,
    /// `compound_representation_item` arena (phase cri).
    pub(crate) compound_representation_items:
        crate::ir::Arena<crate::ir::shape_rep::CompoundRepresentationItem>,
    /// `item_identified_representation_usage` arena (phase iiru).
    pub(crate) item_identified_representation_usages:
        crate::ir::Arena<crate::ir::shape_rep::ItemIdentifiedRepresentationUsage>,
    /// `SYMBOL_TARGET` step entity id → `GeometricRepresentationItemId`.
    /// Populated by the `SymbolTarget` reader so `DEFINED_SYMBOL.target`
    /// can resolve.
    pub(crate) symbol_target_id_map: HashMap<u64, crate::ir::id::GeometricRepresentationItemId>,

    /// Lazily-built plm pool — populated by the plm reader chain
    /// (`CalendarDate` / `LocalTime` / UTC / `DateAndTime` / `DateTimeRole`
    /// in Phase plm-1a; Person/Approval/Security clusters later).
    pub(crate) plm: Option<crate::ir::PlmPool>,
    /// `CALENDAR_DATE` step entity id → `DateId`.
    pub(crate) plm_date_id_map: HashMap<u64, crate::ir::DateId>,
    /// `LOCAL_TIME` step entity id → `LocalTimeId`.
    pub(crate) plm_local_time_id_map: HashMap<u64, crate::ir::LocalTimeId>,
    /// `COORDINATED_UNIVERSAL_TIME_OFFSET` step entity id → `CoordinatedUniversalTimeOffsetId`.
    pub(crate) plm_utc_id_map: HashMap<u64, crate::ir::CoordinatedUniversalTimeOffsetId>,
    /// `DATE_AND_TIME` step entity id → `DateAndTimeId`.
    pub(crate) plm_date_and_time_id_map: HashMap<u64, crate::ir::DateAndTimeId>,
    /// `DATE_TIME_ROLE` step entity id → `DateTimeRoleId`.
    pub(crate) plm_date_time_role_id_map: HashMap<u64, crate::ir::DateTimeRoleId>,
    /// `APPLIED_DATE_AND_TIME_ASSIGNMENT` /
    /// `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` step entity id →
    /// `DateAndTimeAssignmentId`.
    pub(crate) plm_dta_id_map: HashMap<u64, crate::ir::DateAndTimeAssignmentId>,
    /// `PERSON` step entity id → `PersonId`.
    pub(crate) plm_person_id_map: HashMap<u64, crate::ir::PersonId>,
    /// `ORGANIZATION` step entity id → `OrganizationId`.
    pub(crate) plm_organization_id_map: HashMap<u64, crate::ir::OrganizationId>,
    /// `PERSON_AND_ORGANIZATION` step entity id → `PersonAndOrganizationId`.
    pub(crate) plm_p_and_o_id_map: HashMap<u64, crate::ir::PersonAndOrganizationId>,
    /// `PERSON_AND_ORGANIZATION_ROLE` step entity id → `PersonAndOrganizationRoleId`.
    pub(crate) plm_p_and_o_role_id_map: HashMap<u64, crate::ir::PersonAndOrganizationRoleId>,
    /// `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` /
    /// `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT` step entity id →
    /// `PersonAndOrganizationAssignmentId`.
    pub(crate) plm_poa_id_map: HashMap<u64, crate::ir::PersonAndOrganizationAssignmentId>,
    /// `APPROVAL_STATUS` step entity id → `ApprovalStatusId`.
    pub(crate) plm_approval_status_id_map: HashMap<u64, crate::ir::ApprovalStatusId>,
    /// `APPROVAL_ROLE` step entity id → `ApprovalRoleId`.
    pub(crate) plm_approval_role_id_map: HashMap<u64, crate::ir::ApprovalRoleId>,
    /// `APPROVAL` step entity id → `ApprovalId`.
    pub(crate) plm_approval_id_map: HashMap<u64, crate::ir::ApprovalId>,
    /// `APPROVAL_DATE_TIME` step entity id → `ApprovalDateTimeId`.
    pub(crate) plm_approval_date_time_id_map: HashMap<u64, crate::ir::ApprovalDateTimeId>,
    /// `APPROVAL_PERSON_ORGANIZATION` step entity id → `ApprovalPersonOrganizationId`.
    pub(crate) plm_approval_person_organization_id_map:
        HashMap<u64, crate::ir::ApprovalPersonOrganizationId>,
    /// `APPLIED_APPROVAL_ASSIGNMENT` / `CC_DESIGN_APPROVAL` step entity id →
    /// `ApprovalAssignmentId`.
    pub(crate) plm_aa_id_map: HashMap<u64, crate::ir::ApprovalAssignmentId>,
    /// `SECURITY_CLASSIFICATION_LEVEL` step entity id → `SecurityClassificationLevelId`.
    pub(crate) plm_security_level_id_map: HashMap<u64, crate::ir::SecurityClassificationLevelId>,
    /// `SECURITY_CLASSIFICATION` step entity id → `SecurityClassificationId`.
    pub(crate) plm_security_classification_id_map:
        HashMap<u64, crate::ir::SecurityClassificationId>,
    /// `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` /
    /// `CC_DESIGN_SECURITY_CLASSIFICATION` step entity id →
    /// `SecurityClassificationAssignmentId`.
    pub(crate) plm_sca_id_map: HashMap<u64, crate::ir::SecurityClassificationAssignmentId>,
    /// `IDENTIFICATION_ROLE` step entity id → `IdentificationRoleId`.
    pub(crate) plm_identification_role_id_map: HashMap<u64, crate::ir::IdentificationRoleId>,
    /// `EXTERNAL_SOURCE` step entity id → `ExternalSourceId`.
    pub(crate) plm_external_source_id_map: HashMap<u64, crate::ir::ExternalSourceId>,
    /// `APPLIED_IDENTIFICATION_ASSIGNMENT` /
    /// `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` step entity id →
    /// `IdentificationAssignmentId`.
    pub(crate) plm_ia_id_map: HashMap<u64, crate::ir::IdentificationAssignmentId>,
    /// `DOCUMENT_TYPE` step entity id → `DocumentTypeId`.
    pub(crate) plm_document_type_id_map: HashMap<u64, crate::ir::DocumentTypeId>,
    /// `DOCUMENT` / `DOCUMENT_FILE` step entity id → `DocumentId`.
    pub(crate) plm_document_id_map: HashMap<u64, crate::ir::DocumentId>,
    /// `DOCUMENT_REPRESENTATION_TYPE` step entity id → `DocumentRepresentationTypeId`.
    pub(crate) plm_document_representation_type_id_map:
        HashMap<u64, crate::ir::DocumentRepresentationTypeId>,
    /// `DOCUMENT_PRODUCT_EQUIVALENCE` step entity id → `DocumentProductEquivalenceId`.
    pub(crate) plm_document_product_equivalence_id_map:
        HashMap<u64, crate::ir::DocumentProductEquivalenceId>,
    /// `APPLIED_DOCUMENT_REFERENCE` step entity id → `DocumentReferenceId`.
    pub(crate) plm_document_reference_id_map: HashMap<u64, crate::ir::DocumentReferenceId>,
    /// `GROUP` step entity id → `GroupId`.
    pub(crate) plm_group_id_map: HashMap<u64, crate::ir::GroupId>,
    /// `APPLIED_GROUP_ASSIGNMENT` step entity id → `GroupAssignmentId`.
    pub(crate) plm_ga_id_map: HashMap<u64, crate::ir::GroupAssignmentId>,
    /// `OBJECT_ROLE` step entity id → `ObjectRoleId`.
    pub(crate) plm_object_role_id_map: HashMap<u64, crate::ir::ObjectRoleId>,
    /// `ROLE_ASSOCIATION` step entity id → `RoleAssociationId`.
    pub(crate) plm_role_association_id_map: HashMap<u64, crate::ir::RoleAssociationId>,
    /// `ADDRESS` / `PERSONAL_ADDRESS` step entity id → `AddressId`.
    pub(crate) plm_address_id_map: HashMap<u64, crate::ir::AddressId>,
    /// `APPLICATION_CONTEXT` step entity id → `ApplicationContextId`.
    pub(crate) plm_application_context_id_map: HashMap<u64, crate::ir::ApplicationContextId>,
    /// `APPLICATION_PROTOCOL_DEFINITION` step entity id → `ApplicationProtocolDefinitionId`.
    pub(crate) plm_application_protocol_definition_id_map:
        HashMap<u64, crate::ir::ApplicationProtocolDefinitionId>,

    /// Per-file tally of non-standard required fields the reader normalized
    /// to a standard default. Key = (type-qualified field description,
    /// normalized-to description), value = count. Flushed into `warnings` as
    /// one [`ConvertError::NonStandardInput`] per key at the end of `convert`.
    pub(crate) nonstandard_normalizations: BTreeMap<(String, &'static str), usize>,

    /// Part-sets already reported via `ConvertError::UnhandledComplex` this
    /// file, so a shape recurring across many instances warns once.
    pub(crate) unhandled_complex_seen: std::collections::BTreeSet<String>,

    pub(crate) warnings: Vec<ConvertError>,
}

impl ReaderContext {
    /// Record that a required field was non-standard (Unset / unrecognized)
    /// and the reader normalized it to `normalized_to`. Aggregated per file
    /// so a 980k-instance defect surfaces as a single summary warning.
    pub(crate) fn record_nonstandard(&mut self, field: String, normalized_to: &'static str) {
        *self
            .nonstandard_normalizations
            .entry((field, normalized_to))
            .or_default() += 1;
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
            ..Self::default()
        };
        ctx.run_topo(graph);
        ctx.resolve_product_contexts();
        ctx.resolve_sdr_product_geometry();
        ctx.ensure_product_ref_frames();
        ctx.resolve_nauo_instances();
        ctx.finalize_assembly();
        ctx.resolve_sdr_links();
        ctx.resolve_pdr_links();
        let header = header::extract_file_header(&graph.header, &mut ctx.warnings);
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
                units: ctx.units,
                unitless_contexts: ctx.unitless_contexts,
                geometric_item_specific_usages: ctx.geometric_item_specific_usages,
                assembly: ctx.assembly,
                schema: graph.schema.clone(),
                header,
                visualization: ctx.visualization,
                properties: ctx.properties,
                pmi: ctx.pmi,
                shape_aspects: ctx.shape_aspects,
                composite_group_shape_aspects: ctx.composite_group_shape_aspects,
                centre_of_symmetries: ctx.centre_of_symmetries,
                all_around_shape_aspects: ctx.all_around_shape_aspects,
                datum_systems: ctx.datum_systems,
                tolerance_zones: ctx.tolerance_zones,
                datum_targets: ctx.datum_targets,
                placed_datum_target_features: ctx.placed_datum_target_features,
                shape_aspect_relationships: ctx.shape_aspect_relationships,
                plm: ctx.plm,
                units_pool: build_units_pool(
                    ctx.named_units_arena,
                    ctx.mwu_arena,
                    ctx.due_arena,
                    ctx.derived_unit_arena,
                    ctx.dimensional_exponents,
                ),
                form_features: ctx.form_features,
                representations: ctx.representations,
                representation_items: ctx.representation_items,
                characterized_objects: ctx.characterized_objects,
                representation_maps: ctx.representation_maps,
                mapped_items: ctx.mapped_items,
                numeric_representation_items: ctx.numeric_representation_items,
                tessellated_items: ctx.tessellated_items,
                tessellated_faces: ctx.tessellated_faces,
                tessellated_surface_sets: ctx.tessellated_surface_sets,
                geometric_representation_items: ctx.geometric_representation_items,
                representation_relationships: ctx.representation_relationships,
                compound_representation_items: ctx.compound_representation_items,
                item_identified_representation_usages: ctx.item_identified_representation_usages,
            },
            warnings: ctx.warnings,
            parse_warnings: graph.warnings.clone(),
        }
    }

    /// Resolve `product_pc_step_refs` / `product_pdc_step_refs` (raw STEP
    /// entity ids captured by the `PRODUCT` / `PRODUCT_DEFINITION` handlers) into
    /// typed `ProductContextId` / `ProductDefinitionContextId` and write them
    /// back onto each `Product`. Run after the assembly-context handlers
    /// populate the id maps.
    fn resolve_product_contexts(&mut self) {
        for (pid, pc_step_id) in &self.product_pc_step_refs {
            if let Some(&pcid) = self.product_context_id_map.get(pc_step_id) {
                self.assembly_products[*pid].product_context = Some(pcid);
            }
        }
        for (pid, pdc_step_id) in &self.product_pdc_step_refs {
            if let Some(&pdcid) = self.product_definition_context_id_map.get(pdc_step_id) {
                self.assembly_products[*pid].pdef_context = Some(pdcid);
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
    /// instances. Order-independent: identical result under either dispatch
    /// strategy. A NAUO with no transform warns and drops, as before.
    fn resolve_nauo_instances(&mut self) {
        for pending in std::mem::take(&mut self.pending_nauo_instances) {
            let Some(&transform) = self.nauo_transform_map.get(&pending.nauo_id) else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id: pending.nauo_id,
                    detail: String::from("NEXT_ASSEMBLY_USAGE_OCCURRENCE with no transform found"),
                });
                continue;
            };
            self.assembly_products[pending.parent]
                .instances
                .push(crate::ir::assembly::Instance {
                    child: pending.child,
                    transform,
                    occurrence_id: pending.occurrence_id,
                    occurrence_name: pending.occurrence_name,
                });
        }
    }

    /// Wrap the collected products into an `AssemblyTree` if any PRODUCT
    /// entities were seen. `roots` lists every top-level product (a forest
    /// for multi-part files).
    fn finalize_assembly(&mut self) {
        if self.product_arena_map.is_empty() {
            return;
        }
        // Collect every ProductId that appears as an Instance.child. The
        // remaining products are root candidates.
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
        });
    }

    /// Resolve the raw `SHAPE_DEFINITION_REPRESENTATION` links stashed by the
    /// SDR handler (definition = a `PROPERTY_DEFINITION`). Runs after all entities
    /// are read, so `property_def_step_to_id` and `repr_id_map` are
    /// complete. Unresolved links (PD or representation not modelled) drop —
    /// FAIL-safe.
    fn resolve_sdr_links(&mut self) {
        if self.sdr_link_refs.is_empty() {
            return;
        }
        let mut links = Vec::new();
        for (def_ref, rep_ref) in std::mem::take(&mut self.sdr_link_refs) {
            let effective = self.srr_equiv_map.get(&rep_ref).copied().unwrap_or(rep_ref);
            if let (Some(&definition), Some(&used_representation)) = (
                self.property_def_step_to_id.get(&def_ref),
                self.repr_id_map.get(&effective),
            ) {
                links.push(crate::ir::property::ShapeDefinitionRepresentationLink {
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
            if let (Some(&definition), Some(&used_representation)) = (
                self.property_def_step_to_id.get(&def_ref),
                self.repr_id_map.get(&effective),
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
        if let Some(&id) = self.unitless_context_id_map.get(&from) {
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
        resolve_in_map(&self.point_map, from, to, field_name)
    }

    pub(crate) fn resolve_direction(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<DirectionId, ConvertError> {
        resolve_in_map(&self.direction_map, from, to, field_name)
    }

    pub(crate) fn resolve_curve(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<CurveId, ConvertError> {
        resolve_in_map(&self.curve_map, from, to, field_name)
    }

    pub(crate) fn resolve_surface(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<SurfaceId, ConvertError> {
        resolve_in_map(&self.surface_map, from, to, field_name)
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
        resolve_in_map(&self.edge_map, from, to, field_name)
    }

    pub(crate) fn resolve_face_bound(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<WireId, ConvertError> {
        resolve_in_map(&self.face_bound_map, from, to, field_name)
    }

    pub(crate) fn resolve_face(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<FaceId, ConvertError> {
        resolve_in_map(&self.face_map, from, to, field_name)
    }

    pub(crate) fn resolve_shell(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<ShellId, ConvertError> {
        resolve_in_map(&self.shell_map, from, to, field_name)
    }

    /// Two-step lookup `PRODUCT_DEFINITION #N → PRODUCT #N → ProductId`
    /// shared by the NAUO tree-wiring handlers.
    pub(crate) fn resolve_product_by_pdef(
        &self,
        from: u64,
        pdef_ref: u64,
        field_name: &'static str,
    ) -> Result<ProductId, ConvertError> {
        let product_step_id =
            self.pdef_to_product
                .get(&pdef_ref)
                .copied()
                .ok_or(ConvertError::MissingReference {
                    from,
                    to: pdef_ref,
                    field_name,
                })?;
        self.product_arena_map.get(&product_step_id).copied().ok_or(
            ConvertError::MissingReference {
                from,
                to: product_step_id,
                field_name,
            },
        )
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
        resolve_in_map(&self.axis1_map, from, to, field_name)
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
