//! Pass orchestration for geometry, topology and assembly conversion.

use super::{ReaderContext, has_all_parts};
use crate::entities::{ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind};
use crate::parser::entity::{EntityGraph, RawEntity};

impl ReaderContext {
    /// Pass 0: unit context. Runs before all geometry passes so that
    /// `model.units` is known by the time downstream code inspects it.
    pub(super) fn run_unit_pass(&mut self, graph: &EntityGraph) {
        // Pass 0-1: SI_UNIT- / CONVERSION_BASED_UNIT-bearing leaves
        // (LENGTH / PLANE_ANGLE / SOLID_ANGLE flavour). Each leaf is its
        // own ComplexEntityHandler keyed on the kind-specific part.
        self.dispatch_registry(graph, PassLevel::Pass0Leaf);

        // Pass 0-1b: UNCERTAINTY_MEASURE_WITH_UNIT (simple entity that
        // depends on Pass 0-1's length_unit_map).
        self.dispatch_registry(graph, PassLevel::Pass0Uncertainty);

        // Pass 0-2: GLOBAL_UNIT_ASSIGNED_CONTEXT.
        self.dispatch_registry(graph, PassLevel::Pass0Context);
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

        // Pass 4-3: SURFACE_CURVE / SEAM_CURVE — alias to curve_3d so edges
        // that reference these see the underlying 3D curve. Must come after
        // 4-1 and 4-2 (curve_3d usually resolves to a simple or rational curve).
        self.dispatch_registry(graph, PassLevel::Pass4_3SurfaceCurve);

        // Pass 4-3c: TRIMMED_CURVE / COMPOSITE_CURVE_SEGMENT (independent)
        // → COMPOSITE_CURVE. Must come after Pass 4-1, 4-2, 4-3 — any of
        // those simple/rational/surface-curve forms may appear as the basis
        // or parent curve.
        self.dispatch_registry(graph, PassLevel::Pass4_3cTrimSeg);
        self.dispatch_registry(graph, PassLevel::Pass4_3cComp);

        // Pass 4-3b: resolve pcurves on each SURFACE_CURVE / SEAM_CURVE.
        // Requires access to `EntityGraph` (to traverse the PCURVE →
        // DEFINITIONAL_REPRESENTATION → 2D curve chain), so this runs as a
        // post-pass outside the dispatch macro. Pass 4a must have already
        // populated `curve_2d_map` and `surface_map`.
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
                crate::entities::geometry::surface_curve::collect_surface_curve_pcurves(
                    self, id, attrs, graph,
                );
            }
        }

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

        // Pass 6-8: NEXT_ASSEMBLY_USAGE_OCCURRENCE — push Instances into
        // parent products' Group content.
        self.dispatch_registry(graph, PassLevel::Pass6Nauo);

        // Pass 7: visualization (passive tree-inline). Depends on Pass 5 / 6 —
        // STYLED_ITEM.item resolution needs solid_map / curve_map / point_map
        // to be populated. Each sub-pass clones from the prior sub-pass's
        // map so the final IR is a fully inlined tree.
        self.dispatch_registry(graph, PassLevel::Pass7Colour);
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
        self.dispatch_registry(graph, PassLevel::Pass7Assignment);
        self.dispatch_registry(graph, PassLevel::Pass7StyledItem);
        self.dispatch_registry(graph, PassLevel::Pass7Mdgpr);

        // Pass 8: PMI scaffolding — SHAPE_ASPECT entries that anchor
        // future Tolerance / Datum / GD&T work. Runs before the property
        // converters so that a future Pattern B PD pass can look up
        // SHAPE_ASPECT ids when resolving its target ref.
        self.dispatch_registry(graph, PassLevel::Pass8ShapeAspect);

        // Pass 8: properties — user-defined attribute chain
        // (PROPERTY_DEFINITION + REPRESENTATION + PROPERTY_DEFINITION_REPRESENTATION).
        // Depends on Pass 0 (unit ctx) and Pass 6 (`pdef_to_product` for
        // resolving the PD's target).
        self.dispatch_registry(graph, PassLevel::Pass8Measure);
        self.dispatch_registry(graph, PassLevel::Pass8PropertyDef);
        // PDR walks the bound REPRESENTATION through `graph` because the
        // generic REPRESENTATION name conflicts with MDGPR / SR.
        self.dispatch_registry(graph, PassLevel::Pass8Pdr);
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
            (
                ReadKind::Complex {
                    required_parts,
                    read,
                },
                RawEntity::Complex { parts, .. },
            ) if has_all_parts(parts, required_parts) => {
                if let Err(e) = read(self, id, parts, graph) {
                    self.warnings.push(e);
                }
            }
            _ => {}
        }
    }
}

/// One-shot validator: flags pairs of complex handlers in the same pass
/// whose `required_parts` sets are subset-related — such a pair would
/// always double-fire on entities matching the larger set, producing
/// duplicate IR pushes. Same-name simple handlers are *not* flagged:
/// 3D/2D sister handlers (e.g. `CARTESIAN_POINT`, `DIRECTION`) share a
/// name by design and self-discriminate by coordinate count.
///
/// Two complex handlers with disjoint required-parts can still both match
/// a non-standard entity that carries every marker; that case is
/// entity-level and outside this static check.
fn validate_registry_no_ambiguity() {
    use std::sync::OnceLock;
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        for (i, a) in ENTITY_HANDLERS.iter().enumerate() {
            for b in ENTITY_HANDLERS.iter().skip(i + 1) {
                if a.pass_level != b.pass_level {
                    continue;
                }
                if let (
                    ReadKind::Complex {
                        required_parts: ra, ..
                    },
                    ReadKind::Complex {
                        required_parts: rb, ..
                    },
                ) = (&a.kind, &b.kind)
                {
                    let a_in_b = ra.iter().all(|p| rb.contains(p));
                    let b_in_a = rb.iter().all(|p| ra.contains(p));
                    assert!(
                        !(a_in_b || b_in_a),
                        "ambiguous complex handlers in pass: {ra:?} vs {rb:?}",
                    );
                }
            }
        }
    });
}
