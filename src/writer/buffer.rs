//! IR → `Vec<WriterEntity>` conversion.
//!
//! `WriteBuffer` assigns `#N` ids and assembles `WriterEntity` values as it
//! walks the IR. Each concern lives in its own sub-module:
//!
//! - [`geometry`] — points, directions, placements, curves, surfaces
//! - [`topology`] — vertices, edges, wires, faces, shells, solids
//! - [`units`] — `SI_UNIT`-based unit context
//! - [`assembly`] — single-part PRODUCT chain + ABSR + SDR

use std::collections::HashMap;

use super::WriteError;
use super::entity::WriterEntity;
use crate::ir::{
    Curve2dId, CurveId, Direction2dId, DirectionId, EdgeId, FaceId, Placement1dId, Placement2dId,
    Placement3dId, Point2dId, PointId, ProductId, ShellId, SolidId, StepModel, SurfaceId, VertexId,
    WireId,
};

pub(crate) mod assembly;
pub(crate) mod form_features;
pub(crate) mod geometry;
pub(crate) mod mapped_item;
pub(crate) mod numeric_representation_item;
pub(crate) mod plm;
pub(crate) mod pmi;
pub(crate) mod property;
pub(crate) mod tessellation;
pub(crate) mod topology;
pub(crate) mod units;
pub(crate) mod visualization;

pub(crate) struct WriteBuffer<'m> {
    pub(crate) model: &'m StepModel,
    pub(crate) next_id: u64,
    pub(crate) entities: Vec<WriterEntity>,
    pub(crate) point_ids: HashMap<PointId, u64>,
    pub(crate) direction_ids: HashMap<DirectionId, u64>,
    pub(crate) placement_ids: HashMap<Placement3dId, u64>,
    pub(crate) placement_1d_ids: HashMap<Placement1dId, u64>,
    pub(crate) placement_2d_ids: HashMap<Placement2dId, u64>,
    pub(crate) curve_ids: HashMap<CurveId, u64>,
    pub(crate) surface_ids: HashMap<SurfaceId, u64>,
    pub(crate) vertex_ids: HashMap<VertexId, u64>,
    pub(crate) edge_ids: HashMap<EdgeId, u64>,
    pub(crate) wire_ids: HashMap<WireId, u64>,
    pub(crate) face_ids: HashMap<FaceId, u64>,
    pub(crate) shell_ids: HashMap<ShellId, u64>,
    pub(crate) solid_ids: HashMap<SolidId, u64>,
    pub(crate) point_2d_ids: HashMap<Point2dId, u64>,
    pub(crate) direction_2d_ids: HashMap<Direction2dId, u64>,
    pub(crate) curve_2d_ids: HashMap<Curve2dId, u64>,
    /// Emitted `PLANAR_EXTENT` / `PLANAR_BOX` step ids, indexed by
    /// `PlanarExtentId` — keeps `emit_planar_extent` idempotent so the
    /// standalone arena loop and a `VIEW_VOLUME`'s `view_window` ref emit
    /// the entity exactly once.
    pub(crate) planar_extent_ids: HashMap<crate::ir::PlanarExtentId, u64>,
    /// STEP entity ids of every emitted `REPRESENTATION_CONTEXT` complex
    /// entity, indexed by `UnitContextId.0`. Populated up-front in `emit_all`
    /// so every representation emitter can resolve its `Option<UnitContextId>`
    /// to a cached id.
    pub(crate) unit_context_ids: Vec<u64>,
    /// STEP entity id of every emitted representation (`ABSR` / `MSSR` /
    /// plain `SR` / `GBWSR` / `GBSSR` / `MDGPR`), indexed by
    /// `RepresentationId.0`. Geometry representations are filled by
    /// `emit_representations_pre_pass` in `representations` arena order
    /// before the product chain runs; `MDGPR` slots are appended by
    /// `emit_visualization_if_set`. The product chain resolves each
    /// product's `representation_id` / `outer_representation_id` to a
    /// cached id instead of re-emitting the representation inline. Empty
    /// for hand/kernel-built IR (the arena is reader-populated only).
    pub(crate) representation_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `REPRESENTATION_MAP`, indexed by
    /// `RepresentationMapId.0`. Populated by `emit_mapped_items` before the
    /// `MAPPED_ITEM` loop so each item resolves its `mapping_source` ref.
    pub(crate) representation_map_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `COORDINATES_LIST`, indexed by
    /// `TessellatedItemId.0`. Populated by `emit_tessellation` before the
    /// `COMPLEX_TRIANGULATED_FACE` loop so each face resolves its
    /// `coordinates` ref.
    pub(crate) tessellated_item_step_ids: Vec<u64>,
    /// Emitted `COMPLEX_TRIANGULATED_FACE` / `COMPLEX_TRIANGULATED_SURFACE_SET`
    /// step ids, indexed by `TessellatedFaceId.0` / `TessellatedSurfaceSetId.0`.
    /// Populated by `emit_tessellation` so a `TESSELLATED_GEOMETRIC_SET`
    /// child resolves through `emit_tessellated_item_ref`.
    pub(crate) tessellated_face_step_ids: Vec<u64>,
    pub(crate) tessellated_surface_set_step_ids: Vec<u64>,
    /// Emitted `LEADER_CURVE` step ids, indexed by
    /// `AnnotationCurveOccurrenceId.0`. Populated by
    /// `emit_annotation_curve_occurrences` before `emit_annotation_occurrences`
    /// so `TERMINATOR_SYMBOL` / `LEADER_TERMINATOR` can resolve their
    /// `annotated_curve` back-reference.
    pub(crate) acoc_step_ids: Vec<u64>,
    /// Emitted `annotation_occurrence` enum step ids, indexed by
    /// `AnnotationOccurrenceId.0`. Populated by
    /// `emit_annotation_occurrences` (one push per enum entry, in arena
    /// order) before `emit_draughting_callouts` so its `contents` SELECT
    /// members resolve.
    pub(crate) ao_step_ids: Vec<u64>,
    /// Emitted `draughting_callout` step ids, indexed by
    /// `DraughtingCalloutId.0`. Populated by `emit_draughting_callouts`
    /// before `emit_draughting_callout_relationships`.
    pub(crate) draughting_callout_step_ids: Vec<u64>,
    /// Emitted `TOLERANCE_ZONE` step ids, indexed by `ToleranceZoneId.0`.
    /// Populated by `emit_tolerance_zones` so
    /// `ProjectedZoneDefinitionHandler::write` can resolve `zone`.
    pub(crate) tolerance_zone_step_ids: Vec<u64>,
    /// Emitted `TYPE_QUALIFIER` step ids, indexed by `TypeQualifierId.0`.
    /// Populated by `emit_pmi_if_set` so `MeasureQualificationHandler::write`
    /// can resolve a `qualifiers` SET member.
    pub(crate) type_qualifier_step_ids: Vec<u64>,
    /// Emitted `VALUE_FORMAT_TYPE_QUALIFIER` step ids, indexed by
    /// `ValueFormatTypeQualifierId.0`. Same role as
    /// `type_qualifier_step_ids`.
    pub(crate) value_format_type_qualifier_step_ids: Vec<u64>,
    /// Emitted `representation_item` arena step ids (phase
    /// repr-item-arena-1), indexed by `RepresentationItemId.0`. Consumed
    /// by `emit_representation_item_ref` 의 11번째 variant.
    pub(crate) representation_item_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `NAMED_UNIT` complex from
    /// [`crate::ir::UnitsPool::named_units`], indexed by `NamedUnitId.0`.
    /// Populated by `emit_units_pool_if_set` before GUAC + MWU + DUE emit,
    /// so every consumer (including the GUAC writer that resolves
    /// `UnitContext.{length, plane_angle, solid_angle}`) resolves its
    /// `NamedUnitId` ref with a single index lookup.
    pub(crate) named_unit_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `MEASURE_WITH_UNIT` subtype, indexed
    /// by `MeasureWithUnitId.0`. Currently unused by downstream emitters
    /// (no entity in this phase consumes an MWU ref through the units
    /// pool — the existing `UMU` / `PROPERTY_DEFINITION_REPRESENTATION`
    /// paths predate units-1 and route through the legacy ctx caches).
    /// The vec is populated for parity with the other arenas and to
    /// support post-units-1 consumers (e.g. MFUO `quantity`).
    pub(crate) mwu_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `DERIVED_UNIT_ELEMENT`, indexed by
    /// `DerivedUnitElementId.0`. Consumed by `DERIVED_UNIT` emission
    /// (units-1b) to resolve `elements` refs.
    pub(crate) due_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `DERIVED_UNIT`, indexed by
    /// `DerivedUnitId.0`. Currently unused by downstream emitters; kept
    /// for parity with the other units-pool caches and for any future
    /// consumer that wires a `ref_derived_unit` field.
    pub(crate) derived_unit_step_ids: Vec<u64>,
    /// Lazily emitted `DIMENSIONAL_EXPONENTS(1, 0, ...)` step id, shared by
    /// every length-flavour CBU outer in this file. `None` until the first
    /// length CBU is emitted (units-3c dedup).
    pub(crate) length_dim_exp_step: Option<u64>,
    /// Lazily emitted `DIMENSIONAL_EXPONENTS(0, 0, ...)` step id, shared by
    /// every dimensionless emitter (plane-angle / solid-angle CBU outers,
    /// area / volume DUE consumers).
    pub(crate) dimensionless_dim_exp_step: Option<u64>,
    /// Lazily emitted `DIMENSIONAL_EXPONENTS(0, 0, 1, ...)` step id, shared
    /// by every mass CBU outer.
    pub(crate) mass_dim_exp_step: Option<u64>,
    /// STEP entity id of every emitted `COLOUR_RGB` /
    /// `DRAUGHTING_PRE_DEFINED_COLOUR` entity, indexed by `ColourId.0`.
    /// Populated by `emit_visualization_if_set` before any consumer
    /// (`FILL_AREA_STYLE_COLOUR`, `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`)
    /// needs to resolve a colour ref.
    pub(crate) colour_step_ids: Vec<u64>,
    /// Emitted `SYMBOL_COLOUR` step ids, indexed by `SymbolColourId.0`
    /// (phase symbol-colour). Future `SYMBOL_STYLE` writers consume this.
    pub(crate) symbol_colour_step_ids: Vec<u64>,
    /// Emitted `TEXT_STYLE_FOR_DEFINED_FONT` step ids, indexed by
    /// `TextStyleForDefinedFontId.0` (phase text-style-font). Future
    /// `text_style.character_appearance` SELECT writers consume this.
    pub(crate) text_style_for_defined_font_step_ids: Vec<u64>,
    /// Emitted `PRE_DEFINED_MARKER` step ids (phase pre-defined-marker),
    /// indexed by `PreDefinedMarkerId.0`.
    pub(crate) pre_defined_marker_step_ids: Vec<u64>,
    /// Emitted `text_style` enum-arena step ids (phase text-style-box),
    /// indexed by `TextStyleId.0`. Reserved for future consumers
    /// (`presentation_style_select` SELECT) — no step-io entity references
    /// this cache today.
    pub(crate) text_style_step_ids: Vec<u64>,
    /// Emitted `DRAUGHTING_PRE_DEFINED_TEXT_FONT` step ids (phase
    /// text-literal — cache added retroactively for the `text_literal.font`
    /// SELECT). Indexed by `DraughtingPreDefinedTextFontId.0`.
    pub(crate) dptf_step_ids: Vec<u64>,
    /// Emitted `TEXT_LITERAL` step ids (phase text-literal), indexed by
    /// `TextLiteralId.0`. Consumed by the `COMPOSITE_TEXT` emitter for the
    /// `text_or_character` SELECT.
    pub(crate) text_literal_step_ids: Vec<u64>,
    /// Emitted `DRAUGHTING_MODEL_ITEM_ASSOCIATION` step ids (phase dmia),
    /// indexed by `DraughtingModelItemAssociationId.0`. No other entity
    /// currently references the cache; retained for symmetry.
    pub(crate) dmia_step_ids: Vec<u64>,
    /// Emitted `(GRC PRC REP_CONTEXT)` complex step ids (phase
    /// unitless-context), indexed by `UnitlessContextId.0`. Consumed by
    /// `repr_context_attr` when a representation's `context_of_items`
    /// resolves to a `RepresentationContextRef::Unitless`.
    pub(crate) unitless_context_step_ids: Vec<u64>,
    /// Emitted `GEOMETRIC_ITEM_SPECIFIC_USAGE` step ids (phase gisu),
    /// indexed by `GeometricItemSpecificUsageId.0`. No other entity
    /// currently references the cache; retained for symmetry with DMIA.
    pub(crate) gisu_step_ids: Vec<u64>,
    /// Emitted `INVISIBILITY` step ids (phase invisibility), indexed by
    /// `InvisibilityId.0`. Retained for symmetry — no other entity
    /// references this cache.
    pub(crate) invisibility_step_ids: Vec<u64>,
    /// Emitted `PRESENTATION_VIEW` / `PRESENTATION_AREA` step ids (phase
    /// pr-core), indexed by `PresentationRepresentationId.0`. Consumed by
    /// `AREA_IN_SET` and `PRESENTATION_SIZE` emitters in subsequent
    /// sub-phases.
    pub(crate) presentation_representation_step_ids: Vec<u64>,
    /// Emitted `PRESENTATION_SET` step ids (phase pr-core), indexed by
    /// `PresentationSetId.0`. Consumed by `AREA_IN_SET.in_set`.
    pub(crate) presentation_set_step_ids: Vec<u64>,
    /// Emitted `AREA_IN_SET` step ids (phase pr-size), indexed by
    /// `AreaInSetId.0`. Consumed by `PRESENTATION_SIZE.unit` SELECT.
    pub(crate) area_in_set_step_ids: Vec<u64>,
    /// STEP entity id of every emitted curve-font entity
    /// (`PRE_DEFINED_CURVE_FONT` / `DRAUGHTING_PRE_DEFINED_CURVE_FONT`),
    /// indexed by `PreDefinedCurveFontId.0`. Consumed by the `CURVE_STYLE`
    /// writer.
    pub(crate) pre_defined_curve_font_step_ids: Vec<u64>,
    /// STEP entity id of every emitted pre-defined-symbol entity
    /// (`PRE_DEFINED_SYMBOL` / `PRE_DEFINED_TERMINATOR_SYMBOL`), indexed by
    /// `PreDefinedSymbolId.0`. No step-io consumer reads this cache yet —
    /// it exists for round-trip symmetry with the arena.
    pub(crate) pre_defined_symbol_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `GeometricRepresentationItem`
    /// arena entry, indexed by `GeometricRepresentationItemId.0`.
    /// Populated by `emit_geometric_representation_items` (phase ds-st)
    /// in two passes — `SymbolTarget` first so `DefinedSymbol` can resolve
    /// its `target` ref through this cache.
    pub(crate) geometric_representation_item_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `CURVE_STYLE` entity, indexed by
    /// `CurveStyleId.0`. Consumed by the PSA writer when dispatching a
    /// `PsaStyle::Curve(...)` entry.
    pub(crate) curve_style_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `STYLED_ITEM` entity, indexed by
    /// `StyledItemId.0`. Populated by `emit_visualization_if_set` before
    /// MDGPR emission so each MDGPR can resolve its `items` list to
    /// cached STEP ids with one index lookup per entry.
    pub(crate) styled_item_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `PRESENTATION_STYLE_ASSIGNMENT`
    /// entity, indexed by `PresentationStyleAssignmentId.0`. Populated by
    /// `emit_visualization_if_set` before any `STYLED_ITEM` /
    /// `OVER_RIDING_STYLED_ITEM` emission so each styled item resolves its
    /// `styles` ref list with one index lookup per entry.
    pub(crate) psa_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `SURFACE_STYLE_RENDERING` /
    /// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` entity, indexed by
    /// `SurfaceStyleRenderingId.0`. Populated by `emit_visualization_if_set`
    /// before `SURFACE_SIDE_STYLE` emission so each
    /// `SurfaceSideStyleEntry::Rendering` resolves with one index lookup.
    pub(crate) ssr_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `FoundedItem` arena entry, indexed
    /// by `FoundedItemId.0`. Populated by `emit_visualization_if_set` in a
    /// 2-pass walk (`FillAreaStyle` first so `SurfaceStyleFillArea` can
    /// resolve its `fill_area` ref); consumed by
    /// `SurfaceStyleFillAreaHandler` and downstream styled-side writers.
    pub(crate) founded_item_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `CameraModel` arena entry, indexed
    /// by `CameraModelId.0`. Populated by `emit_visualization_if_set`
    /// after `founded_item_step_ids`; consumed by `emit_camera_usage_arena`
    /// to resolve `CameraUsage.mapping_origin`.
    pub(crate) viz_camera_model_step_ids: Vec<u64>,
    /// plm Date/Time caches — populated by `emit_plm_if_set` in
    /// dependency order so downstream entities (`LocalTime`, `DateAndTime`)
    /// resolve refs through one index lookup.
    pub(crate) plm_utc_step_ids: Vec<u64>,
    pub(crate) plm_date_step_ids: Vec<u64>,
    pub(crate) plm_date_time_role_step_ids: Vec<u64>,
    pub(crate) plm_local_time_step_ids: Vec<u64>,
    pub(crate) plm_date_and_time_step_ids: Vec<u64>,
    /// plm Person/Org caches — populated by `emit_plm_if_set`.
    pub(crate) plm_person_step_ids: Vec<u64>,
    pub(crate) plm_organization_step_ids: Vec<u64>,
    pub(crate) plm_p_and_o_role_step_ids: Vec<u64>,
    pub(crate) plm_p_and_o_step_ids: Vec<u64>,
    /// plm Approval caches — populated by `emit_plm_if_set` in dependency
    /// order: leaves (status / role) → `Approval` → linkers.
    pub(crate) plm_approval_status_step_ids: Vec<u64>,
    pub(crate) plm_approval_role_step_ids: Vec<u64>,
    pub(crate) plm_approval_step_ids: Vec<u64>,
    pub(crate) plm_approval_date_time_step_ids: Vec<u64>,
    pub(crate) plm_approval_person_organization_step_ids: Vec<u64>,
    /// plm Security caches — populated by `emit_plm_if_set` in
    /// dependency order: level → classification → assignments.
    /// SCA itself has no consumers and is not cached.
    pub(crate) plm_security_level_step_ids: Vec<u64>,
    pub(crate) plm_security_classification_step_ids: Vec<u64>,
    /// plm Identification caches — populated by `emit_plm_if_set`.
    /// `AppliedExternalIdentificationAssignment` itself has no consumers
    /// and is not cached.
    pub(crate) plm_identification_role_step_ids: Vec<u64>,
    pub(crate) plm_external_source_step_ids: Vec<u64>,
    /// plm Document caches — populated by `emit_plm_if_set`. `Applied`
    /// `DocumentReference` is top-level (no consumer) and not cached.
    pub(crate) plm_document_type_step_ids: Vec<u64>,
    pub(crate) plm_document_step_ids: Vec<u64>,
    pub(crate) plm_document_representation_type_step_ids: Vec<u64>,
    pub(crate) plm_document_product_equivalence_step_ids: Vec<u64>,
    /// plm Group cache — `AppliedGroupAssignment` is top-level (no
    /// consumer) and not cached.
    pub(crate) plm_group_step_ids: Vec<u64>,
    /// plm `APPLIED_DOCUMENT_REFERENCE` cache — populated alongside the
    /// Document cluster emit so the Role cluster can resolve
    /// `RoleSelect::DocumentReference` step ids.
    pub(crate) plm_document_reference_step_ids: Vec<u64>,
    /// plm Role cache — `RoleAssociation` is top-level (no consumer)
    /// and not cached.
    pub(crate) plm_object_role_step_ids: Vec<u64>,
    /// plm `address` arena — top-level (no current consumer); cache
    /// reserved for future enhancement phases.
    pub(crate) plm_address_step_ids: Vec<u64>,
    /// IR `ApplicationContext` index → emitted `APPLICATION_CONTEXT` step id.
    pub(crate) ac_step_ids: Vec<u64>,
    /// IR `ShapeAspectId.0` index → emitted `SHAPE_ASPECT` step id.
    /// Populated by `emit_pmi_if_set`; consumed by `id_attribute` writer.
    pub(crate) shape_aspect_step_ids: Vec<u64>,
    /// `SHAPE_ASPECT` subtype index → emitted step id (phase shape-aspect-ref).
    /// Populated by `emit_shape_aspect_subtypes`; consumed by
    /// `emit_shape_aspect_ref`.
    pub(crate) composite_shape_aspect_step_ids: Vec<u64>,
    pub(crate) centre_of_symmetry_step_ids: Vec<u64>,
    pub(crate) all_around_shape_aspect_step_ids: Vec<u64>,
    /// IR `DatumId.0` / `DatumFeatureId.0` index → emitted step id (phase
    /// datum-feature). Populated by `emit_datums` / `emit_datum_features`;
    /// consumed by `emit_shape_aspect_ref`.
    pub(crate) datum_step_ids: Vec<u64>,
    pub(crate) datum_feature_step_ids: Vec<u64>,
    /// IR `GeneralDatumReferenceId.0` index → emitted step id (phase
    /// datum-system). Populated by `emit_general_datum_references`; consumed
    /// by `emit_datum_systems` to resolve a `DATUM_SYSTEM`'s `constituents`.
    pub(crate) general_datum_reference_step_ids: Vec<u64>,
    /// IR `DatumSystemId.0` index → emitted `DATUM_SYSTEM` step id (phase
    /// datum-system). Populated by `emit_datum_systems`; consumed by
    /// `emit_shape_aspect_ref`.
    pub(crate) datum_system_step_ids: Vec<u64>,
    /// Step-id cache for `datum_targets` (phase datum-target). Indexed by
    /// `DatumTargetId.0`. Consumed by `emit_shape_aspect_ref` when a
    /// `ShapeAspectRef::DatumTarget(_)` reaches the writer
    /// (e.g. `feature_for_datum_target_relationship.related_shape_aspect`).
    pub(crate) datum_target_step_ids: Vec<u64>,
    /// Step-id cache for `placed_datum_target_features` (phase
    /// datum-target). Indexed by `PlacedDatumTargetFeatureId.0`.
    pub(crate) placed_datum_target_feature_step_ids: Vec<u64>,
    /// Step-id caches for the plus-minus-tolerance cluster (phase
    /// plus-minus-tolerance). The `tolerance_value` / `limits_and_fits`
    /// caches feed `PLUS_MINUS_TOLERANCE.range`; the dimensional caches feed
    /// its `toleranced_dimension` (`emit_dimensional_locations` /
    /// `emit_dimensional_sizes` fill those — index-cached this phase).
    pub(crate) tolerance_value_step_ids: Vec<u64>,
    pub(crate) limits_and_fits_step_ids: Vec<u64>,
    pub(crate) dimensional_location_step_ids: Vec<u64>,
    pub(crate) dimensional_size_step_ids: Vec<u64>,
    /// Step-id caches for the tolerance-zone phase. The two
    /// `geometric_tolerance` caches feed `TOLERANCE_ZONE.defining_tolerance`
    /// (`emit_geometric_tolerances` / `emit_geometric_tolerance_with_datum_references`
    /// index-fill them); `tolerance_zone_form_step_ids` feeds its `form`
    /// (`emit_pmi_pool`'s `tolerance_zone_forms` loop fills it).
    pub(crate) geometric_tolerance_step_ids: Vec<u64>,
    pub(crate) geometric_tolerance_with_datum_reference_step_ids: Vec<u64>,
    pub(crate) tolerance_zone_form_step_ids: Vec<u64>,
    /// IR `ApplicationProtocolDefinition` index → emitted
    /// `APPLICATION_PROTOCOL_DEFINITION` step id.
    pub(crate) apd_step_ids: Vec<u64>,
    /// IR `ProductContext` index → emitted `PRODUCT_CONTEXT`
    /// (or `MECHANICAL_CONTEXT`) step id.
    pub(crate) pc_step_ids: Vec<u64>,
    /// IR `ProductDefinitionContext` index → emitted
    /// `PRODUCT_DEFINITION_CONTEXT` (or `DESIGN_CONTEXT`) step id.
    pub(crate) pdc_step_ids: Vec<u64>,
    /// Per-`UnitContext` resolved leaf STEP ids `(length, angle, solid_angle)`,
    /// indexed by `UnitContextId.0`. Each tuple is filled in by the GUAC
    /// writer by looking up `UnitContext.{length, plane_angle, solid_angle}`
    /// in [`Self::named_unit_step_ids`] — no new leaf entities are emitted
    /// here. Consumed by the property emitter when resolving a measure's
    /// unit ref.
    pub(crate) unit_leaf_ids: Vec<(u64, u64, u64)>,
    /// `ProductId → best step id for cross-references that target this
    /// product`. Populated by `emit_assembly_chain`; consumed by the
    /// property emitter and the plm `applied_*_assignment` writers so
    /// `Property.target` / SELECT items can be resolved to a STEP ref.
    ///
    /// Value is the `PRODUCT_DEFINITION` step id for products that have a
    /// PDEF chain (the common case — every product that came from a source
    /// `PRODUCT_DEFINITION` or that the kernel built with a
    /// `geometry_context`). For document-style products (`pdef_context =
    /// None && geometry_context = None`, observed in NIST AP242 PMI
    /// fixtures) the PDEF chain is skipped and the value is the `PRODUCT`
    /// step id itself. plm SELECT readers handle both via
    /// `entities/plm/mod.rs::resolve_date_time_item` (chain 3 walks
    /// `product_arena_map` directly), so the round-trip stays symmetric.
    ///
    /// Empty when the model has no assembly (kernel-built IR with
    /// properties only — the property emitter silently skips in that case).
    pub(crate) product_def_ids: std::collections::HashMap<ProductId, u64>,
    /// `ProductId → PRODUCT entity step id`. Phase pc-unify-a:
    /// consumed by `emit_product_categories_arena` so PRPC.products
    /// refs land on the right entity (PDEF doesn't match the schema
    /// here — PRPC.products narrows to PRODUCT itself).
    pub(crate) product_step_ids: std::collections::HashMap<ProductId, u64>,
    /// `ProductCategoryId → step id` (PC or PRPC). Filled by
    /// `emit_product_categories_arena`, consumed by
    /// `emit_product_category_relationships_arena`.
    pub(crate) product_category_step_ids: Vec<u64>,
    /// `ProductId → PRODUCT_DEFINITION_SHAPE step id`. Same role as
    /// `product_def_ids` but for the PDS sibling — consumed by the PMI
    /// emitter to resolve `ShapeAspect.target` (SAs reference PDS, not PD).
    pub(crate) product_def_shape_ids: std::collections::HashMap<ProductId, u64>,
    /// STEP entity id of every emitted `PROPERTY_DEFINITION`, indexed by
    /// `PropertyId.0`. Filled by `emit_properties_if_set`; consumed by the
    /// `GENERAL_PROPERTY_ASSOCIATION` emitter to resolve `derived_definition`.
    /// A slot stays 0 when that property's emit early-returned (no product
    /// chain) — the GPA emitter skips a 0 slot.
    pub(crate) property_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `property_definitions` arena entry
    /// (`PROPERTY_DEFINITION` Itself or `PRODUCT_DEFINITION_SHAPE`),
    /// indexed by `PropertyDefinitionId.0`. Filled by
    /// `emit_property_definitions_if_set` immediately after the assembly
    /// chain so subsequent passes (`emit_pmi_if_set` consuming
    /// `product_def_shape_ids`, `emit_properties_if_set` resolving
    /// `Property.definition`, `emit_general_property_associations` for
    /// `derived_definition`) all see filled slots.
    pub(crate) property_definition_step_ids: Vec<u64>,
    /// STEP entity id of every emitted `GENERAL_PROPERTY`, indexed by
    /// `GeneralPropertyId.0`. Consumed by the GPA emitter for `base_definition`.
    pub(crate) general_property_step_ids: Vec<u64>,
}

impl<'m> WriteBuffer<'m> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new(model: &'m StepModel) -> Self {
        Self {
            model,
            next_id: 0,
            entities: Vec::new(),
            point_ids: HashMap::new(),
            direction_ids: HashMap::new(),
            placement_ids: HashMap::new(),
            placement_1d_ids: HashMap::new(),
            placement_2d_ids: HashMap::new(),
            curve_ids: HashMap::new(),
            surface_ids: HashMap::new(),
            vertex_ids: HashMap::new(),
            edge_ids: HashMap::new(),
            wire_ids: HashMap::new(),
            face_ids: HashMap::new(),
            shell_ids: HashMap::new(),
            solid_ids: HashMap::new(),
            point_2d_ids: HashMap::new(),
            direction_2d_ids: HashMap::new(),
            curve_2d_ids: HashMap::new(),
            planar_extent_ids: HashMap::new(),
            unit_context_ids: Vec::new(),
            representation_step_ids: Vec::new(),
            representation_map_step_ids: Vec::new(),
            tessellated_item_step_ids: Vec::new(),
            tessellated_face_step_ids: Vec::new(),
            tessellated_surface_set_step_ids: Vec::new(),
            acoc_step_ids: Vec::new(),
            ao_step_ids: Vec::new(),
            draughting_callout_step_ids: Vec::new(),
            tolerance_zone_step_ids: Vec::new(),
            type_qualifier_step_ids: Vec::new(),
            value_format_type_qualifier_step_ids: Vec::new(),
            representation_item_step_ids: Vec::new(),
            unit_leaf_ids: Vec::new(),
            named_unit_step_ids: Vec::new(),
            mwu_step_ids: Vec::new(),
            due_step_ids: Vec::new(),
            derived_unit_step_ids: Vec::new(),
            length_dim_exp_step: None,
            dimensionless_dim_exp_step: None,
            mass_dim_exp_step: None,
            colour_step_ids: Vec::new(),
            symbol_colour_step_ids: Vec::new(),
            text_style_for_defined_font_step_ids: Vec::new(),
            pre_defined_marker_step_ids: Vec::new(),
            text_style_step_ids: Vec::new(),
            dptf_step_ids: Vec::new(),
            text_literal_step_ids: Vec::new(),
            dmia_step_ids: Vec::new(),
            unitless_context_step_ids: Vec::new(),
            gisu_step_ids: Vec::new(),
            invisibility_step_ids: Vec::new(),
            presentation_representation_step_ids: Vec::new(),
            presentation_set_step_ids: Vec::new(),
            area_in_set_step_ids: Vec::new(),
            pre_defined_curve_font_step_ids: Vec::new(),
            pre_defined_symbol_step_ids: Vec::new(),
            geometric_representation_item_step_ids: Vec::new(),
            curve_style_step_ids: Vec::new(),
            styled_item_step_ids: Vec::new(),
            psa_step_ids: Vec::new(),
            ssr_step_ids: Vec::new(),
            founded_item_step_ids: Vec::new(),
            viz_camera_model_step_ids: Vec::new(),
            plm_utc_step_ids: Vec::new(),
            plm_date_step_ids: Vec::new(),
            plm_date_time_role_step_ids: Vec::new(),
            plm_local_time_step_ids: Vec::new(),
            plm_date_and_time_step_ids: Vec::new(),
            plm_person_step_ids: Vec::new(),
            plm_organization_step_ids: Vec::new(),
            plm_p_and_o_role_step_ids: Vec::new(),
            plm_p_and_o_step_ids: Vec::new(),
            plm_approval_status_step_ids: Vec::new(),
            plm_approval_role_step_ids: Vec::new(),
            plm_approval_step_ids: Vec::new(),
            plm_approval_date_time_step_ids: Vec::new(),
            plm_approval_person_organization_step_ids: Vec::new(),
            plm_security_level_step_ids: Vec::new(),
            plm_security_classification_step_ids: Vec::new(),
            plm_identification_role_step_ids: Vec::new(),
            plm_external_source_step_ids: Vec::new(),
            plm_document_type_step_ids: Vec::new(),
            plm_document_step_ids: Vec::new(),
            plm_document_representation_type_step_ids: Vec::new(),
            plm_document_product_equivalence_step_ids: Vec::new(),
            plm_group_step_ids: Vec::new(),
            plm_document_reference_step_ids: Vec::new(),
            plm_object_role_step_ids: Vec::new(),
            plm_address_step_ids: Vec::new(),
            ac_step_ids: Vec::new(),
            shape_aspect_step_ids: Vec::new(),
            composite_shape_aspect_step_ids: Vec::new(),
            centre_of_symmetry_step_ids: Vec::new(),
            all_around_shape_aspect_step_ids: Vec::new(),
            datum_step_ids: Vec::new(),
            datum_feature_step_ids: Vec::new(),
            general_datum_reference_step_ids: Vec::new(),
            datum_system_step_ids: Vec::new(),
            datum_target_step_ids: Vec::new(),
            placed_datum_target_feature_step_ids: Vec::new(),
            tolerance_value_step_ids: Vec::new(),
            geometric_tolerance_step_ids: Vec::new(),
            geometric_tolerance_with_datum_reference_step_ids: Vec::new(),
            tolerance_zone_form_step_ids: Vec::new(),
            limits_and_fits_step_ids: Vec::new(),
            dimensional_location_step_ids: Vec::new(),
            dimensional_size_step_ids: Vec::new(),
            apd_step_ids: Vec::new(),
            pc_step_ids: Vec::new(),
            pdc_step_ids: Vec::new(),
            product_def_ids: std::collections::HashMap::new(),
            product_step_ids: std::collections::HashMap::new(),
            product_category_step_ids: Vec::new(),
            product_def_shape_ids: std::collections::HashMap::new(),
            property_step_ids: Vec::new(),
            property_definition_step_ids: Vec::new(),
            general_property_step_ids: Vec::new(),
        }
    }

    pub(crate) fn finish_entities(self) -> Vec<WriterEntity> {
        self.entities
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_all(&mut self) -> Result<(), WriteError> {
        // Order: geometry -> topology -> units -> assembly. Mirrors the
        // OCCT-flavoured fixture layout (topology before units) and keeps
        // all cross-pool references backward (parent after children).
        // Arena iteration yields the original Id order, so dedup maps set
        // in one pass are reused in the next.
        for id in self.model.geometry.points.iter_ids() {
            self.emit_point(id)?;
        }
        for id in self.model.geometry.directions.iter_ids() {
            self.emit_direction(id)?;
        }
        for id in self.model.geometry.placements.iter_ids() {
            self.emit_axis2_placement_3d(id)?;
        }
        for id in self.model.geometry.placements_1d.iter_ids() {
            self.emit_axis1_placement(id)?;
        }
        for id in self.model.geometry.curves.iter_ids() {
            self.emit_curve(id)?;
        }
        for id in self.model.geometry.surfaces.iter_ids() {
            self.emit_surface(id)?;
        }
        // 2D geometry (PCURVE parametric space) — emitted after 3D surfaces
        // so `emit_surface_curve_wrapper` can cache-hit the 3D basis surface,
        // and before topology so `emit_edge` can cache-hit the 2D curves.
        // Arena is empty for files without PCURVE content → zero iterations.
        for id in self.model.geometry.points_2d.iter_ids() {
            self.emit_point_2d(id)?;
        }
        for id in self.model.geometry.directions_2d.iter_ids() {
            self.emit_direction_2d(id)?;
        }
        for id in self.model.geometry.placements_2d.iter_ids() {
            self.emit_axis2_placement_2d(id)?;
        }
        // PLANAR_EXTENT / PLANAR_BOX — after the placement loops so a
        // PLANAR_BOX's `emit_axis2_placement_*` only cache-hits (emitting a
        // placement out of arena order would desync the placement arenas).
        for id in self.model.geometry.planar_extents.iter_ids() {
            self.emit_planar_extent(id)?;
        }
        for id in self.model.geometry.curves_2d.iter_ids() {
            self.emit_curve_2d(id)?;
        }
        for id in self.model.geometry.vertices.iter_ids() {
            self.emit_vertex(id)?;
        }
        for id in self.model.topology.edges.iter_ids() {
            self.emit_edge(id)?;
        }
        for id in self.model.topology.wires.iter_ids() {
            self.emit_wire(id)?;
        }
        for id in self.model.topology.faces.iter_ids() {
            self.emit_face(id)?;
        }
        for id in self.model.topology.shells.iter_ids() {
            self.emit_shell(id)?;
        }
        for id in self.model.topology.solids.iter_ids() {
            self.emit_solid(id)?;
        }
        // units-2: emit the units pool first so `named_unit_step_ids` is
        // populated before GUAC emit (which now looks up its leaf step
        // ids from that cache instead of producing fresh entities).
        self.emit_units_pool_if_set()?;
        // Emit one REPRESENTATION_CONTEXT per IR `UnitContext`. The cached
        // STEP ids land in `unit_context_ids` and `unit_leaf_ids` so each
        // downstream emitter can resolve `Option<UnitContextId>` with a
        // single index lookup. Leaf entities are reused from the units
        // pool — see GUAC writer.
        self.unit_context_ids = Vec::with_capacity(self.model.units.len());
        self.unit_leaf_ids = Vec::with_capacity(self.model.units.len());
        for ctx in self.model.units.iter() {
            let id = self.emit_unit_context(ctx.clone())?;
            self.unit_context_ids.push(id);
        }
        // Emit unitless (GRC PRC REP_CONTEXT) contexts after the unit-bearing
        // contexts so referencing representations can look up their step ids
        // via `unitless_context_step_ids`.
        self.emit_unitless_contexts()?;
        // Pre-emit every geometry representation (ABSR / MSSR / plain SR /
        // GBWSR / GBSSR) in `representations` arena order so re-read assigns
        // identical `RepresentationId`s — the same round-trip-stability
        // mechanism the curve / surface / solid arenas rely on. MDGPR is
        // skipped here and emitted by the visualization pass (it depends on
        // STYLED_ITEMs). No-op for hand/kernel-built IR (arena unpopulated).
        // Pre-emit SBSM slots of the `geometric_representation_item` arena
        // first so the upcoming `emit_representations_pre_pass` (MSSR) can
        // resolve its child SBSMs through the GRI cache (phase
        // sbsm-cluster). The symbol-domain GRI entries still emit after
        // visualization (`emit_geometric_representation_items`).
        self.emit_sbsm_in_gri_arena()?;
        self.emit_representations_pre_pass()?;
        self.emit_product_chain_if_eligible()?;
        // PC cluster (phase pc-unify-a) — arena-driven emit. Coexists with
        // the legacy `emit_product_category_chain` inline path until phase
        // pc-unify-b removes the latter.
        self.emit_product_categories_arena();
        self.emit_product_category_relationships_arena();
        self.emit_pmi_if_set();
        // Second half of the PD orchestrator — runs after SA emit so
        // `shape_aspect_step_ids` is populated for Pattern B targets
        // (PROPERTY_DEFINITION whose definition is a SHAPE_ASPECT).
        self.emit_property_definitions_non_pds();
        // general_datum_reference + DATUM_SYSTEM — emitted before the
        // ShapeAspectRef consumers below so `datum_system_step_ids` is
        // filled when `emit_shape_aspect_ref` runs. `emit_datum_systems`
        // also needs `general_datum_reference_step_ids`, so the order is
        // general_datum_reference → datum_system → consumers.
        self.emit_general_datum_references();
        self.emit_datum_systems();
        // DATUM_TARGET / PLACED_DATUM_TARGET_FEATURE — same wave as
        // datum_systems. Fills the two new `*_step_ids` caches so
        // `emit_shape_aspect_ref` resolves their `ShapeAspectRef` variants
        // (e.g. `feature_for_datum_target_relationship.related_shape_aspect`).
        self.emit_datum_targets();
        self.emit_placed_datum_target_features();
        // SHAPE_ASPECT_RELATIONSHIP — after emit_pmi_if_set so every
        // shape-aspect-family step-id cache is filled.
        self.emit_shape_aspect_relationships()?;
        self.emit_dimensional_sizes()?;
        self.emit_dimensional_locations()?;
        // geometric_tolerance form tolerances — after the units pass
        // (`mwu_step_ids`) and emit_pmi_if_set (shape-aspect caches).
        self.emit_geometric_tolerances();
        // geometric_tolerance_with_datum_reference — also after
        // emit_datum_systems (`datum_system_step_ids`).
        self.emit_geometric_tolerance_with_datum_references();
        // GEOMETRIC_TOLERANCE_RELATIONSHIP — pairs two GT entries; both
        // GT step-id caches must be filled by the two emits above.
        self.emit_geometric_tolerance_relationships();
        // TOLERANCE_ZONE — after both geometric_tolerance emits (its
        // `defining_tolerance` caches) and emit_pmi_if_set (the
        // `tolerance_zone_form_step_ids` cache).
        self.emit_tolerance_zones();
        // PROJECTED_ZONE_DEFINITION — needs the `tolerance_zone_step_ids`
        // cache filled above, plus shape-aspect caches and `mwu_step_ids`.
        self.emit_tolerance_zone_definitions();
        // plus-minus-tolerance cluster — TOLERANCE_VALUE / LIMITS_AND_FITS
        // first (PLUS_MINUS_TOLERANCE's `range`), then PLUS_MINUS_TOLERANCE
        // which also needs the dimensional caches filled above.
        self.emit_tolerance_values();
        self.emit_limits_and_fits();
        self.emit_plus_minus_tolerances();
        // MEASURE_QUALIFICATION — depends on type_qualifier_step_ids /
        // value_format_type_qualifier_step_ids (filled by emit_pmi_pool)
        // and mwu_step_ids.
        self.emit_measure_qualifications();
        // QUALIFIED / VALUE REPRESENTATION_ITEM — depends on type_qualifier
        // / value_format_type_qualifier step ids (emit_pmi_pool).
        self.emit_representation_items();
        // CHARACTERIZED_ITEM_WITHIN_REPRESENTATION — depends on
        // representation_step_ids + per-type arena step ids.
        self.emit_characterized_objects();
        // tessellation arena emit moved earlier (phase tessellation-repr-item)
        // so STYLED_ITEM can resolve a TESSELLATED_SOLID target through
        // `tessellated_item_step_ids` during visualization. Only the
        // `tessellated_items` arena is filled here; annotation_occurrence
        // and related orphans still emit after visualization.
        self.emit_tessellation()?;
        self.emit_visualization_if_set()?;
        // GEOMETRIC_REPRESENTATION_ITEM (phase ds-st) — emitted after
        // visualization so `pre_defined_symbol_step_ids` is populated
        // before `DefinedSymbol.definition` resolves through it.
        self.emit_geometric_representation_items()?;
        // COMPOUND_REPRESENTATION_ITEM — orphan, runs after all
        // representation_item arenas are emitted (DRI is re-emitted
        // inline by the handler).
        self.emit_compound_representation_items()?;
        // ITEM_IDENTIFIED_REPRESENTATION_USAGE — orphan, after all
        // PMI / shape_aspect / representation step-id caches populated.
        for iiru in self
            .model
            .item_identified_representation_usages
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::shape_rep::iiru::ItemIdentifiedRepresentationUsageHandler;
            ItemIdentifiedRepresentationUsageHandler::write(self, iiru)?;
        }
        // CIRCULAR_AREA — orphan, no ref to representations or items.
        for ca in self
            .model
            .geometry
            .circular_areas
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::geometry::circular_area::CircularAreaHandler;
            CircularAreaHandler::write(self, ca)?;
        }
        // BOUNDED_PCURVE — orphan, references surface + representation.
        for psc in self
            .model
            .geometry
            .parameter_space_curves
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::geometry::bounded_pcurve::BoundedPCurveHandler;
            use crate::ir::geometry::ParameterSpaceCurve;
            match psc {
                ParameterSpaceCurve::BoundedPCurve(b) => {
                    BoundedPCurveHandler::write(self, b)?;
                }
            }
        }
        // BOUNDED_SURFACE_CURVE + INTERSECTION_CURVE — orphan, after curves
        // + surfaces emitted.
        for sc in self
            .model
            .geometry
            .surface_curves
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::geometry::surface_curve_subtypes::{
                BoundedSurfaceCurveHandler, IntersectionCurveHandler,
            };
            use crate::ir::geometry::SurfaceCurve;
            match sc {
                SurfaceCurve::BoundedSurfaceCurve(b) => {
                    BoundedSurfaceCurveHandler::write(self, b)?;
                }
                SurfaceCurve::IntersectionCurve(i) => {
                    IntersectionCurveHandler::write(self, i)?;
                }
            }
        }
        // REPRESENTATION_MAP + MAPPED_ITEM — after visualization so the
        // `representation_step_ids` cache covers MDGPR slots too.
        self.emit_mapped_items()?;
        // INTEGER/REAL_REPRESENTATION_ITEM — orphan value-items, no refs.
        self.emit_numeric_representation_items()?;
        // COORDINATES_LIST + COMPLEX_TRIANGULATED_FACE — tessellation moved
        // earlier (before `emit_visualization_if_set`) to let STYLED_ITEM
        // resolve TESSELLATED_SOLID targets through `tessellated_item_step_ids`.
        // annotation_occurrence — after visualization (`psa_step_ids`) and
        // after tessellation (`tessellated_item_step_ids` for a
        // `TESSELLATED_ANNOTATION_OCCURRENCE`'s `item`).
        // LEADER_CURVE first — fills `acoc_step_ids` consumed by
        // `TERMINATOR_SYMBOL` / `LEADER_TERMINATOR` in
        // `emit_annotation_occurrences`.
        self.emit_annotation_curve_occurrences();
        self.emit_annotation_occurrences();
        // DRAUGHTING_CALLOUT — after annotation_occurrences (`ao_step_ids`)
        // and after `acoc_step_ids` so `contents` SELECT resolves.
        self.emit_draughting_callouts();
        // DRAUGHTING_CALLOUT_RELATIONSHIP — after draughting_callouts.
        self.emit_draughting_callout_relationships();
        // DRAUGHTING_MODEL — delayed emit (Mdgpr pattern). At this point
        // every items ref cache (styled_item, ao, draughting_callout,
        // representation_item, per-geometry placements) is populated, so
        // the items refs serialise to valid step ids.
        self.emit_draughting_models()?;
        // TESSELLATED_SHAPE_REPRESENTATION — delayed emit (Mdgpr / DM
        // pattern). Runs after `emit_tessellation` (line 687) so the
        // tessellated step-id caches are populated, and after
        // `emit_draughting_models` so `representation_step_ids` is
        // appended in arena tail order (Mdgpr → DM → TSR).
        self.emit_tessellated_shape_representations()?;
        // CONSTRUCTIVE_GEOMETRY_REPRESENTATION — delayed emit (Mdgpr / DM /
        // TSR pattern). Runs after the prior delayed emits so
        // representation_step_ids is appended in arena order
        // (Mdgpr → DM → TSR → CGR).
        self.emit_constructive_geometry_representations()?;
        // SHAPE_REPRESENTATION_WITH_PARAMETERS — delayed emit. After
        // CGR so representation_step_ids keeps arena order.
        self.emit_shape_representation_with_parameters()?;
        // CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP — runs after
        // every Representation delayed emit so rep_1 / rep_2 resolve
        // through the fully populated representation_step_ids cache.
        self.emit_representation_relationships()?;
        // CAMERA_USAGE — delayed emit (mirrors Mdgpr / DraughtingModel
        // pattern). `mapped_representation` may target a DM, so this runs
        // after `emit_draughting_models` populates the DM slot of
        // `representation_step_ids`. Overwrites the 0 placeholder slot
        // that `emit_mapped_items` reserved for each CameraUsage entry.
        self.emit_camera_usage_arena()?;
        // CAMERA_IMAGE / CAMERA_IMAGE_3D_WITH_SCALE — depends on the
        // CameraUsage slot of `representation_map_step_ids` populated by
        // `emit_camera_usage_arena`.
        self.emit_camera_image_arena()?;
        // DRAUGHTING_MODEL_ITEM_ASSOCIATION — after representation chain
        // (`representation_step_ids` now includes DraughtingModel slots),
        // `ao_step_ids`, and `draughting_callout_step_ids`.
        self.emit_dmia();
        // GEOMETRIC_ITEM_SPECIFIC_USAGE — same dependencies as DMIA.
        self.emit_gisu();
        // INVISIBILITY — depends on styled_item / representation /
        // draughting_callout step ids (all populated above). Delayed
        // emit (Mdgpr / DraughtingModel pattern) — placing this inside
        // `emit_visualization_if_set` panics on corpora that reference
        // `DRAUGHTING_CALLOUT` since that cache is filled later.
        self.emit_invisibilities()?;
        // PRESENTATION_VIEW / AREA / SET — depends on repr_item and per-
        // geometry placement caches (all populated above).
        self.emit_presentation_repr_cluster()?;
        // AREA_IN_SET + PRESENTATION_SIZE — depends on pr-core step ids.
        self.emit_pr_size()?;
        // PRESENTED_ITEM_REPRESENTATION + APPLIED_PRESENTED_ITEM — depends
        // on pr-core caches + product chain (`product_def_ids`).
        self.emit_pr_item()?;
        self.emit_plm_if_set()?;
        self.emit_properties_if_set();
        self.emit_form_features_if_set()?;
        Ok(())
    }

    pub(crate) fn fresh(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}
