//! Pass orchestration for geometry, topology and assembly conversion.

use super::assembly::{pdef_shape_to_nauo_ref, pdef_shape_to_pdef_ref};
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

    #[allow(clippy::too_many_lines)]
    pub(super) fn run_geometry_passes(&mut self, graph: &EntityGraph) {
        macro_rules! run_pass {
            ($graph:expr, $ctx:expr, $( $name:literal => $method:ident ),+ $(,)?) => {
                for (&id, entity) in &$graph.entities {
                    if $ctx.pcurve_subtree_ids.contains(&id) { continue; }
                    let (name, attrs) = match entity {
                        RawEntity::Simple { name, attributes, .. } => (name.as_str(), attributes.as_slice()),
                        RawEntity::Complex { .. } => continue,
                    };
                    let result = match name {
                        $( $name => $ctx.$method(id, attrs), )+
                        _ => continue,
                    };
                    if let Err(e) = result {
                        $ctx.warnings.push(e);
                    }
                }
            };
        }

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
        // Loop until the surfaces arena stops growing; discard transient
        // missing-basis warnings after any round that made progress, so only
        // the final no-progress round's warnings (= genuine errors) remain.
        loop {
            let surfaces_before = self.geometry.surfaces.len();
            let warnings_snapshot = self.warnings.len();
            run_pass!(graph, self, "OFFSET_SURFACE" => convert_offset_surface);
            if self.geometry.surfaces.len() == surfaces_before {
                break;
            }
            self.warnings.truncate(warnings_snapshot);
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
        macro_rules! run_pass {
            ($graph:expr, $ctx:expr, $( $name:literal => $method:ident ),+ $(,)?) => {
                for (&id, entity) in &$graph.entities {
                    if $ctx.pcurve_subtree_ids.contains(&id) { continue; }
                    let (name, attrs) = match entity {
                        RawEntity::Simple { name, attributes, .. } => (name.as_str(), attributes.as_slice()),
                        RawEntity::Complex { .. } => continue,
                    };
                    let result = match name {
                        $( $name => $ctx.$method(id, attrs), )+
                        _ => continue,
                    };
                    if let Err(e) = result {
                        $ctx.warnings.push(e);
                    }
                }
            };
        }

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
        //
        // DOMAIN_TBD: PRODUCT_DEFINITION_SHAPE classification 은 convert_*
        // 부재 (graph traversal 만 함) 라 handler trait 부적절. Plan 7+ 에서
        // post-pass helper 추상화 검토 가능.
        for (&id, entity) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            match entity {
                RawEntity::Simple { name, .. } if name == "PRODUCT_DEFINITION_SHAPE" => {
                    // A PDEF_SHAPE points at either a PRODUCT_DEFINITION
                    // (product-describing, Phase A / Pass 6-5) or a NAUO
                    // (instance-describing, Phase B / Pass 6-7). Populate
                    // whichever map applies.
                    if let Some(pdef_ref) = pdef_shape_to_pdef_ref(graph, id) {
                        self.pdef_shape_to_pdef.insert(id, pdef_ref);
                    } else if let Some(nauo_ref) = pdef_shape_to_nauo_ref(graph, id) {
                        self.pdef_shape_to_nauo.insert(id, nauo_ref);
                    }
                }
                _ => {}
            }
        }

        // Pass 6-5: SHAPE_DEFINITION_REPRESENTATION — classify each product
        // as Solid(SolidId) or leave as Group(empty).
        self.dispatch_registry(graph, PassLevel::Pass6Sdr);

        // Pass 6-6: ITEM_DEFINED_TRANSFORMATION — build transform_map.
        self.dispatch_registry(graph, PassLevel::Pass6Idt);

        // Pass 6-7: CONTEXT_DEPENDENT_SHAPE_REPRESENTATION — bind each
        // NAUO to a Transform3d. Custom loop because the converter needs
        // access to `graph` to resolve the RR-complex sub-entity.
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
            if name != "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" {
                continue;
            }
            if let Err(e) = self.convert_context_dependent_shape_representation(id, attrs, graph) {
                self.warnings.push(e);
            }
        }

        // Pass 6-8: NEXT_ASSEMBLY_USAGE_OCCURRENCE — push Instances into
        // parent products' Group content.
        self.dispatch_registry(graph, PassLevel::Pass6Nauo);

        // Pass 7: visualization (passive tree-inline). Depends on Pass 5 / 6 —
        // STYLED_ITEM.item resolution needs solid_map / curve_map / point_map
        // to be populated. Each sub-pass clones from the prior sub-pass's
        // map so the final IR is a fully inlined tree.
        run_pass!(graph, self, "COLOUR_RGB" => convert_colour_rgb);
        run_pass!(graph, self, "FILL_AREA_STYLE_COLOUR" => convert_fill_area_style_colour);
        run_pass!(graph, self, "FILL_AREA_STYLE" => convert_fill_area_style);
        // SSFA + SSRWP both populate viz_sss_entry_map so a SURFACE_SIDE_STYLE
        // referencing either entity type resolves uniformly. Transparent must
        // run before SSRWP so the rendering converter can resolve property refs.
        run_pass!(graph, self, "SURFACE_STYLE_FILL_AREA" => convert_surface_style_fill_area);
        run_pass!(graph, self,
            "SURFACE_STYLE_TRANSPARENT" => convert_surface_style_transparent);
        run_pass!(graph, self,
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES" => convert_surface_style_rendering_with_properties);
        run_pass!(graph, self, "SURFACE_SIDE_STYLE" => convert_surface_side_style);
        run_pass!(graph, self, "SURFACE_STYLE_USAGE" => convert_surface_style_usage);
        run_pass!(graph, self,
            "PRESENTATION_STYLE_ASSIGNMENT" => convert_presentation_style_assignment);
        run_pass!(graph, self, "STYLED_ITEM" => convert_styled_item);
        run_pass!(graph, self,
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION" => convert_mdgpr);

        // Pass 8: PMI scaffolding — SHAPE_ASPECT entries that anchor
        // future Tolerance / Datum / GD&T work. Runs before the property
        // converters so that a future Pattern B PD pass can look up
        // SHAPE_ASPECT ids when resolving its target ref.
        run_pass!(graph, self, "SHAPE_ASPECT" => convert_shape_aspect);

        // Pass 8: properties — user-defined attribute chain
        // (PROPERTY_DEFINITION + REPRESENTATION + PROPERTY_DEFINITION_REPRESENTATION).
        // Depends on Pass 0 (unit ctx) and Pass 6 (`pdef_to_product` for
        // resolving the PD's target).
        run_pass!(graph, self,
            "MEASURE_REPRESENTATION_ITEM" => convert_measure_representation_item);
        run_pass!(graph, self,
            "PROPERTY_DEFINITION" => convert_property_definition);

        // PDR convert needs `&graph` to read the bound REPRESENTATION
        // entity directly — REPRESENTATION is a generic entity name shared
        // with MDGPR / SR, so a per-pass map would conflate them.
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
            if name != "PROPERTY_DEFINITION_REPRESENTATION" {
                continue;
            }
            if let Err(e) = self.convert_property_definition_representation(id, attrs, graph) {
                self.warnings.push(e);
            }
        }
    }

    /// Walk the [`ENTITY_HANDLERS`] registry and dispatch every handler whose
    /// `pass_level` matches. Single-round only; multi-round entities (currently
    /// just `OFFSET_SURFACE`) keep their bespoke `run_pass!` + loop.
    fn dispatch_registry(&mut self, graph: &EntityGraph, pass_level: PassLevel) {
        for entry in ENTITY_HANDLERS {
            if entry.pass_level != pass_level {
                continue;
            }
            self.dispatch_entry(graph, entry);
        }
    }

    /// Apply one registry entry to every matching entity in the graph.
    /// `ReadKind::Simple` matches `RawEntity::Simple` by name; `Complex`
    /// matches `RawEntity::Complex` whose parts contain every required
    /// part name. Mirrors the `run_pass!` macro body for the simple path
    /// and the hand-rolled Pass 4-2 loop for the complex path.
    fn dispatch_entry(&mut self, graph: &EntityGraph, entry: &EntityHandlerEntry) {
        for (&id, ent) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            match (&entry.kind, ent) {
                (
                    ReadKind::Simple { read },
                    RawEntity::Simple {
                        name, attributes, ..
                    },
                ) if name == entry.name => {
                    if let Err(e) = read(self, id, attributes) {
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
                    if let Err(e) = read(self, id, parts) {
                        self.warnings.push(e);
                    }
                }
                _ => {}
            }
        }
    }

    /// Like [`dispatch_registry`], but for handlers whose entities live
    /// inside a `PCURVE` `DEFINITIONAL_REPRESENTATION` subtree (2D
    /// geometry). Plan 5.5 introduced this twin entry point so that
    /// shared entity names (`CARTESIAN_POINT`, `DIRECTION`, `LINE`, …)
    /// can have separate 3D / 2D handlers without confusion.
    fn dispatch_registry_2d(&mut self, graph: &EntityGraph, pass_level: PassLevel) {
        for entry in ENTITY_HANDLERS {
            if entry.pass_level != pass_level {
                continue;
            }
            self.dispatch_entry_2d(graph, entry);
        }
    }

    /// Apply one registry entry to every entity *inside* a pcurve
    /// subtree — the mirror image of [`dispatch_entry`] (which skips
    /// pcurve subtree members). The body is otherwise identical; the
    /// duplication is a deliberate trade-off documented in the Plan 5.5
    /// design (see `internal/IR_DESIGN.md` for the planned future
    /// generalisation into a single `EntityContext`-driven dispatch).
    fn dispatch_entry_2d(&mut self, graph: &EntityGraph, entry: &EntityHandlerEntry) {
        for (&id, ent) in &graph.entities {
            if !self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            match (&entry.kind, ent) {
                (
                    ReadKind::Simple { read },
                    RawEntity::Simple {
                        name, attributes, ..
                    },
                ) if name == entry.name => {
                    if let Err(e) = read(self, id, attributes) {
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
                    if let Err(e) = read(self, id, parts) {
                        self.warnings.push(e);
                    }
                }
                _ => {}
            }
        }
    }
}
