//! Pass orchestration for geometry, topology and assembly conversion.

use std::collections::{BTreeSet, HashMap};

use super::ReaderContext;
use crate::entities::{ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

/// Pass levels whose handlers read 2D (pcurve-subtree) geometry. Used by the
/// topological dispatch to decide whether the pcurve-subtree skip applies
/// (3D handlers skip pcurve-subtree entities; 2D handlers self-discriminate).
fn is_2d_pass(pass_level: PassLevel) -> bool {
    matches!(
        pass_level,
        PassLevel::Pass4aPoint
            | PassLevel::Pass4aVector
            | PassLevel::Pass4aCurve
            | PassLevel::Pass4aRational
    )
}

/// Name → handler index for topological dispatch (avoids scanning all
/// `ENTITY_HANDLERS` per entity). Simple handlers key on entity name; complex
/// handlers are matched by `has_all_parts` so they share one bucket.
struct TopoIndex {
    simple: HashMap<&'static str, Vec<&'static EntityHandlerEntry>>,
    complex: Vec<&'static EntityHandlerEntry>,
}

fn build_topo_index() -> TopoIndex {
    let mut simple: HashMap<&'static str, Vec<&'static EntityHandlerEntry>> = HashMap::new();
    let mut complex: Vec<&'static EntityHandlerEntry> = Vec::new();
    for entry in ENTITY_HANDLERS {
        match entry.kind {
            ReadKind::Simple { .. } => simple.entry(entry.name).or_default().push(entry),
            ReadKind::Complex { .. } => complex.push(entry),
        }
    }
    TopoIndex { simple, complex }
}

/// Collect the **direct** `#N` references an entity makes in its own
/// attributes (recursing lists / typed params, NOT following the targets).
/// These are the dependency edges for the topological order.
fn direct_refs(ent: &RawEntity, out: &mut Vec<u64>) {
    match ent {
        RawEntity::Simple { attributes, .. } => {
            for a in attributes {
                direct_refs_attr(a, out);
            }
        }
        RawEntity::Complex { parts, .. } => {
            for p in parts {
                for a in &p.attributes {
                    direct_refs_attr(a, out);
                }
            }
        }
    }
}

fn direct_refs_attr(a: &Attribute, out: &mut Vec<u64>) {
    match a {
        Attribute::EntityRef(n) => out.push(*n),
        Attribute::List(items) => {
            for i in items {
                direct_refs_attr(i, out);
            }
        }
        Attribute::Typed { value, .. } => direct_refs_attr(value, out),
        _ => {}
    }
}

/// Topological order of the DATA-section instances (dependencies first) via
/// Kahn's algorithm with a `#N` tie-break (deterministic). Returns the order
/// of all *acyclic* nodes; nodes in a reference cycle never reach in-degree 0
/// and are **excluded** (the caller flags them — see the error-on-cycle
/// policy). The corpus is a verified DAG, so this is purely defensive.
fn topo_order(graph: &EntityGraph) -> Vec<u64> {
    let n = graph.entities.len();
    let mut in_degree: HashMap<u64, usize> = HashMap::with_capacity(n);
    let mut dependents: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut refs = Vec::new();
    for (&id, ent) in &graph.entities {
        refs.clear();
        direct_refs(ent, &mut refs);
        refs.sort_unstable();
        refs.dedup();
        let mut deg = 0usize;
        for &t in &refs {
            // Edge t -> id (t must precede id). Self-refs and refs to absent
            // instances are kept in the degree so a self-loop is treated as a
            // cycle (node excluded) rather than silently processed.
            if graph.entities.contains_key(&t) {
                dependents.entry(t).or_default().push(id);
                deg += 1;
            }
        }
        in_degree.insert(id, deg);
    }
    let mut ready: BTreeSet<u64> = in_degree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut order = Vec::with_capacity(n);
    while let Some(&id) = ready.iter().next() {
        ready.remove(&id);
        order.push(id);
        if let Some(deps) = dependents.get(&id) {
            for &d in deps {
                if let Some(e) = in_degree.get_mut(&d) {
                    *e -= 1;
                    if *e == 0 {
                        ready.insert(d);
                    }
                }
            }
        }
    }
    order
}

impl ReaderContext {
    /// Pass 0: unit context. Runs before all geometry passes so that
    /// `model.units` is known by the time downstream code inspects it.
    pub(super) fn run_unit_pass(&mut self, graph: &EntityGraph) {
        // Pass 0-1: SI_UNIT- / CONVERSION_BASED_UNIT-bearing leaves
        // (LENGTH / PLANE_ANGLE / SOLID_ANGLE flavour). Each leaf is its
        // own ComplexEntityHandler keyed on the kind-specific part.
        // DIMENSIONAL_EXPONENTS (phase dim-exp-arena-a) — leaf entity that
        // NAMED_UNIT subtypes will eventually reference via `dimensions`.
        // Runs before Pass0Leaf so dim_exp_id_map is populated.
        self.dispatch_registry(graph, PassLevel::Pass0DimExp);
        self.dispatch_registry(graph, PassLevel::Pass0Leaf);

        // units-2: every CBU outer registered with `cbu_base = None` during
        // Pass0Leaf (the base SI may not have been processed yet — entity-id
        // order is fixture-dependent). Walk the recorded `outer → MWU` map,
        // resolve each MWU's `unit_component` via the graph, and mutate the
        // outer's flavor entry to point at the base's `NamedUnitId`.
        self.backfill_cbu_base(graph);

        // Pass 0-1b: UNCERTAINTY_MEASURE_WITH_UNIT (simple entity that
        // depends on Pass 0-1's length_unit_map).
        self.dispatch_registry(graph, PassLevel::Pass0Uncertainty);

        // Pass 0-2: GLOBAL_UNIT_ASSIGNED_CONTEXT.
        self.dispatch_registry(graph, PassLevel::Pass0Context);

        // Pass 0-3 (units-1): MEASURE_WITH_UNIT subtype family +
        // MASS_UNIT + DERIVED_UNIT_ELEMENT. Depends on `named_unit_id_map`
        // populated by Pass 0-1 unit-leaf handlers.
        self.dispatch_registry(graph, PassLevel::Pass0MwuDue);

        // Pass 0-4 (units-1b): DERIVED_UNIT — depends on `due_id_map`
        // populated by Pass 0-3.
        self.dispatch_registry(graph, PassLevel::Pass0Du);
    }

    /// For each CBU outer recorded in `cbu_outer_to_mwu`, look up its
    /// conversion-factor MWU in `graph`, extract `unit_component` to get
    /// the base SI's `entity_id`, then mutate the outer's flavor entry in
    /// `named_units_arena` to set `cbu_base = Some(base_NamedUnitId)`.
    fn backfill_cbu_base(&mut self, graph: &EntityGraph) {
        use crate::ir::units::NamedUnit;
        let pairs: Vec<(u64, u64)> = self
            .cbu_outer_to_mwu
            .iter()
            .map(|(&o, &m)| (o, m))
            .collect();
        for (outer_id, mwu_id) in pairs {
            let base_entity = match graph.entities.get(&mwu_id) {
                Some(RawEntity::Simple { attributes, .. }) => {
                    attributes.iter().find_map(|a| match a {
                        // Typed wrapper: MEASURE_TYPE(real). Skip — that's the value.
                        crate::parser::entity::Attribute::EntityRef(e) => Some(*e),
                        _ => None,
                    })
                }
                _ => None,
            };
            let Some(base_entity_id) = base_entity else {
                continue;
            };
            let Some(&base_nuid) = self.named_unit_id_map.get(&base_entity_id) else {
                continue;
            };
            let Some(&outer_nuid) = self.named_unit_id_map.get(&outer_id) else {
                continue;
            };
            match &mut self.named_units_arena.items[outer_nuid.0 as usize] {
                NamedUnit::Length(f) => f.cbu_base = Some(base_nuid),
                NamedUnit::PlaneAngle(f) => f.cbu_base = Some(base_nuid),
                NamedUnit::Mass(f) => f.cbu_base = Some(base_nuid),
                // SolidAngle and Ratio have no CBU variant in corpus.
                NamedUnit::SolidAngle(_) | NamedUnit::Ratio(_) => {}
            }
        }
    }

    pub(super) fn run_geometry_passes(&mut self, graph: &EntityGraph) {
        // Pass 1: points and directions (CARTESIAN_POINT + DIRECTION).
        self.dispatch_registry(graph, PassLevel::Pass1);
        // Pass 2: vectors (depend on directions).
        self.dispatch_registry(graph, PassLevel::Pass2);
        // Pass 3: axis placements (depend on points + directions).
        self.dispatch_registry(graph, PassLevel::Pass3);
        // Pass 4-1: simple leaf curves and surfaces (mutually independent).
        self.dispatch_registry(graph, PassLevel::Pass4Leaf);

        // Pass 4-2: complex rational B-spline curves and surfaces. Both
        // ComplexEntityHandler impls live under
        // `entities::geometry::{rational_bspline_curve, rational_bspline_surface}`.
        self.dispatch_registry(graph, PassLevel::Pass4Rational);

        // Pass 4a: PCURVE parametric-space 2D geometry. Runs before
        // Pass 4-3 so the PCURVE resolver can look up 2D curves it
        // references. Plan 5.5 wires every Pass 4a sub-pass through
        // `dispatch_registry_2d` (handlers in `entities/geometry/*_2d.rs`).
        // 4a-1: 2D points / directions.
        self.dispatch_registry_2d(graph, PassLevel::Pass4aPoint);
        // 4a-2: 2D vectors + axis2_placement_2d.
        self.dispatch_registry_2d(graph, PassLevel::Pass4aVector);
        // 4a-3: 2D curves.
        self.dispatch_registry_2d(graph, PassLevel::Pass4aCurve);
        // 4a-4: 2D rational B-spline curves (complex entities).
        self.dispatch_registry_2d(graph, PassLevel::Pass4aRational);
        // 4-pe: PLANAR_EXTENT / PLANAR_BOX — after the 2D block so a
        // PLANAR_BOX placement resolves against either placement map.
        self.dispatch_registry(graph, PassLevel::Pass4PlanarExtent);

        // Pass 4-3c: TRIMMED_CURVE / COMPOSITE_CURVE_SEGMENT (independent)
        // → COMPOSITE_CURVE. Must come after Pass 4-1, 4-2 — those
        // simple/rational forms appear as the basis or parent curve.
        self.dispatch_registry(graph, PassLevel::Pass4_3cTrimSeg);
        self.dispatch_registry(graph, PassLevel::Pass4_3cComp);

        // Pass 4-3: SURFACE_CURVE / SEAM_CURVE — alias to curve_3d so edges
        // that reference these see the underlying 3D curve. Runs after the
        // 4-3c trimmed/composite pass because `curve_3d` can itself be a
        // TRIMMED_CURVE (grabcad exports); resolving it requires those
        // already captured.
        self.dispatch_registry(graph, PassLevel::Pass4_3SurfaceCurve);

        // Pass 4-4A: derived surfaces that wrap a curve (swept curve / axis
        // of revolution / extrusion vector). Single sweep — no dependency
        // on other derived surfaces.
        self.dispatch_registry(graph, PassLevel::Pass4_4Swept);

        // Pass 4-4B: OFFSET_SURFACE — wraps another surface as its basis.
        // When that basis is itself an OFFSET or a 4-4A derived surface that
        // comes later in entity-id order, a single sweep fails to resolve.
        // `dispatch_registry_until_fixpoint` repeats `dispatch_registry`
        // for `Pass4_4Offset` until the surfaces arena stops growing,
        // discarding transient missing-basis warnings between rounds so
        // only the final no-progress round's warnings (= genuine errors)
        // remain.
        self.dispatch_registry_until_fixpoint(graph, PassLevel::Pass4_4Offset, |ctx| {
            ctx.geometry.surfaces.len()
        });

        // Pass 4-3b: resolve pcurves on each SURFACE_CURVE / SEAM_CURVE.
        // Requires access to `EntityGraph` (to traverse the PCURVE →
        // DEFINITIONAL_REPRESENTATION → 2D curve chain), so this runs as a
        // post-pass outside the dispatch macro. Runs after every Pass 4
        // surface stage so `surface_map` covers Pass4_4 entries
        // (`SURFACE_OF_REVOLUTION` / `OFFSET_SURFACE` etc.) that PCURVE
        // basis_surface refs may target.
        for (&id, entity) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            let (name, attrs) = match entity {
                RawEntity::Simple {
                    name, attributes, ..
                } => (name.as_str(), attributes.as_slice()),
                RawEntity::Complex { .. } => continue,
            };
            if name == "SURFACE_CURVE" || name == "SEAM_CURVE" {
                crate::entities::geometry::surface_curve::collect_surface_curve(
                    self,
                    id,
                    attrs,
                    graph,
                    name == "SEAM_CURVE",
                );
            }
        }
    }

    pub(super) fn run_topology_passes(&mut self, graph: &EntityGraph) {
        // Plan 4 wired every Pass 5-N step through `dispatch_registry`,
        // so the bespoke `run_pass!` macro that used to live here is gone.
        // Pass 5-1: vertices
        self.dispatch_registry(graph, PassLevel::Pass5Vertex);
        // Pass 5-2: edges (depend on vertices + curves)
        self.dispatch_registry(graph, PassLevel::Pass5Edge);
        // Pass 5-3: oriented edges (intermediate, depend on edges)
        self.dispatch_registry(graph, PassLevel::Pass5OrientedEdge);
        // Pass 5-4: edge loops + vertex loops (intermediate, depend on
        // oriented edges / vertices). Both go in the same PassLevel since
        // they don't depend on each other.
        self.dispatch_registry(graph, PassLevel::Pass5EdgeLoop);
        // Pass 5-5: face bounds / outer bounds (depend on edge loops)
        self.dispatch_registry(graph, PassLevel::Pass5FaceBound);
        // Pass 5-6: faces (ADVANCED_FACE + FACE_SURFACE, depend on face bounds + surfaces)
        self.dispatch_registry(graph, PassLevel::Pass5Face);
        // Pass 5-7a: shells (depend on faces)
        self.dispatch_registry(graph, PassLevel::Pass5Shell);
        // Pass 5-7b: oriented shells (depend on CLOSED_SHELL already in
        // the arena; kept separate because ORIENTED_CLOSED_SHELL entities
        // may precede their referenced CLOSED_SHELL in #N order).
        self.dispatch_registry(graph, PassLevel::Pass5OrientedShell);
        // Pass 5-8: solids (depend on shells + oriented shells)
        self.dispatch_registry(graph, PassLevel::Pass5Solid);
    }

    /// Pass 6: assembly/product graph (Phase A — PRODUCT chain + shape
    /// classification; Phase B adds instances, transforms, tree root).
    #[allow(clippy::too_many_lines)]
    pub(super) fn run_assembly_passes(&mut self, graph: &EntityGraph) {
        // Every Pass 6 / 7 / 8 sub-pass dispatches through
        // `dispatch_registry`. Graph-aware handlers (CDSR, PDS classify,
        // PDR) receive `&EntityGraph` through the unified trait signature.

        // Pass 6-1: PRODUCT → Arena<Product> + product_arena_map
        self.dispatch_registry(graph, PassLevel::Pass6Product);
        // Pass 6-1b: PRODUCT_CATEGORY chain. Sub-pass a populates pc_meta_map
        // and prpc_meta_map (PRPC also attaches the kind half to each
        // Product); sub-pass b consumes both maps to fill in the PC root.
        self.dispatch_registry(graph, PassLevel::Pass6ProductCategory);
        self.dispatch_registry(graph, PassLevel::Pass6ProductCategoryRel);
        // Pass 6-2: PRODUCT_DEFINITION_FORMATION [+ _WITH_SPECIFIED_SOURCE].
        // Two distinct handlers so the subtype can flip the
        // formation_with_source loyalty flag on the referenced Product.
        self.dispatch_registry(graph, PassLevel::Pass6PdefFormation);
        // Pass 6-3: PRODUCT_DEFINITION → pdef_to_product
        self.dispatch_registry(graph, PassLevel::Pass6Pdef);
        // Pass 6-4: SBSM (surface body shell list) — must precede MSSR.
        self.dispatch_registry(graph, PassLevel::Pass6Sbsm);
        // Pass 6-4a: ABSR + MSSR + plain SR (shape representations).
        // `dispatch_registry` exact-matches entity names; ABSR/MSSR subtypes
        // carry distinct concrete names in STEP files so the plain
        // `SHAPE_REPRESENTATION` handler catches only the bare type used by
        // Group products and the outer wrapper of Fusion 360 / CATIA
        // indirect-SR chains.
        self.dispatch_registry(graph, PassLevel::Pass6ShapeRep);
        // Pass 6-4f: GEOMETRIC_(CURVE_)SET — must precede GBWSR/GBSSR so the
        // wireframe converters can look up the curve-set payload.
        self.dispatch_registry(graph, PassLevel::Pass6CurveSet);
        // Pass 6-4g: GBWSR / GBSSR — wraps GEOMETRIC_(CURVE_)SETs into a
        // shape representation. Both forms reach the same IR variant
        // (`ProductContent::Wireframe`) with a `repr_kind` flag preserving
        // the source spelling.
        self.dispatch_registry(graph, PassLevel::Pass6Gbsr);
        // Pass 6-4b: simple SHAPE_REPRESENTATION_RELATIONSHIP (indirection
        // from plain SR to ABSR / MSSR / wireframe). Must run after the
        // shape-representation passes above so the is-target lookup sees
        // populated maps.
        self.dispatch_registry(graph, PassLevel::Pass6SrRel);

        // Pre-Pass 6-5: classify PRODUCT_DEFINITION_SHAPE entities by whether
        // their `definition` target is a PRODUCT_DEFINITION (product-owned)
        // versus a NAUO (instance-owned — Phase B).
        self.dispatch_registry(graph, PassLevel::Pass6PdsClassify);

        // Pass 6-5: SHAPE_DEFINITION_REPRESENTATION — classify each product
        // as Solid(SolidId) or leave as Group(empty).
        self.dispatch_registry(graph, PassLevel::Pass6Sdr);

        // Pass 6-6: ITEM_DEFINED_TRANSFORMATION — build transform_map.
        self.dispatch_registry(graph, PassLevel::Pass6Idt);

        // Pass 6-7: CONTEXT_DEPENDENT_SHAPE_REPRESENTATION — bind each
        // NAUO to a Transform3d via the RR-complex sub-entity.
        self.dispatch_registry(graph, PassLevel::Pass6Cdsr);

        // REPRESENTATION_MAP then MAPPED_ITEM (phase mapped-item). Two
        // passes: a MAPPED_ITEM's `#N` may precede its REPRESENTATION_MAP,
        // so `mapping_source` only resolves once every map is in the arena.
        self.dispatch_registry(graph, PassLevel::Pass6RepresentationMap);
        self.dispatch_registry(graph, PassLevel::Pass6MappedItem);

        // COORDINATES_LIST then COMPLEX_TRIANGULATED_FACE (phase tessellation).
        // Two passes: a CTF's `#N` may precede its COORDINATES_LIST.
        self.dispatch_registry(graph, PassLevel::Pass6CoordinatesList);
        self.dispatch_registry(graph, PassLevel::Pass6ComplexTriangulatedFace);
        self.dispatch_registry(graph, PassLevel::Pass6TessellatedGeometricSet);
        // SYMBOL_TARGET — geometric_representation_item SUBTYPE resolving
        // `placement` through `placement_map`. DEFINED_SYMBOL (Pass8) runs
        // later so its `target` can resolve through `symbol_target_id_map`.
        self.dispatch_registry(graph, PassLevel::Pass6SymbolTarget);

        // Pass 6-8: NEXT_ASSEMBLY_USAGE_OCCURRENCE — push Instances into
        // parent products' Group content.
        self.dispatch_registry(graph, PassLevel::Pass6Nauo);

        // Pass 7: visualization (passive tree-inline). Depends on Pass 5 / 6 —
        // STYLED_ITEM.item resolution needs solid_map / curve_map / point_map
        // to be populated. Each sub-pass clones from the prior sub-pass's
        // map so the final IR is a fully inlined tree.
        self.dispatch_registry(graph, PassLevel::Pass7Colour);
        // SYMBOL_COLOUR — depends on Pass7Colour viz_colour_id_map.
        self.dispatch_registry(graph, PassLevel::Pass7SymbolColour);
        // SYMBOL_STYLE — depends on symbol_colour_id_map.
        self.dispatch_registry(graph, PassLevel::Pass7SymbolStyle);
        // POINT_STYLE — depends on PreDefinedMarker / Colour / MWU id maps.
        self.dispatch_registry(graph, PassLevel::Pass7PointStyle);
        // TEXT_STYLE_FOR_DEFINED_FONT — same timing as SymbolColour.
        self.dispatch_registry(graph, PassLevel::Pass7TextStyleForDefinedFont);
        // TEXT_STYLE_WITH_BOX_CHARACTERISTICS — depends on
        // `text_style_for_defined_font_id_map` for the character_appearance ref.
        self.dispatch_registry(graph, PassLevel::Pass7TextStyleBox);
        self.dispatch_registry(graph, PassLevel::Pass7FillColour);
        self.dispatch_registry(graph, PassLevel::Pass7FillArea);
        // SSFA + SSRWP both populate viz_sss_entry_map so a SURFACE_SIDE_STYLE
        // referencing either entity type resolves uniformly. Transparent must
        // run before SSRWP so the rendering converter can resolve property refs.
        self.dispatch_registry(graph, PassLevel::Pass7SurfaceFill);
        self.dispatch_registry(graph, PassLevel::Pass7Transparent);
        self.dispatch_registry(graph, PassLevel::Pass7Rendering);
        self.dispatch_registry(graph, PassLevel::Pass7SurfaceSide);
        self.dispatch_registry(graph, PassLevel::Pass7Usage);
        // CURVE_STYLE chain — must run before PSA so the assignment reader
        // can dispatch curve-style refs to PsaStyle::Curve(...).
        self.dispatch_registry(graph, PassLevel::Pass7CurveStyle);
        // SURFACE_STYLE_BOUNDARY — `founded_item` SUBTYPE whose
        // `style_of_boundary` SELECT resolves through curve_style /
        // surface_style_rendering id maps just filled.
        self.dispatch_registry(graph, PassLevel::Pass7SurfaceStyleBoundary);
        // SURFACE_STYLE_PARAMETER_LINE — same dependencies as SSB.
        self.dispatch_registry(graph, PassLevel::Pass7SurfaceStyleParameterLine);
        self.dispatch_registry(graph, PassLevel::Pass7Assignment);
        self.dispatch_registry(graph, PassLevel::Pass7StyledItem);
        // OVER_RIDING_STYLED_ITEM — depends on the styled_item arena
        // populated by Pass7StyledItem (its over_ridden_style ref must
        // resolve to an existing entry).
        self.dispatch_registry(graph, PassLevel::Pass7OverRiding);
        // CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM — same dependencies as
        // Pass7OverRiding (over_ridden_style requires the styled_item arena
        // populated; item / style_context route through
        // resolve_representation_item_ref).
        self.dispatch_registry(graph, PassLevel::Pass7ContextDependent);
        self.dispatch_registry(graph, PassLevel::Pass7Mdgpr);
        // PRESENTATION_LAYER_ASSIGNMENT — top-level grouping of STYLED_ITEM
        // entries into named display layers. Runs after the styled-item
        // arena is populated by Pass7OverRiding so `assigned_items` refs
        // resolve to existing arena ids.
        self.dispatch_registry(graph, PassLevel::Pass7Pla);
        // LEADER_CURVE (phase annotation-curve-leader) — fills
        // `annotation_curve_occurrence_id_map` consumed by
        // `Pass7AnnotationPlane` for TERMINATOR_SYMBOL / LEADER_TERMINATOR
        // `annotated_curve` resolution.
        self.dispatch_registry(graph, PassLevel::Pass7AnnotationCurve);
        // ANNOTATION_PLANE (phase annotation-plane) — after Pass7Assignment
        // so `styles` refs resolve through `viz_psa_id_map`.
        self.dispatch_registry(graph, PassLevel::Pass7AnnotationPlane);
        // DRAUGHTING_CALLOUT + LEADER_DIRECTED_CALLOUT (phase
        // draughting-callout) — depends on the annotation-occurrence /
        // annotation-curve-occurrence id maps for `contents` SELECT
        // resolution.
        self.dispatch_registry(graph, PassLevel::Pass7DraughtingCallout);
        // DRAUGHTING_CALLOUT_RELATIONSHIP — depends on the
        // draughting_callout id map filled in the previous pass.
        self.dispatch_registry(graph, PassLevel::Pass8DraughtingCalloutRelationship);
        // plm Date/Time chain — leaves (Pass9PlmDateLeaves: CALENDAR_DATE,
        // COORDINATED_UNIVERSAL_TIME_OFFSET, DATE_TIME_ROLE) → LOCAL_TIME
        // (Pass9PlmLocalTime, UTC dep) → DATE_AND_TIME (Pass9PlmDateAndTime,
        // date + local_time dep).
        self.dispatch_registry(graph, PassLevel::Pass9PlmDateLeaves);
        self.dispatch_registry(graph, PassLevel::Pass9PlmLocalTime);
        self.dispatch_registry(graph, PassLevel::Pass9PlmDateAndTime);
        // Date-and-time assignment — references DateAndTime + DateTimeRole
        // arenas + the assembly product chain via `items` SELECT.
        self.dispatch_registry(graph, PassLevel::Pass9PlmDta);
        // Person/Org cluster leaves -> PersonAndOrganization (pairing) ->
        // PersonAndOrganizationAssignment.
        self.dispatch_registry(graph, PassLevel::Pass9PlmPoLeaves);
        self.dispatch_registry(graph, PassLevel::Pass9PlmPersonAndOrganization);
        self.dispatch_registry(graph, PassLevel::Pass9PlmPoa);
        // Approval cluster — leaves (status / role) -> Approval (status ref)
        // -> linkers (date-time / person-organization).
        self.dispatch_registry(graph, PassLevel::Pass9PlmApprovalLeaves);
        self.dispatch_registry(graph, PassLevel::Pass9PlmApproval);
        self.dispatch_registry(graph, PassLevel::Pass9PlmApprovalLinkers);
        // Approval assignments — top-level (no consumers); reference
        // Approval + the assembly product chain via the items SELECT.
        self.dispatch_registry(graph, PassLevel::Pass9PlmAa);
        // Security cluster — level leaf -> classification -> assignments.
        self.dispatch_registry(graph, PassLevel::Pass9PlmSecLevel);
        self.dispatch_registry(graph, PassLevel::Pass9PlmSecClass);
        self.dispatch_registry(graph, PassLevel::Pass9PlmSca);
        // Identification cluster — role + external_source leaves -> assignments.
        self.dispatch_registry(graph, PassLevel::Pass9PlmIdLeaves);
        self.dispatch_registry(graph, PassLevel::Pass9PlmIa);
        // Document cluster — type leaf -> document arena -> linkers.
        self.dispatch_registry(graph, PassLevel::Pass9PlmDocType);
        self.dispatch_registry(graph, PassLevel::Pass9PlmDocument);
        self.dispatch_registry(graph, PassLevel::Pass9PlmDocLinkers);
        // Group cluster — leaf -> assignment.
        self.dispatch_registry(graph, PassLevel::Pass9PlmGroup);
        self.dispatch_registry(graph, PassLevel::Pass9PlmGa);
        // Role cluster — ObjectRole leaf -> RoleAssociation.
        self.dispatch_registry(graph, PassLevel::Pass9PlmObjectRole);
        self.dispatch_registry(graph, PassLevel::Pass9PlmRoleAssoc);
        // Address cluster — ADDRESS (Itself) + PERSONAL_ADDRESS (refs Person).
        self.dispatch_registry(graph, PassLevel::Pass9PlmAddress);
        // Application cluster — AC leaf -> APD (refs AC).
        self.dispatch_registry(graph, PassLevel::Pass9PlmAppContext);
        self.dispatch_registry(graph, PassLevel::Pass9PlmAppProtocol);
        // ID_ATTRIBUTE / NAME_ATTRIBUTE / DESCRIPTION_ATTRIBUTE are dispatched
        // later (Pass9PlmAttributes, after the Pass 8 PMI + property block):
        // ID_ATTRIBUTE.identified_item can target a SHAPE_ASPECT, whose id map
        // is only filled by Pass8ShapeAspect below.
        // Assembly-product context — PC/MC + PDC/DC (refs AC).
        self.dispatch_registry(graph, PassLevel::Pass9AssemblyContext);
        // PDCA cluster — PDCR leaf -> PDCA (refs PDC + PDEF + PDCR).
        self.dispatch_registry(graph, PassLevel::Pass9PdcRole);
        self.dispatch_registry(graph, PassLevel::Pass9Pdca);

        // Pass 8: PMI scaffolding — SHAPE_ASPECT entries that anchor
        // future Tolerance / Datum / GD&T work. Runs before the property
        // converters so that a future Pattern B PD pass can look up
        // SHAPE_ASPECT ids when resolving its target ref.
        self.dispatch_registry(graph, PassLevel::Pass8ShapeAspect);
        // SHAPE_ASPECT_RELATIONSHIP — resolves both endpoints through the
        // shape_aspect (+ subtype) id maps Pass8ShapeAspect just filled.
        self.dispatch_registry(graph, PassLevel::Pass8ShapeAspectRel);
        // DIMENSIONAL_SIZE — resolves `applies_to` through the same maps.
        self.dispatch_registry(graph, PassLevel::Pass8Dimensional);
        // general_datum_reference — resolves `base` through `datum_id_map`
        // (filled by Pass8ShapeAspect) and `of_shape` through the product chain.
        self.dispatch_registry(graph, PassLevel::Pass8GeneralDatumReference);
        // DATUM_SYSTEM — resolves `constituents` through the
        // general_datum_reference id map Pass8GeneralDatumReference filled.
        self.dispatch_registry(graph, PassLevel::Pass8DatumSystem);
        // VIEW_VOLUME — resolves CARTESIAN_POINT / PLANAR_BOX refs.
        self.dispatch_registry(graph, PassLevel::Pass8ViewVolume);
        // CAMERA_MODEL_D3 — resolves VIEW_VOLUME / AXIS2_PLACEMENT_3D refs.
        self.dispatch_registry(graph, PassLevel::Pass8CameraModel);

        // Pass 8: properties — user-defined attribute chain
        // (PROPERTY_DEFINITION + REPRESENTATION + PROPERTY_DEFINITION_REPRESENTATION).
        // Depends on Pass 0 (unit ctx) and Pass 6 (`pdef_to_product` for
        // resolving the PD's target).
        self.dispatch_registry(graph, PassLevel::Pass8Measure);
        // Re-resolve SHAPE_DIMENSION_REPRESENTATION items deferred from
        // Pass6ShapeRep now that the complex MRI arena entries exist.
        self.resolve_deferred_sdr_items();
        // geometric_tolerance form tolerances — resolve `magnitude` through
        // `mwu_id_map` / `repr_item_id_map` and `toleranced_shape_aspect`
        // through the shape-aspect id maps Pass8ShapeAspect filled.
        self.dispatch_registry(graph, PassLevel::Pass8GeometricTolerance);
        // geometric_tolerance_with_datum_reference — also needs the
        // datum_system id map Pass8DatumSystem filled.
        self.dispatch_registry(graph, PassLevel::Pass8GtWithDatumReference);
        // GEOMETRIC_TOLERANCE_RELATIONSHIP — depends on both GT enum id
        // maps (Pass8GeometricTolerance + Pass8GtWithDatumReference)
        // populated above.
        self.dispatch_registry(graph, PassLevel::Pass8GtRelationship);
        // TOLERANCE_ZONE — shape_aspect subtype resolving `defining_tolerance`
        // through the two geometric_tolerance id maps just filled.
        self.dispatch_registry(graph, PassLevel::Pass8ToleranceZone);
        // PROJECTED_ZONE_DEFINITION — depends on the tolerance_zone id
        // map (Pass8ToleranceZone) + shape-aspect id maps + mwu_id_map.
        self.dispatch_registry(graph, PassLevel::Pass8ProjectedZoneDefinition);
        // MEASURE_QUALIFICATION — depends on qualifier id maps
        // (Pass8ShapeAspect) + mwu_id_map.
        self.dispatch_registry(graph, PassLevel::Pass8MeasureQualification);
        // DIMENSIONAL_CHARACTERISTIC_REPRESENTATION — depends on the
        // dimensional id maps (Pass8Dimensional) + repr_id_map (Pass6).
        self.dispatch_registry(
            graph,
            PassLevel::Pass8DimensionalCharacteristicRepresentation,
        );
        // QUALIFIED_REPRESENTATION_ITEM / VALUE_REPRESENTATION_ITEM —
        // QRI depends on type_qualifier / value_format_type_qualifier id
        // maps (Pass8ShapeAspect). VRI is self-contained.
        self.dispatch_registry(graph, PassLevel::Pass8RepresentationItem);
        // (DRAUGHTING_MODEL REPRESENTATION SHAPE_REPRESENTATION
        // TESSELLATED_SHAPE_REPRESENTATION) complex MI — the PMI
        // geometric-validation draughting model. Must run before the CIWR
        // pass so a CIWR whose `rep` is this draughting model resolves
        // through repr_id_map. Its `items` (annotations / geometry) are
        // already populated by Pass6/Pass7.
        self.dispatch_registry(graph, PassLevel::Pass8DraughtingModelStComplex);
        // DRAUGHTING_MODEL (simple form) — depends on Pass7 ids for items
        // (styled_item, annotation_occurrence, draughting_callout). Runs here,
        // before the CIWR pass, so a CIWR whose `rep` is a simple draughting
        // model resolves through repr_id_map (mirrors the StComplex form
        // above; still before Pass8Dmia).
        self.dispatch_registry(graph, PassLevel::Pass8DraughtingModel);
        // CHARACTERIZED_ITEM_WITHIN_REPRESENTATION — depends on repr_id_map
        // (Pass6 + the two DRAUGHTING_MODEL passes above) + per-type arena id maps.
        self.dispatch_registry(graph, PassLevel::Pass8CharacterizedItemWithinRepresentation);
        // TOLERANCE_VALUE / LIMITS_AND_FITS, then PLUS_MINUS_TOLERANCE which
        // resolves `range` through the former and `toleranced_dimension`
        // through the Pass8Dimensional id maps.
        self.dispatch_registry(graph, PassLevel::Pass8ToleranceValue);
        self.dispatch_registry(graph, PassLevel::Pass8PlusMinusTolerance);
        // GENERAL_PROPERTY before PROPERTY_DEFINITION: a PD's `definition`
        // can be a GENERAL_PROPERTY (general_property member of
        // characterized_definition), so general_property_id_map must be
        // filled first. GENERAL_PROPERTY itself reads only scalars, no PD
        // dependency. GPA (below) still runs after Pass8Pdr.
        self.dispatch_registry(graph, PassLevel::Pass8GeneralProperty);
        self.dispatch_registry(graph, PassLevel::Pass8PropertyDef);
        // PDR walks the bound REPRESENTATION through `graph` because the
        // generic REPRESENTATION name conflicts with MDGPR / SR.
        self.dispatch_registry(graph, PassLevel::Pass8Pdr);
        // GPA (Pass 8-5) resolves `derived_definition` through the property
        // arena built by Pass8Pdr, so it must run after it.
        self.dispatch_registry(graph, PassLevel::Pass8Gpa);
        // ID_ATTRIBUTE / NAME_ATTRIBUTE / DESCRIPTION_ATTRIBUTE —
        // identified_item / named_item / described_item SELECT resolves through
        // shape_aspect (Pass8ShapeAspect) / datum / tolerance_zone /
        // property_definition (Pass8PropertyDef) / plm_group / plm_address /
        // plm_application_context / derived_unit / product_definition id maps,
        // all populated above. Must run after the PMI + property block so a
        // SHAPE_ASPECT-targeting identified_item resolves (previously this ran
        // before Pass8ShapeAspect and every such ID_ATTRIBUTE was dropped).
        self.dispatch_registry(graph, PassLevel::Pass9PlmAttributes);
        // TEXT_LITERAL — depends on placement maps (Pass4) + dptf_id_map
        // (Pass8ShapeAspect, already dispatched above).
        self.dispatch_registry(graph, PassLevel::Pass8TextLiteral);
        // COMPOSITE_TEXT — depends on text_literal_id_map just filled.
        self.dispatch_registry(graph, PassLevel::Pass8CompositeText);
        // CAMERA_USAGE — `representation_map` SUBTYPE whose
        // `mapped_representation` may target a DRAUGHTING_MODEL, so runs
        // after Pass8DraughtingModel populates the DM slot of repr_id_map.
        self.dispatch_registry(graph, PassLevel::Pass8CameraUsage);
        // TESSELLATED_SHAPE_REPRESENTATION — a delayed-emit `representation`
        // variant kept near the arena tail. Writer's push-built
        // `representation_step_ids` aligns with arena ids by index regardless
        // of read order (the DRAUGHTING_MODEL passes resolve earlier above).
        self.dispatch_registry(graph, PassLevel::Pass8TsrRead);
        // CONSTRUCTIVE_GEOMETRY_REPRESENTATION — runs after Pass8TsrRead
        // so the representations arena keeps delayed-emit variants
        // contiguous at the tail (Mdgpr → DM → TSR → CGR).
        self.dispatch_registry(graph, PassLevel::Pass8CgrRead);
        // CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP — depends on
        // `repr_id_map` covering CGR (filled by Pass8CgrRead).
        self.dispatch_registry(graph, PassLevel::Pass8CgrrRead);
        // MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP — RepresentationRelationship
        // SUBTYPE. rep_1/rep_2 resolve through repr_id_map.
        self.dispatch_registry(graph, PassLevel::Pass8MddrRead);
        // ITEM_IDENTIFIED_REPRESENTATION_USAGE — depends on repr_id_map
        // + PMI id maps (shape_aspect / datum / dimensional_size / etc.).
        self.dispatch_registry(graph, PassLevel::Pass8IiruRead);
        // SHAPE_REPRESENTATION_WITH_PARAMETERS — Representation subtype.
        self.dispatch_registry(graph, PassLevel::Pass8SrwpRead);
        // COMPOUND_REPRESENTATION_ITEM — resolves child refs through
        // descriptive_item_map (Pass8Measure) + per-arena representation
        // item id maps. Scheduled last in the Pass8 block.
        self.dispatch_registry(graph, PassLevel::Pass8CompoundRepItem);
        // BOUNDED_PCURVE — `parameter_space_curve` SUBTYPE (orphan).
        // Surface + repr_id_map populated by earlier passes.
        self.dispatch_registry(graph, PassLevel::Pass8BoundedPCurve);
        // BOUNDED_SURFACE_CURVE + INTERSECTION_CURVE — surface_curve
        // SUBTYPEs (corpus 0; round-trip only).
        self.dispatch_registry(graph, PassLevel::Pass8SurfaceCurveSubtypes);
        // DEFINED_SYMBOL — depends on `symbol_target_id_map` (Pass6SymbolTarget)
        // and `viz_pre_defined_symbol_id_map` (Pass7Colour-block).
        self.dispatch_registry(graph, PassLevel::Pass8DefinedSymbol);
        // CAMERA_IMAGE + CAMERA_IMAGE_3D_WITH_SCALE — `mapped_item` SUBTYPE
        // resolving `mapping_source` through `representation_map_id_map`
        // (whose CameraUsage slots Pass8CameraUsage just filled).
        self.dispatch_registry(graph, PassLevel::Pass8CameraImage);
        // DRAUGHTING_MODEL_ITEM_ASSOCIATION — depends on repr_id_map +
        // annotation_occurrence_id_map + draughting_callout_id_map.
        self.dispatch_registry(graph, PassLevel::Pass8Dmia);
        // GEOMETRIC_ITEM_SPECIFIC_USAGE — depends on shape_aspect family
        // id maps + repr_id_map + representation_item arenas.
        self.dispatch_registry(graph, PassLevel::Pass8Gisu);
        // INVISIBILITY — depends on styled_item / repr / draughting_callout
        // id maps.
        self.dispatch_registry(graph, PassLevel::Pass8Invisibility);
        // PRESENTATION_VIEW / PRESENTATION_AREA / PRESENTATION_SET — needs
        // placement / repr_item / context maps populated.
        self.dispatch_registry(graph, PassLevel::Pass8PrCore);
        // AREA_IN_SET + PRESENTATION_SIZE — depends on the pr-core id maps.
        self.dispatch_registry(graph, PassLevel::Pass8PrSize);
        // PRESENTED_ITEM_REPRESENTATION + APPLIED_PRESENTED_ITEM — depends on
        // pr-core id maps + product chain.
        self.dispatch_registry(graph, PassLevel::Pass8PrItem);
    }

    /// Walk every entity in `graph` (id-sorted via `BTreeMap`) and dispatch
    /// every registry entry whose `pass_level` matches and whose
    /// name/required-parts predicate matches the entity. Pool insertion
    /// order across IR arenas therefore tracks source-file id order
    /// regardless of how many handlers contribute to a given pool — adding
    /// a new handler that pushes to an existing pool cannot reorder
    /// pre-existing entries. Multiple handlers may match the same entity
    /// by design (e.g. 3D / 2D `CARTESIAN_POINT` co-exist at `Pass1` and
    /// self-discriminate by coordinate count).
    /// Re-resolve `SHAPE_DIMENSION_REPRESENTATION` items deferred from
    /// `Pass6ShapeRep` (phase measure-arena-1). Runs after `Pass8Measure` so
    /// the complex `MEASURE_REPRESENTATION_ITEM` arena entries (and their
    /// `repr_item_id_map` ids) exist. Refs are resolved in their original
    /// order; unresolved ones drop, matching the prior inline behaviour.
    fn resolve_deferred_sdr_items(&mut self) {
        use crate::entities::visualization::styled_item::resolve_representation_item_ref;
        use crate::ir::shape_rep::Representation;
        let raw = std::mem::take(&mut self.sdr_raw_items);
        for (repr_id, refs) in raw {
            let items: Vec<_> = refs
                .into_iter()
                .filter_map(|r| resolve_representation_item_ref(self, r))
                .collect();
            if let Representation::ShapeDimensionRepresentation(sdr) =
                &mut self.representations[repr_id]
            {
                sdr.items = items;
            }
        }
    }

    fn dispatch_registry(&mut self, graph: &EntityGraph, pass_level: PassLevel) {
        validate_registry_no_ambiguity();
        for (&id, ent) in &graph.entities {
            let is_pcurve = self.pcurve_subtree_ids.contains(&id);
            for entry in ENTITY_HANDLERS {
                if entry.pass_level != pass_level {
                    continue;
                }
                // CARTESIAN_POINT / DIRECTION are coord-count classified
                // (their 2D/3D sister handlers silently skip the wrong
                // dimension), so they do not honour the pcurve-subtree
                // partition: a 2-component instance can legitimately
                // appear at the top level (orphan in IR) and must still
                // reach the 2D arena for round-trip preservation.
                let respect_pcurve = !matches!(entry.name, "CARTESIAN_POINT" | "DIRECTION");
                if respect_pcurve && is_pcurve {
                    continue;
                }
                self.dispatch_one(graph, entry, id, ent);
            }
        }
    }

    /// Repeat [`Self::dispatch_registry`] for `pass_level` until `measure`
    /// stops changing. The first round populates handlers whose dependencies
    /// were already in place; subsequent rounds pick up entities whose basis
    /// was emitted by an earlier round of the same pass. Mirrors the
    /// pre-Plan-7 `OFFSET_SURFACE` loop semantics: warnings produced before
    /// the final no-progress round are discarded so only genuine errors
    /// (basis still missing at fixpoint) reach `self.warnings`.
    #[allow(dead_code)] // wired in Plan 7 stage C6 (OFFSET_SURFACE migration)
    fn dispatch_registry_until_fixpoint<F>(
        &mut self,
        graph: &EntityGraph,
        pass_level: PassLevel,
        measure: F,
    ) where
        F: Fn(&Self) -> usize,
    {
        loop {
            let before = measure(self);
            let warnings_snapshot = self.warnings.len();
            self.dispatch_registry(graph, pass_level);
            if measure(self) == before {
                break;
            }
            self.warnings.truncate(warnings_snapshot);
        }
    }

    /// Like [`dispatch_registry`], but for handlers whose entities live
    /// inside a `PCURVE` `DEFINITIONAL_REPRESENTATION` subtree (2D
    /// geometry). Plan 5.5 introduced this twin entry point so that
    /// shared entity names (`CARTESIAN_POINT`, `DIRECTION`, `LINE`, …)
    /// can have separate 3D / 2D handlers without confusion. Each 2D
    /// handler self-discriminates by coordinate count or by the first
    /// cross-reference and silently returns `Ok(())` for wrong-dimension
    /// or 3D entities — pcurve-subtree filtering is not applied here.
    fn dispatch_registry_2d(&mut self, graph: &EntityGraph, pass_level: PassLevel) {
        validate_registry_no_ambiguity();
        for (&id, ent) in &graph.entities {
            for entry in ENTITY_HANDLERS {
                if entry.pass_level != pass_level {
                    continue;
                }
                self.dispatch_one(graph, entry, id, ent);
            }
        }
    }

    /// Topological dispatch (Stage 2): convert every instance once in
    /// dependency order instead of the hand-ordered pass list. Replicates the
    /// inline post-passes `run_*_passes` performed (`backfill_cbu_base`,
    /// `SURFACE_CURVE` pcurve collection, deferred SDR items) after the loop.
    pub(super) fn run_topo(&mut self, graph: &EntityGraph) {
        let order = topo_order(graph);
        if order.len() < graph.entities.len() {
            // Reference cycle / self-loop: the unprocessed nodes can't be built
            // by the resolve-then-construct reader (chicken-and-egg). Flag as
            // malformed (error-on-cycle policy). Corpus is a verified DAG, so
            // this never triggers in practice.
            let unresolved = graph.entities.len() - order.len();
            self.warnings
                .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                    entity_id: 0,
                    detail: format!(
                        "cyclic reference: {unresolved} instance(s) unprocessable (malformed file)"
                    ),
                });
        }
        // Order-independent seeding of the CBU `conversion_factor` suppression
        // set. In pass mode the CONVERSION_BASED_UNIT is read before its
        // embedded MWU, so reading the CBU populates `cbu_internal_mwu_refs`
        // in time. Under topo the MWU (a dependency) is processed first, so
        // the set must be seeded up front or the MWU duplicates the inline
        // conversion factor the writer re-emits.
        self.prescan_cbu_internal_mwu_refs(graph);
        let index = build_topo_index();
        for id in order {
            let Some(ent) = graph.get(id) else { continue };
            self.dispatch_one_topo(graph, id, ent, &index);
            // Fold the SURFACE_CURVE / SEAM_CURVE pcurve collection in at the
            // entity's topo position (its surfaces / 2D curves are already done).
            if let RawEntity::Simple {
                name, attributes, ..
            } = ent
                && (name == "SURFACE_CURVE" || name == "SEAM_CURVE")
            {
                crate::entities::geometry::surface_curve::collect_surface_curve(
                    self,
                    id,
                    attributes,
                    graph,
                    name == "SEAM_CURVE",
                );
            }
        }
        // Inline post-passes that `run_*_passes` ran mid-sequence — now after
        // the single loop (all producers done; equivalent timing).
        self.backfill_cbu_base(graph);
        self.resolve_deferred_sdr_items();
    }

    /// Seed `cbu_internal_mwu_refs` from every `CONVERSION_BASED_UNIT`'s
    /// `conversion_factor` ref (attr index 1) so the MWU handlers suppress the
    /// embedded duplicate regardless of dispatch order. Mirrors the insert in
    /// `read_conversion_based_unit_body` (which fires for any CBU name,
    /// recognised or not), just hoisted ahead of the topo loop.
    fn prescan_cbu_internal_mwu_refs(&mut self, graph: &EntityGraph) {
        for ent in graph.entities.values() {
            let RawEntity::Complex { parts, .. } = ent else {
                continue;
            };
            for part in parts {
                if part.name == "CONVERSION_BASED_UNIT"
                    && let Some(Attribute::EntityRef(r)) = part.attributes.get(1)
                {
                    self.cbu_internal_mwu_refs.insert(*r);
                }
            }
        }
    }

    /// Dispatch all name-matching handlers for one instance, applying the
    /// pcurve-subtree skip to 3D handlers (2D handlers self-discriminate).
    /// Same matching/skip rules as `dispatch_registry` / `dispatch_registry_2d`,
    /// just driven by the topo loop instead of pass order.
    fn dispatch_one_topo(
        &mut self,
        graph: &EntityGraph,
        id: u64,
        ent: &RawEntity,
        index: &TopoIndex,
    ) {
        let is_pcurve = self.pcurve_subtree_ids.contains(&id);
        let candidates: &[&EntityHandlerEntry] = match ent {
            RawEntity::Simple { name, .. } => {
                index.simple.get(name.as_str()).map_or(&[], Vec::as_slice)
            }
            RawEntity::Complex { .. } => &index.complex,
        };
        for &entry in candidates {
            if !is_2d_pass(entry.pass_level) {
                // 3D handler: honour the pcurve-subtree partition (POINT /
                // DIRECTION self-discriminate by coord count, so exempt).
                let respect_pcurve = !matches!(entry.name, "CARTESIAN_POINT" | "DIRECTION");
                if respect_pcurve && is_pcurve {
                    continue;
                }
            }
            self.dispatch_one(graph, entry, id, ent);
        }
        // Exact-case matching: a complex instance whose part-set matches no
        // handler case at all is dropped — surface it. The check is against the
        // registry (not whether dispatch *ran* a handler) so a pcurve-skipped
        // instance, which still has a matching handler, is not flagged.
        if let RawEntity::Complex { parts, .. } = ent {
            self.warn_unhandled_complex(id, parts);
        }
    }

    /// Try to dispatch a single `(entry, entity)` pair. The handler runs
    /// when name (simple) or required-parts (complex) match; otherwise
    /// nothing happens. Multiple handlers may match the same entity by
    /// design (self-discriminating sister handlers).
    fn dispatch_one(
        &mut self,
        graph: &EntityGraph,
        entry: &EntityHandlerEntry,
        id: u64,
        ent: &RawEntity,
    ) {
        match (&entry.kind, ent) {
            (
                ReadKind::Simple { read },
                RawEntity::Simple {
                    name, attributes, ..
                },
            ) if name == entry.name => {
                if let Err(e) = read(self, id, attributes, graph) {
                    self.warnings.push(e);
                }
            }
            (ReadKind::Complex { cases, read }, RawEntity::Complex { parts, .. })
                if crate::reader::matches_any_case(parts, cases) =>
            {
                if let Err(e) = read(self, id, parts, graph) {
                    self.warnings.push(e);
                }
            }
            _ => {}
        }
    }

    /// Warn (once per distinct part-set per file) that a complex instance's
    /// part-set matches no complex handler case and was dropped. Skipped when
    /// some handler case *does* match (e.g. a pcurve-skipped instance) or the
    /// shape is allow-listed as read indirectly by another handler.
    fn warn_unhandled_complex(&mut self, id: u64, parts: &[RawEntityPart]) {
        let handled = ENTITY_HANDLERS.iter().any(|e| {
            matches!(&e.kind, ReadKind::Complex { cases, .. }
                if crate::reader::matches_any_case(parts, cases))
        });
        if handled {
            return;
        }
        let mut names: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
        names.sort();
        names.dedup();
        let actual: BTreeSet<&str> = names.iter().map(String::as_str).collect();
        if INDIRECTLY_READ_COMPLEX_CASES
            .iter()
            .any(|c| c.iter().copied().collect::<BTreeSet<_>>() == actual)
        {
            return;
        }
        if self.unhandled_complex_seen.insert(names.join("+")) {
            self.warnings
                .push(crate::ir::error::ConvertError::UnhandledComplex {
                    entity_id: id,
                    parts: names,
                });
        }
    }
}

/// Complex part-sets that are *not* dispatched by a registered handler but are
/// read indirectly by another handler walking the graph — so an "unhandled
/// complex" warning would be a false positive. (The RR complex is resolved by
/// `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION`.)
const INDIRECTLY_READ_COMPLEX_CASES: &[&[&str]] = &[&[
    "REPRESENTATION_RELATIONSHIP",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "SHAPE_REPRESENTATION_RELATIONSHIP",
]];

/// One-shot validator: under exact-case matching, asserts no two
/// **distinct-name** complex handlers (across *all* passes) declare a shared
/// exact case — such a pair would both claim the same instance, dropping the
/// loser's parts. Same-name handlers are exempt: they are 2D/3D sisters
/// (e.g. `RATIONAL_B_SPLINE_CURVE`) that deliberately share cases and are
/// disambiguated at dispatch by the pcurve-subtree skip / 2D-vs-3D pass split.
fn validate_registry_no_ambiguity() {
    use std::sync::OnceLock;
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let case_eq =
            |a: &[&str], b: &[&str]| a.len() == b.len() && a.iter().all(|p| b.contains(p));
        for (i, a) in ENTITY_HANDLERS.iter().enumerate() {
            for b in ENTITY_HANDLERS.iter().skip(i + 1) {
                if a.name == b.name {
                    continue; // 2D/3D sisters share cases by design.
                }
                if let (ReadKind::Complex { cases: ca, .. }, ReadKind::Complex { cases: cb, .. }) =
                    (&a.kind, &b.kind)
                {
                    for x in *ca {
                        for y in *cb {
                            assert!(
                                !case_eq(x, y),
                                "ambiguous complex handlers '{}' vs '{}' share exact case {x:?}",
                                a.name,
                                b.name,
                            );
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod topo_tests {
    use super::topo_order;

    fn graph(data: &str) -> crate::parser::EntityGraph {
        let src = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\n\
             FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n{data}\nENDSEC;\n\
             END-ISO-10303-21;\n"
        );
        crate::parse(&src).expect("parse")
    }

    fn pos(order: &[u64], id: u64) -> usize {
        order.iter().position(|&x| x == id).expect("in order")
    }

    #[test]
    fn dependencies_precede_dependents_and_full_coverage() {
        // #1 -> #2,#3 ; #2 -> #4 ; #3 -> #4 ; #4 -> (none)
        let g = graph("#1=A('',#2,#3);\n#2=B('',#4);\n#3=C('',#4);\n#4=D('');");
        let order = topo_order(&g);
        assert_eq!(order.len(), 4, "all acyclic nodes covered");
        assert!(pos(&order, 4) < pos(&order, 2));
        assert!(pos(&order, 4) < pos(&order, 3));
        assert!(pos(&order, 2) < pos(&order, 1));
        assert!(pos(&order, 3) < pos(&order, 1));
        // deterministic #N tie-break: #4 first, then #2 before #3.
        assert_eq!(order, vec![4, 2, 3, 1]);
    }

    #[test]
    fn cycle_nodes_excluded() {
        // #1 <-> #2 cycle, #3 acyclic leaf referenced by #1.
        let g = graph("#1=A('',#2,#3);\n#2=B('',#1);\n#3=C('');");
        let order = topo_order(&g);
        // #3 is acyclic and processable; #1/#2 (cycle) are excluded.
        assert_eq!(order, vec![3], "only the acyclic leaf is ordered");
    }
}
