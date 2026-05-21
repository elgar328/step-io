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
    /// STEP entity id of every emitted curve-font entity (currently
    /// `DRAUGHTING_PRE_DEFINED_CURVE_FONT`), indexed by `CurveFontId.0`.
    /// Consumed by the `CURVE_STYLE` writer.
    pub(crate) curve_font_step_ids: Vec<u64>,
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
    /// STEP entity id of every emitted `GENERAL_PROPERTY`, indexed by
    /// `GeneralPropertyId.0`. Consumed by the GPA emitter for `base_definition`.
    pub(crate) general_property_step_ids: Vec<u64>,
}

impl<'m> WriteBuffer<'m> {
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
            unit_context_ids: Vec::new(),
            representation_step_ids: Vec::new(),
            representation_map_step_ids: Vec::new(),
            tessellated_item_step_ids: Vec::new(),
            unit_leaf_ids: Vec::new(),
            named_unit_step_ids: Vec::new(),
            mwu_step_ids: Vec::new(),
            due_step_ids: Vec::new(),
            derived_unit_step_ids: Vec::new(),
            length_dim_exp_step: None,
            dimensionless_dim_exp_step: None,
            mass_dim_exp_step: None,
            colour_step_ids: Vec::new(),
            curve_font_step_ids: Vec::new(),
            curve_style_step_ids: Vec::new(),
            styled_item_step_ids: Vec::new(),
            psa_step_ids: Vec::new(),
            ssr_step_ids: Vec::new(),
            founded_item_step_ids: Vec::new(),
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
            apd_step_ids: Vec::new(),
            pc_step_ids: Vec::new(),
            pdc_step_ids: Vec::new(),
            product_def_ids: std::collections::HashMap::new(),
            product_def_shape_ids: std::collections::HashMap::new(),
            property_step_ids: Vec::new(),
            general_property_step_ids: Vec::new(),
        }
    }

    pub(crate) fn finish_entities(self) -> Vec<WriterEntity> {
        self.entities
    }

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
        // Pre-emit every geometry representation (ABSR / MSSR / plain SR /
        // GBWSR / GBSSR) in `representations` arena order so re-read assigns
        // identical `RepresentationId`s — the same round-trip-stability
        // mechanism the curve / surface / solid arenas rely on. MDGPR is
        // skipped here and emitted by the visualization pass (it depends on
        // STYLED_ITEMs). No-op for hand/kernel-built IR (arena unpopulated).
        self.emit_representations_pre_pass()?;
        self.emit_product_chain_if_eligible()?;
        self.emit_pmi_if_set();
        // SHAPE_ASPECT_RELATIONSHIP — after emit_pmi_if_set so every
        // shape-aspect-family step-id cache is filled.
        self.emit_shape_aspect_relationships()?;
        self.emit_visualization_if_set()?;
        // REPRESENTATION_MAP + MAPPED_ITEM — after visualization so the
        // `representation_step_ids` cache covers MDGPR slots too.
        self.emit_mapped_items()?;
        // ANNOTATION_PLANE — after visualization so `psa_step_ids` is filled.
        self.emit_annotation_occurrences();
        // INTEGER/REAL_REPRESENTATION_ITEM — orphan value-items, no refs.
        self.emit_numeric_representation_items()?;
        // COORDINATES_LIST + COMPLEX_TRIANGULATED_FACE — orphan tessellation.
        self.emit_tessellation()?;
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
