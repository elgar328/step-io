//! Converts a raw [`EntityGraph`] into a typed [`StepModel`].
//!
//! This module is the boundary between the parser layer and the IR layer.
//! It uses multi-pass eager conversion: entities are processed in dependency
//! order so that referenced objects are always available when needed.

use std::collections::{HashMap, HashSet};

use crate::ir::arena::Arena;
use crate::ir::assembly::{AssemblyTree, Product, Transform3d, WireframeContent};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Pcurve, TransitionCode};
use crate::ir::id::{
    ColourId, Curve2dId, CurveFontId, CurveId, CurveStyleId, Direction2dId, DirectionId, EdgeId,
    FaceId, FoundedItemId, Placement1dId, Placement2dId, Placement3dId, Point2dId, PointId,
    PresentationStyleAssignmentId, ProductId, ShellId, SolidId, StyledItemId, SurfaceId,
    SurfaceStyleRenderingId, UnitContextId, VertexId, WireId,
};
use crate::ir::model::{GeometryPool, StepModel, TopologyPool};
use crate::ir::shape_rep::{AngleUnit, LengthUncertainty, LengthUnit, SolidAngleUnit, UnitContext};
use crate::ir::topology::{Orientation, OrientedEdge};
use crate::ir::visualization::{FillAreaStyleColour, VisualizationPool};
// CurveFontId / CurveStyleId / StyledItemId imported above; the map types reference them directly.
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

mod geometry;
mod header;
mod passes;

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
    /// Unit / uncertainty contexts accumulated during Pass 0-2 — one entry
    /// per `REPRESENTATION_CONTEXT` complex entity in the source file.
    pub(crate) units: Arena<UnitContext>,
    /// `REPRESENTATION_CONTEXT #N → UnitContextId` populated by Pass 0-2.
    /// Used by representation converters (ABSR, MSSR, plain SR, GBWSR, GBSSR,
    /// MDGPR) to translate their `context_of_items` ref into an `UnitContextId`.
    pub(crate) context_id_map: HashMap<u64, UnitContextId>,
    /// `representation #N → UnitContextId` for the 5 product-bearing
    /// representation entities (ABSR / MSSR / plain SR / GBWSR / GBSSR).
    /// Single map suffices because STEP entity ids are globally unique within
    /// a file. The SDR pass reads this map to attach `Product.geometry_context`.
    /// MDGPR resolves directly into `Mdgpr.context` and does not use this map.
    pub(crate) repr_context_map: HashMap<u64, UnitContextId>,

    /// `true` if any `CONVERSION_BASED_UNIT` whose `name` matched an SI
    /// length spelling (`'METRE'` / `'MILLIMETRE'` / `'CENTIMETRE'`) was
    /// processed. Surfaces into `UnitContext.length_cbu_wrapped` so the
    /// writer reproduces the wrapper. Reset per `convert()` call via
    /// `Default`.
    pub(crate) length_cbu_wrapped: bool,
    /// Same idea as `length_cbu_wrapped` but for plane-angle units —
    /// `'RADIAN'` self-wrap. Surfaces into
    /// `UnitContext.plane_angle_cbu_wrapped`.
    pub(crate) plane_angle_cbu_wrapped: bool,
    /// `true` once any plain SI unit complex (no `CONVERSION_BASED_UNIT`
    /// part) was observed with an explicit `DIMENSIONAL_EXPONENTS` entity
    /// ref in its `NAMED_UNIT.dimensions` slot — the ABC-tier convention.
    /// Sticky cumulative: a single explicit observation locks the flag,
    /// every subsequently built `UnitContext` carries the same value.
    pub(crate) dim_exp_explicit: bool,

    /// Entity ids inside any `DEFINITIONAL_REPRESENTATION` subtree (PCURVE
    /// parametric-space geometry). 3D passes skip them so their 2D
    /// `CARTESIAN_POINT` / `DIRECTION` / `LINE` / … don't collide with 3D
    /// conversion. Pass 4a then walks the same set to populate the 2D
    /// arenas (`points_2d`, `directions_2d`, `curves_2d`).
    pub(crate) pcurve_subtree_ids: HashSet<u64>,

    // Unit entity maps: STEP #N → resolved unit variant.
    pub(crate) length_unit_map: HashMap<u64, LengthUnit>,
    pub(crate) angle_unit_map: HashMap<u64, AngleUnit>,
    pub(crate) solid_angle_unit_map: HashMap<u64, SolidAngleUnit>,
    /// `UNCERTAINTY_MEASURE_WITH_UNIT #N → value+metadata` for uncertainty
    /// entities whose `unit_component` resolved to a length unit. Populated
    /// between Pass 0-1 (unit leaves) and Pass 0-2 (context assembly).
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

    // 2D geometry (PCURVE parametric space) maps.
    pub(crate) point_2d_map: HashMap<u64, Point2dId>,
    pub(crate) direction_2d_map: HashMap<u64, Direction2dId>,
    pub(crate) curve_2d_map: HashMap<u64, Curve2dId>,
    pub(crate) vector_2d_map: HashMap<u64, (Direction2dId, f64)>,
    pub(crate) placement_2d_map: HashMap<u64, Placement2dId>,
    /// `SURFACE_CURVE / SEAM_CURVE #N → Vec<Pcurve>`. Populated during
    /// Pass 4-3 and consumed by `convert_edge_curve` to attach pcurves to
    /// each edge.
    pub(crate) surface_curve_pcurves_map: HashMap<u64, Vec<Pcurve>>,

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
    /// wrapper orientation)`. Populated by Pass 5-7b, consumed by
    /// `convert_brep_with_voids` in Pass 5-8.
    pub(crate) oriented_closed_shell_map: HashMap<u64, (ShellId, Orientation)>,

    // Assembly (Phase A): product arena + lookup maps populated by Pass 6.
    // `assembly` is filled in `convert()` after Pass 6 if any PRODUCT was seen.
    pub(crate) assembly: Option<AssemblyTree>,
    pub(crate) assembly_products: Arena<Product>,
    pub(crate) product_contexts: Arena<crate::ir::ProductContext>,
    pub(crate) product_definition_contexts: Arena<crate::ir::ProductDefinitionContext>,
    pub(crate) product_context_id_map: HashMap<u64, crate::ir::ProductContextId>,
    pub(crate) product_definition_context_id_map:
        HashMap<u64, crate::ir::ProductDefinitionContextId>,
    /// `ProductId → raw STEP entity id of the product's first PC`
    /// (`PRODUCT.frame_of_reference[0]`). `resolve_product_contexts`
    /// converts these refs to `ProductContextId` after `Pass9AssemblyContext`.
    pub(crate) product_pc_step_refs: HashMap<ProductId, u64>,
    /// Same pattern, populated by `PRODUCT_DEFINITION.frame_of_reference`.
    pub(crate) product_pdc_step_refs: HashMap<ProductId, u64>,
    pub(crate) product_definition_context_roles: Arena<crate::ir::ProductDefinitionContextRole>,
    pub(crate) product_definition_context_associations:
        Arena<crate::ir::ProductDefinitionContextAssociation>,
    pub(crate) pdc_role_id_map: HashMap<u64, crate::ir::ProductDefinitionContextRoleId>,
    pub(crate) pdca_id_map: HashMap<u64, crate::ir::ProductDefinitionContextAssociationId>,
    pub(crate) product_arena_map: HashMap<u64, ProductId>,
    pub(crate) formation_to_product: HashMap<u64, u64>,
    pub(crate) pdef_to_product: HashMap<u64, u64>,
    pub(crate) absr_solid_map: HashMap<u64, Vec<SolidId>>,
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION #N → Placement3dId` for the first
    /// `AXIS2_PLACEMENT_3D` item in the ABSR's `items` list (its coordinate
    /// reference frame). Consumed by SDR conversion to populate
    /// `Product.shape_ref_frame`.
    pub(crate) absr_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `SHELL_BASED_SURFACE_MODEL #N → resolved shell ids`. Populated in
    /// Pass 5-8b and consumed by MSSR conversion to flatten shells.
    pub(crate) sbsm_shells_map: HashMap<u64, Vec<ShellId>>,
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
    /// during Pass 6-4 so SDR conversion can attach the plain SR's reference
    /// frame to `Product.outer_sr_frame` when the indirection chain is taken.
    pub(crate) plain_sr_frame_map: HashMap<u64, Placement3dId>,
    /// `COMPOSITE_CURVE_SEGMENT #N → (transition, same_sense, parent_curve
    /// step id)`. Populated by Pass 4 immediately after `TRIMMED_CURVE` so
    /// `COMPOSITE_CURVE` conversion can resolve segments by entity ref.
    pub(crate) composite_segment_map: HashMap<u64, (TransitionCode, bool, u64)>,
    /// `PRODUCT_CATEGORY #N → (name, description)`. Populated by Pass 6-1b
    /// sub-a so the PCR pass can resolve the supertype side of the chain.
    pub(crate) pc_meta_map: HashMap<u64, (String, Option<String>)>,
    /// `PRODUCT_RELATED_PRODUCT_CATEGORY #N → (name, description, products
    /// list)`. Populated by Pass 6-1b sub-a so the PCR pass can iterate the
    /// PRPC's product refs and attach the PC root to each.
    pub(crate) prpc_meta_map: HashMap<u64, (String, Option<String>, Vec<u64>)>,
    /// `GEOMETRIC_(CURVE_)SET #N → (curves, loose points)`. Populated in
    /// Pass 6-4 just before the GBWSR / GBSSR converters consume it.
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
    /// Populated before Pass 6-5.
    pub(crate) pdef_shape_to_pdef: HashMap<u64, u64>,
    /// `PRODUCT_DEFINITION_SHAPE #N → NEXT_ASSEMBLY_USAGE_OCCURRENCE #N` when
    /// the `pdef_shape` points at a `NAUO` (instance-tagged). Populated
    /// alongside `pdef_shape_to_pdef` and consumed by Pass 6-7.
    pub(crate) pdef_shape_to_nauo: HashMap<u64, u64>,
    pub(crate) transform_map: HashMap<u64, Transform3d>,
    pub(crate) nauo_transform_map: HashMap<u64, Transform3d>,

    /// Lazily-built `VisualizationPool` — Pass 7 's MDGPR convert pushes
    /// `Mdgpr` records here. `None` if no visualization entities were seen.
    pub(crate) visualization: Option<VisualizationPool>,
    /// `COLOUR_RGB` / `DRAUGHTING_PRE_DEFINED_COLOUR` step entity id →
    /// `ColourId`. Both colour-family readers push into the
    /// `Arena<Colour>` on `ctx.visualization` and record the resulting id
    /// here so downstream consumers (`FILL_AREA_STYLE_COLOUR`,
    /// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) can resolve a colour ref
    /// to an arena index without copying the colour data.
    pub(crate) viz_colour_id_map: HashMap<u64, ColourId>,
    /// `DRAUGHTING_PRE_DEFINED_CURVE_FONT` step entity id → `CurveFontId`.
    /// Populated by the font leaf reader during Pass 7-1; consumed by the
    /// `CURVE_STYLE` reader to resolve a font ref to an arena index.
    pub(crate) viz_curve_font_id_map: HashMap<u64, CurveFontId>,
    /// `CURVE_STYLE` step entity id → `CurveStyleId`. Populated during
    /// `Pass7CurveStyle`; consumed by the PSA reader to dispatch styling
    /// list entries to the `PsaStyle::Curve(...)` variant.
    pub(crate) viz_curve_style_id_map: HashMap<u64, CurveStyleId>,
    /// `FILL_AREA_STYLE_COLOUR #N → FillAreaStyleColour` (Pass 7-2).
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
    /// `SURFACE_STYLE_TRANSPARENT #N → transparency value` (Pass 7-3b).
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
    /// `PRESENTATION_STYLE_ASSIGNMENT` step entity id →
    /// `PresentationStyleAssignmentId`. Populated by the PSA handler after
    /// pushing the resolved variant into
    /// `VisualizationPool::presentation_style_assignments`; consumed by the
    /// `STYLED_ITEM` / `OVER_RIDING_STYLED_ITEM` readers to convert their
    /// `styles` ref lists into arena ids.
    pub(crate) viz_psa_id_map: HashMap<u64, PresentationStyleAssignmentId>,
    /// `STYLED_ITEM` step entity id → `StyledItemId`. Populated by the
    /// `Pass7StyledItem` reader after pushing the resolved `StyledItem`
    /// variant into `VisualizationPool::styled_items`. Consumed by the
    /// MDGPR reader to convert its `items` list into arena references.
    pub(crate) viz_styled_item_id_map: HashMap<u64, StyledItemId>,

    /// Lazily-built property pool — populated by Pass 8's PDR converter.
    /// `None` if the source had no `PROPERTY_DEFINITION_REPRESENTATION`.
    pub(crate) properties: Option<crate::ir::property::PropertyPool>,
    /// `MEASURE_REPRESENTATION_ITEM #N → PropertyMeasure` (Pass 8-1).
    /// Items with unsupported `MeasureKind` (e.g. `AREA_MEASURE`) are
    /// silently skipped — no map entry. Temp; discarded after Pass 8.
    pub(crate) measure_item_map: HashMap<u64, crate::ir::property::PropertyMeasure>,
    /// `PROPERTY_DEFINITION #N → (name, description, target ProductId)`
    /// (Pass 8-3). PDs whose target ref does not resolve to a Product
    /// (e.g. `SHAPE_ASPECT`) are silently dropped — no map entry.
    pub(crate) property_def_map: HashMap<u64, (String, Option<String>, ProductId)>,

    /// `SHAPE_ASPECT` arena — Pass 8 's `SHAPE_ASPECT` convert pushes
    /// here. Empty when no PMI entities were seen.
    pub(crate) shape_aspects: crate::ir::Arena<crate::ir::ShapeAspect>,

    /// Lazily-built plm pool — populated by the Pass 9 plm reader chain
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

    pub(crate) warnings: Vec<ConvertError>,
}

impl ReaderContext {
    /// Convert an entire [`EntityGraph`] into a [`StepModel`].
    ///
    /// Entities are processed in dependency order across multiple passes.
    /// Unrecognised entities are silently skipped — only entities that the
    /// reader *attempts* to convert but fails produce warnings.
    #[must_use]
    pub fn convert(graph: &EntityGraph) -> ConvertResult {
        let mut ctx = Self {
            pcurve_subtree_ids: collect_pcurve_subtree_ids(graph),
            ..Self::default()
        };
        ctx.run_unit_pass(graph);
        ctx.run_geometry_passes(graph);
        ctx.run_topology_passes(graph);
        ctx.run_assembly_passes(graph);
        ctx.resolve_product_contexts();
        ctx.finalize_assembly();
        let header = header::extract_file_header(&graph.header, &mut ctx.warnings);
        ConvertResult {
            model: StepModel {
                geometry: ctx.geometry,
                topology: ctx.topology,
                units: ctx.units,
                assembly: ctx.assembly,
                schema: graph.schema.clone(),
                header,
                visualization: ctx.visualization,
                properties: ctx.properties,
                shape_aspects: ctx.shape_aspects,
                plm: ctx.plm,
            },
            warnings: ctx.warnings,
            parse_warnings: graph.warnings.clone(),
        }
    }

    /// Resolve `product_pc_step_refs` / `product_pdc_step_refs` (raw STEP
    /// entity ids captured at `Pass6Product` / `Pass6Pdef`) into typed
    /// `ProductContextId` / `ProductDefinitionContextId` and write them
    /// back onto each `Product`. Run after `Pass9AssemblyContext`
    /// (which populates the id maps).
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

    /// Wrap the collected products into an `AssemblyTree` if any PRODUCT
    /// entities were seen. `root` stays `None` in Phase A; Phase B fills it.
    fn finalize_assembly(&mut self) {
        if self.product_arena_map.is_empty() {
            return;
        }
        // Collect every ProductId that appears as an Instance.child. The
        // remaining products are root candidates.
        let mut is_child: HashSet<ProductId> = HashSet::new();
        for product in self.assembly_products.iter() {
            if let crate::ir::assembly::ProductContent::Group(group) = &product.content {
                for inst in &group.instances {
                    is_child.insert(inst.child);
                }
            }
        }
        let roots: Vec<ProductId> = self
            .assembly_products
            .iter_ids()
            .filter(|pid| !is_child.contains(pid))
            .collect();
        let root = match roots.as_slice() {
            [single] => Some(*single),
            [] => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id: 0,
                    detail: String::from(
                        "assembly has no root candidate (every product appears as an instance child)",
                    ),
                });
                // Fallback: first product.
                Some(ProductId(0))
            }
            [first, ..] => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id: 0,
                    detail: format!(
                        "assembly has {} root candidates, using the first",
                        roots.len()
                    ),
                });
                Some(*first)
            }
        };
        let products = std::mem::take(&mut self.assembly_products);
        let product_contexts = std::mem::take(&mut self.product_contexts);
        let product_definition_contexts = std::mem::take(&mut self.product_definition_contexts);
        let product_definition_context_roles =
            std::mem::take(&mut self.product_definition_context_roles);
        let product_definition_context_associations =
            std::mem::take(&mut self.product_definition_context_associations);
        self.assembly = Some(AssemblyTree {
            products,
            root,
            product_contexts,
            product_definition_contexts,
            product_definition_context_roles,
            product_definition_context_associations,
        });
    }

    // ---------------------------------------------------------------------
    // Resolver helpers: look up a STEP entity id in one of the internal
    // maps and return the stored IR id (or a `MissingReference` error).
    // Each converter can collapse four lines of boilerplate into one call.
    // ---------------------------------------------------------------------

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
    /// shared by Pass 6-8 (NAUO tree wiring).
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

/// Check whether a complex entity contains all required parts.
pub(crate) fn has_all_parts(parts: &[RawEntityPart], required: &[&str]) -> bool {
    required
        .iter()
        .all(|name| parts.iter().any(|p| p.name == *name))
}

/// Collect entity ids that belong to any `DEFINITIONAL_REPRESENTATION`
/// subtree. These represent PCURVE parametric-space geometry (2D points,
/// 2D curves, `AXIS2_PLACEMENT_2D`, `PARAMETRIC_REPRESENTATION_CONTEXT`).
/// 3D passes skip them so their 2D `CARTESIAN_POINT` / `DIRECTION` / etc.
/// don't collide with the 3D converters; Pass 4a then walks the same set
/// to populate the 2D arenas.
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
