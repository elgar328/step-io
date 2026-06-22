//! IR → `Vec<WriterEntity>` conversion.
//!
//! `WriteBuffer` assigns `#N` ids and assembles `WriterEntity` values as it
//! walks the IR. Each concern lives in its own sub-module:
//!
//! - [`geometry`] — points, directions, placements, curves, surfaces
//! - [`topology`] — vertices, edges, wires, faces, shells, solids
//! - [`units`] — `SI_UNIT`-based unit context
//! - [`assembly`] — single-part PRODUCT chain + ABSR + SDR

use super::WriteError;
use super::entity::WriterEntity;
use crate::ir::{ProductId, StepModel};

pub(crate) mod assembly;
pub(crate) mod geometry;
pub(crate) mod mapped_item;
pub(crate) mod numeric_representation_item;
pub(crate) mod plm;
pub(crate) mod pmi;
pub(crate) mod property;
pub(crate) mod step_id_cache;
pub(crate) mod tessellation;
pub(crate) mod topology;
pub(crate) mod units;
pub(crate) mod visualization;

/// Coarse emit phase for `emit_all`'s ordering guard. `emit_all` advances this
/// monotonically (it is the master) at each coarse block; cross-phase emit
/// helpers assert `phase >= EmitPhase::<dependency>` on entry. The variant
/// declaration order *is* `emit_all`'s intended call order, so a block moved
/// out of order trips `set_phase`'s monotonic `debug_assert` (or a dependent's
/// `assert_phase`) in debug builds. All of this is `debug_assert!` only — a
/// no-op in release, so output is unaffected.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum EmitPhase {
    Init,
    Geometry,
    Topology,
    Units,
    Representations,
    Product,
    Pmi,
    Tessellation,
    MappedItems,
    Visualization,
    Annotations,
    DraughtingModels,
    CharacterizedObjects,
    ReprRelationships,
    Presentation,
    PlmProperties,
}

pub(crate) struct WriteBuffer<'m> {
    pub(crate) model: &'m StepModel,
    pub(crate) next_id: u64,
    pub(crate) entities: Vec<WriterEntity>,
    /// Emitted STEP id per arena entry, keyed by arena-id type. Replaces the
    /// former hand-declared `*_step_ids: Vec<u64>` fields (arena-1:1 caches).
    pub(crate) step_ids: step_id_cache::StepIdCache,
    /// STEP entity ids of every emitted `REPRESENTATION_CONTEXT` complex
    /// entity, indexed by `UnitContextId.0`. Populated up-front in `emit_all`
    /// so every representation emitter can resolve its `Option<UnitContextId>`
    /// to a cached id.
    pub(crate) unit_context_ids: Vec<u64>,
    // tessellated_item / tessellated_face / tessellated_surface_set step ids
    // now live in `step_ids` keyed by their arena-id type.
    // units-pool step ids (NAMED_UNIT / MEASURE_WITH_UNIT / DERIVED_UNIT_ELEMENT
    // / DERIVED_UNIT) now live in `step_ids` keyed by their arena-id type.
    /// Assembly `ADVANCED_BREP_SHAPE_REPRESENTATION`s (items include a
    /// `MAPPED_ITEM`) whose step id was reserved in `emit_representations_pre_pass`
    /// and whose body is emitted by `emit_deferred_assembly_absr` after
    /// `emit_mapped_items`. `(RepresentationId, reserved step id)`.
    pub(crate) deferred_assembly_absr_ids: Vec<(crate::ir::RepresentationId, u64)>,
    /// IR `ApplicationContext` index → emitted `APPLICATION_CONTEXT` step id.
    pub(crate) ac_step_ids: Vec<u64>,
    /// IR `ProductContext` index → emitted `PRODUCT_CONTEXT`
    /// (or `MECHANICAL_CONTEXT`) step id.
    pub(crate) pc_step_ids: Vec<u64>,
    /// IR `ProductDefinitionContext` index → emitted
    /// `PRODUCT_DEFINITION_CONTEXT` (or `DESIGN_CONTEXT`) step id.
    pub(crate) pdc_step_ids: Vec<u64>,
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
    /// `ProductId → PRODUCT_DEFINITION_SHAPE step id`. Same role as
    /// `product_def_ids` but for the PDS sibling — consumed by the PMI
    /// emitter to resolve `ShapeAspect.target` (SAs reference PDS, not PD).
    pub(crate) product_def_shape_ids: std::collections::HashMap<ProductId, u64>,
    /// `AssemblyComponentUsageId → (property_definitions arena index, source
    /// name, source description)` for NAUO-owned `PRODUCT_DEFINITION_SHAPE`s.
    /// Recorded by `emit_property_definitions_pds_only`; the assembly chain
    /// (`emit_instance_bundle`) emits the PDS body and fills
    /// `property_definition_step_ids[idx]`. If an instance is skipped, the slot
    /// stays `0` and the dependent centroid PD is dropped symmetrically (no
    /// dangling reference). Absent ⇒ kernel-built IR (synthesise instead).
    pub(crate) nauo_pds_arena_slot:
        std::collections::HashMap<crate::ir::id::AssemblyComponentUsageId, (usize, String, String)>,
    /// Coarse emit phase — `emit_all`'s ordering-guard state (master). See
    /// [`EmitPhase`]. Debug-only guard; no effect on output.
    phase: EmitPhase,
    /// Output schema target. `Universal` = emit as-is (current behaviour);
    /// `Ap*` triggers projection (entity prune + `FILE_SCHEMA`/`APD` retarget),
    /// consumed in the post-build projection pass. Stored here so the deep
    /// emit chain can reach it without threading a param everywhere.
    #[allow(dead_code)] // consumed by the projection pass (batch 2c)
    pub(crate) target: crate::writer::SchemaTarget,
}

impl<'m> WriteBuffer<'m> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new(model: &'m StepModel, target: crate::writer::SchemaTarget) -> Self {
        Self {
            model,
            target,
            next_id: 0,
            entities: Vec::new(),
            step_ids: step_id_cache::StepIdCache::default(),
            unit_context_ids: Vec::new(),
            deferred_assembly_absr_ids: Vec::new(),
            ac_step_ids: Vec::new(),
            pc_step_ids: Vec::new(),
            pdc_step_ids: Vec::new(),
            product_def_ids: std::collections::HashMap::new(),
            product_step_ids: std::collections::HashMap::new(),
            product_def_shape_ids: std::collections::HashMap::new(),
            nauo_pds_arena_slot: std::collections::HashMap::new(),
            phase: EmitPhase::Init,
        }
    }

    pub(crate) fn finish_entities(self) -> Vec<WriterEntity> {
        self.entities
    }

    /// Advance the coarse emit phase (monotonic). A non-increasing transition
    /// means an `emit_all` block was reordered — caught in debug builds.
    pub(in crate::writer::buffer) fn set_phase(&mut self, p: EmitPhase) {
        debug_assert!(
            p > self.phase,
            "emit order: phase regressed {:?} -> {p:?} (block swap?)",
            self.phase
        );
        self.phase = p;
    }

    /// Guard a cross-phase dependency: the caller requires every cache from
    /// phases up to `min` to be populated. Debug-only.
    pub(in crate::writer::buffer) fn assert_phase(&self, min: EmitPhase, ctx: &str) {
        debug_assert!(
            self.phase >= min,
            "emit order guard: {ctx} requires phase >= {min:?}, got {:?}",
            self.phase
        );
    }

    // Arena indices fit u32 by the arena's own overflow guard, so the
    // `idx as u32` surface-curve id reconstruction is safe.
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    pub(crate) fn emit_all(&mut self) -> Result<(), WriteError> {
        // Order: geometry -> topology -> units -> assembly. Mirrors the
        // OCCT-flavoured fixture layout (topology before units) and keeps
        // all cross-pool references backward (parent after children).
        // Arena iteration yields the original Id order, so dedup maps set
        // in one pass are reused in the next.
        // Reserve an id per P21 edition 3 external reference up front (in arena
        // order) so any DATA entity resolving one — e.g. CIRCULAR_AREA.centre —
        // emits a consistent `#N`, and write_file can emit the REFERENCE
        // section. Deterministic: same arena → same ids across round-trips.
        for id in self.model.metadata.external_references.iter_ids() {
            let reserved = self.fresh();
            self.set_step_id(id, reserved);
        }
        self.set_phase(EmitPhase::Geometry);
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
        // so `emit_surface_curve_node` can cache-hit the 3D basis surface,
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
        self.set_phase(EmitPhase::Topology);
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
        self.set_phase(EmitPhase::Units);
        self.emit_units_pool_if_set()?;
        // Emit one REPRESENTATION_CONTEXT per IR `UnitContext`. The cached
        // STEP ids land in `unit_context_ids` so each downstream emitter can
        // resolve `Option<UnitContextId>` with a single index lookup. Leaf
        // entities are reused from the units pool — see GUAC writer.
        self.unit_context_ids = Vec::with_capacity(self.model.shape_rep.unit_contexts.len());
        for ctx in self.model.shape_rep.unit_contexts.iter() {
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
        self.set_phase(EmitPhase::Representations);
        self.emit_sbsm_in_gri_arena()?;
        // Emit the value-qualifier pools and the representation_item arena
        // before the representation pre-pass (phase measure-arena-1): a
        // SHAPE_DIMENSION_REPRESENTATION emitted in the pre-pass resolves its
        // MeasureRepresentationItem items through representation_item_step_ids,
        // and the MRI/QRI emit needs the qualifier step caches (units are
        // already populated by emit_units_pool_if_set above).
        self.emit_value_qualifier_pools();
        self.emit_representation_items();
        self.emit_representations_pre_pass()?;
        // DOCUMENT prepass — must run before the product chain so a product's
        // PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS can resolve its
        // documentation_ids to DOCUMENT step ids in emit_pdef. The prepass is
        // leaf (DOCUMENT_TYPE / DOCUMENT / DOCUMENT_FILE reference only the
        // document_type it emits itself), so this earlier position is safe and
        // still precedes emit_property_definitions_non_pds below.
        if let Some(plm) = self.model.plm.clone() {
            self.emit_documents_prepass(&plm)?;
        }
        self.set_phase(EmitPhase::Product);
        self.emit_product_chain_if_eligible()?;
        // PC cluster (phase pc-unify-a) — arena-driven emit. Coexists with
        // the legacy `emit_product_category_chain` inline path until phase
        // pc-unify-b removes the latter.
        self.emit_product_categories_arena();
        self.emit_product_category_relationships_arena();
        self.set_phase(EmitPhase::Pmi);
        self.emit_pmi_if_set();
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
        // geometric_tolerance(+_with_datum_reference) — moved before the PD
        // pass so a PD.definition targeting a GEOMETRIC_TOLERANCE resolves its
        // step id. Deps (mwu / shape-aspect / datum_systems) are all filled by
        // the emits above. GT relationships stay after (below).
        self.emit_geometric_tolerances();
        self.emit_geometric_tolerance_with_datum_references();
        // Second half of the PD orchestrator — runs after every
        // step-id cache its definition SELECT may reference is populated:
        // SA-family (`emit_shape_aspect_ref` covering DATUM_FEATURE /
        // DATUM_SYSTEM / DATUM_TARGET / PLACED_DATUM_TARGET_FEATURE /
        // COMPOSITE_GROUP_SHAPE_ASPECT / CENTRE_OF_SYMMETRY /
        // ALL_AROUND_SHAPE_ASPECT) and the dimensional_location family
        // (`dimensional_location_step_ids`, the shape_aspect_relationship
        // SELECT branch).
        // GENERAL_PROPERTY step ids first — a PD.definition may target a
        // GeneralProperty, so that step-id cache must be filled before
        // emit_property_definitions_non_pds resolves that branch. (DOCUMENT
        // step ids are filled by the prepass moved above the product chain.)
        if let Some(pool) = self.model.properties.clone() {
            self.emit_general_properties(&pool);
        }
        // Reserve CIWR step ids so a PD.definition targeting one resolves the
        // forward ref at emit_property_definitions_non_pds (CO bodies emit
        // later in emit_characterized_objects).
        self.emit_characterized_objects_prepass();
        self.emit_property_definitions_non_pds();
        // GEOMETRIC_TOLERANCE_RELATIONSHIP — pairs two GT entries; both
        // GT step-id caches are filled by emit_geometric_tolerances(+_with_datum)
        // above (moved before the PD pass).
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
        // emit_representation_items moved before emit_representations_pre_pass
        // (phase measure-arena-1) so a SHAPE_DIMENSION_REPRESENTATION emitted
        // in the pre-pass resolves its MeasureRepresentationItem items through
        // a populated representation_item_step_ids cache.
        // NOTE: emit_characterized_objects() is deferred to after the
        // delayed-emit Representation block below — a CIWR body resolves its
        // `rep` (a DraughtingModel, stepped by emit_draughting_models) and its
        // `item` (a DRAUGHTING_CALLOUT, stepped by emit_draughting_callouts),
        // neither of which is populated at this point.
        // tessellation arena emit moved earlier (phase tessellation-repr-item)
        // so STYLED_ITEM can resolve a TESSELLATED_SOLID target through
        // `tessellated_item_step_ids` during visualization. Only the
        // `tessellated_items` arena is filled here; annotation_occurrence
        // and related orphans still emit after visualization.
        self.set_phase(EmitPhase::Tessellation);
        self.emit_tessellation()?;
        // REPRESENTATION_MAP + MAPPED_ITEM (phase si-mapped-item) — moved
        // before visualization so STYLED_ITEM / CDORSI can resolve a
        // MAPPED_ITEM target through `mapped_item_step_ids`. MDGPR /
        // STYLED_ITEM-target MAPPED_ITEMs are reader-cascade-dropped, so
        // the cache need not cover MDGPR slots.
        self.set_phase(EmitPhase::MappedItems);
        self.emit_mapped_items()?;
        // Assembly ABSR bodies whose step ids were reserved in the pre-pass —
        // their MAPPED_ITEM items now resolve (emit_mapped_items just ran).
        self.emit_deferred_assembly_absr()?;
        self.set_phase(EmitPhase::Visualization);
        self.emit_visualization_if_set()?;
        // Camera-origin plain REPRESENTATION_MAPs + their MAPPED_ITEMs, deferred
        // by `emit_mapped_items` until cameras are stepped. Must precede
        // `emit_draughting_models` (MBD complex items index `mapped_item_step_ids`).
        self.emit_camera_origin_mapped_items()?;
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
            .shape_rep
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
        // SURFACE_CURVE family (base/seam + bounded/intersection) — emit any
        // node an edge did not already reach. `emit_surface_curve_node` is
        // step_id-cached so edge-referenced nodes are not re-emitted.
        let surface_curve_count = self.model.geometry.surface_curves.iter().count();
        for idx in 0..surface_curve_count {
            self.emit_surface_curve_node(crate::ir::id::SurfaceCurveSubtypeId(idx as u32))?;
        }
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
        self.set_phase(EmitPhase::Annotations);
        self.emit_annotation_curve_occurrences();
        // APLL_POINT then ANNOTATION_TO_MODEL_LEADER_LINE before
        // `emit_annotation_occurrences` so a `_WITH_LEADER_LINE` APO resolves
        // its `leader_line` refs (leaf apll_point → leader_line → APO).
        self.emit_apll_points();
        self.emit_annotation_placeholder_leader_lines();
        self.emit_annotation_occurrences();
        // DRAUGHTING_CALLOUT — after annotation_occurrences (`ao_step_ids`)
        // and after `acoc_step_ids` so `contents` SELECT resolves.
        self.emit_draughting_callouts();
        // DRAUGHTING_CALLOUT_RELATIONSHIP — after draughting_callouts.
        self.emit_draughting_callout_relationships();
        // ANNOTATION_OCCURRENCE_ASSOCIATIVITY — after annotation occurrence
        // emits (`ao_step_ids` / `acoc_step_ids` populated) so relating /
        // related refs resolve.
        self.emit_annotation_occurrence_associativities();
        // DRAUGHTING_MODEL — delayed emit (Mdgpr pattern). At this point
        // every items ref cache (styled_item, ao, draughting_callout,
        // representation_item, per-geometry placements) is populated, so
        // the items refs serialise to valid step ids.
        self.set_phase(EmitPhase::DraughtingModels);
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
        // DEFAULT_MODEL_GEOMETRIC_VIEW — leaf; its `rep` is a DraughtingModel
        // (stepped above), `item` a camera, `of_shape` a PDS — all now cached.
        self.emit_default_model_geometric_views()?;
        // CHARACTERIZED_ITEM_WITHIN_REPRESENTATION bodies — deferred here
        // (from before emit_property_definitions_non_pds) so a CIWR resolves
        // its `rep` (DraughtingModel, stepped by emit_draughting_models above)
        // and `item` (DRAUGHTING_CALLOUT, stepped by emit_draughting_callouts
        // above). The step ids were reserved by emit_characterized_objects_prepass
        // so the PD-definition forward refs already point at them.
        self.set_phase(EmitPhase::CharacterizedObjects);
        self.emit_characterized_objects();
        // CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP — runs after
        // every Representation delayed emit so rep_1 / rep_2 resolve
        // through the fully populated representation_step_ids cache.
        self.set_phase(EmitPhase::ReprRelationships);
        self.emit_representation_relationships()?;
        // PD-based SHAPE_DEFINITION_REPRESENTATIONs — after BOTH
        // emit_property_definitions_non_pds (definition PD step id) AND every
        // representation delayed emit (used_representation step id). PD-based
        // SDRs don't fold into Product, so step-id order is irrelevant; the
        // re-read stashes + resolves them regardless.
        self.emit_sdr_links();
        // PDR links whose used_representation is a modelled representation
        // (e.g. SHAPE_REPRESENTATION) — same ordering constraints as SDR links.
        self.emit_pdr_links();
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
        self.set_phase(EmitPhase::Presentation);
        self.emit_invisibilities()?;
        // PRESENTATION_VIEW / AREA / SET — depends on repr_item and per-
        // geometry placement caches (all populated above).
        self.emit_presentation_repr_cluster()?;
        // AREA_IN_SET + PRESENTATION_SIZE — depends on pr-core step ids.
        self.emit_pr_size()?;
        // PRESENTED_ITEM_REPRESENTATION + APPLIED_PRESENTED_ITEM — depends
        // on pr-core caches + product chain (`product_def_ids`).
        self.emit_pr_item()?;
        self.set_phase(EmitPhase::PlmProperties);
        self.emit_plm_if_set()?;
        self.emit_properties_if_set();
        Ok(())
    }

    pub(crate) fn fresh(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Emitted STEP id for an arena entry, or `0` if not yet emitted. Accepts
    /// an id by value or by reference (`id` is taken by value so a `&Id` from
    /// list iteration binds without an extra deref at the call site).
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn step_id<I: crate::ir::arena::AsArenaId>(&self, id: I) -> u64 {
        self.step_ids.get(id.as_arena_id())
    }

    /// Record the emitted STEP id for an arena entry.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn set_step_id<I: crate::ir::arena::AsArenaId>(&mut self, id: I, step: u64) {
        self.step_ids.set(id.as_arena_id(), step);
    }
}

/// Writer seam for `#[derive(StepSelect)]`-generated `emit_select`: delegate to
/// [`WriteBuffer::step_id`].
impl crate::ir::select::StepResolver for WriteBuffer<'_> {
    fn step_of<K: crate::ir::arena::AsArenaId>(&self, id: K) -> u64 {
        self.step_id(id)
    }
}

#[cfg(all(test, debug_assertions))]
mod phase_guard_tests {
    use super::{EmitPhase, WriteBuffer};
    use crate::ir::StepModel;

    // A cross-phase emit helper called before its dependency phase must trip
    // the order guard (debug build only — the guard is `debug_assert!`).
    #[test]
    #[should_panic(expected = "emit order guard")]
    fn visualization_before_mapped_items_panics() {
        let model = StepModel::default();
        let mut buf = WriteBuffer::new(&model, crate::writer::SchemaTarget::Universal);
        // phase is `Init` (< MappedItems); the assert fires before any work.
        let _ = buf.emit_visualization_if_set();
    }

    // A non-increasing `set_phase` (a reordered emit_all block) must panic.
    #[test]
    #[should_panic(expected = "phase regressed")]
    fn set_phase_regression_panics() {
        let model = StepModel::default();
        let mut buf = WriteBuffer::new(&model, crate::writer::SchemaTarget::Universal);
        buf.set_phase(EmitPhase::Topology);
        buf.set_phase(EmitPhase::Geometry);
    }
}
