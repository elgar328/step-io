//! Pass orchestration for geometry, topology and assembly conversion.

use super::assembly::{pdef_shape_to_nauo_ref, pdef_shape_to_pdef_ref};
use super::{ReaderContext, has_all_parts};
use crate::ir::topology::FaceKind;
use crate::parser::entity::{EntityGraph, RawEntity};

impl ReaderContext {
    /// Pass 0: unit context. Runs before all geometry passes so that
    /// `model.units` is known by the time downstream code inspects it.
    pub(super) fn run_unit_pass(&mut self, graph: &EntityGraph) {
        // Pass 0-1: SI_UNIT-bearing leaves (LENGTH/PLANE_ANGLE/SOLID_ANGLE).
        for (&id, entity) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            let parts = match entity {
                RawEntity::Complex { parts, .. } => parts,
                RawEntity::Simple { .. } => continue,
            };
            if let Err(e) = self.convert_unit_leaf(id, parts) {
                self.warnings.push(e);
            }
        }

        // Pass 0-1b: UNCERTAINTY_MEASURE_WITH_UNIT (simple entities,
        // depend on Pass 0-1 to know which unit refs are length).
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
            if name != "UNCERTAINTY_MEASURE_WITH_UNIT" {
                continue;
            }
            if let Err(e) = self.convert_uncertainty_measure_with_unit(id, attrs) {
                self.warnings.push(e);
            }
        }

        // Pass 0-2: GLOBAL_UNIT_ASSIGNED_CONTEXT.
        for (&id, entity) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            let parts = match entity {
                RawEntity::Complex { parts, .. } => parts,
                RawEntity::Simple { .. } => continue,
            };
            if !parts
                .iter()
                .any(|p| p.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            {
                continue;
            }
            if let Err(e) = self.convert_global_unit_assigned_context(id, parts) {
                self.warnings.push(e);
            }
        }
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

        // Pass 1: points and directions
        run_pass!(graph, self, "CARTESIAN_POINT" => convert_cartesian_point, "DIRECTION" => convert_direction);
        // Pass 2: vectors (depend on directions)
        run_pass!(graph, self, "VECTOR" => convert_vector);
        // Pass 3: axis placements (depend on points + directions)
        run_pass!(graph, self, "AXIS2_PLACEMENT_3D" => convert_axis2_placement_3d, "AXIS1_PLACEMENT" => convert_axis1_placement);
        // Pass 4-1: simple leaf curves and surfaces
        run_pass!(graph, self, "LINE" => convert_line, "PLANE" => convert_plane, "CYLINDRICAL_SURFACE" => convert_cylindrical_surface, "SPHERICAL_SURFACE" => convert_spherical_surface, "CONICAL_SURFACE" => convert_conical_surface, "TOROIDAL_SURFACE" => convert_toroidal_surface, "CIRCLE" => convert_circle, "ELLIPSE" => convert_ellipse, "B_SPLINE_CURVE_WITH_KNOTS" => convert_bspline_curve_with_knots, "B_SPLINE_SURFACE_WITH_KNOTS" => convert_bspline_surface_with_knots);

        // Pass 4-1b: TRIMMED_CURVE — depends on Pass 4-1 basis curves (LINE,
        // CIRCLE, B_SPLINE_CURVE_WITH_KNOTS).
        run_pass!(graph, self, "TRIMMED_CURVE" => convert_trimmed_curve);
        // Pass 4-1c: COMPOSITE_CURVE_SEGMENT — depends on parent curves
        // (typically TRIMMED_CURVE, occasionally LINE / CIRCLE).
        run_pass!(graph, self, "COMPOSITE_CURVE_SEGMENT" => convert_composite_curve_segment);
        // Pass 4-1d: COMPOSITE_CURVE — depends on segments populated above.
        run_pass!(graph, self, "COMPOSITE_CURVE" => convert_composite_curve);

        // Pass 4-2: complex rational B-spline curves and surfaces (depend on points)
        for (&id, entity) in &graph.entities {
            if self.pcurve_subtree_ids.contains(&id) {
                continue;
            }
            let parts = match entity {
                RawEntity::Complex { parts, .. } => parts,
                RawEntity::Simple { .. } => continue,
            };
            let result = if has_all_parts(
                parts,
                &[
                    "B_SPLINE_CURVE",
                    "B_SPLINE_CURVE_WITH_KNOTS",
                    "RATIONAL_B_SPLINE_CURVE",
                ],
            ) {
                self.convert_rational_bspline_curve(id, parts)
            } else if has_all_parts(
                parts,
                &[
                    "B_SPLINE_SURFACE",
                    "B_SPLINE_SURFACE_WITH_KNOTS",
                    "RATIONAL_B_SPLINE_SURFACE",
                ],
            ) {
                self.convert_rational_bspline_surface(id, parts)
            } else {
                continue;
            };
            if let Err(e) = result {
                self.warnings.push(e);
            }
        }

        // Pass 4a: PCURVE parametric-space 2D geometry. Iterates the
        // `pcurve_subtree_ids` (populated by `collect_pcurve_pcurve_subtree_ids`) and routes
        // each 2D entity to the 2D converter family. Runs before Pass 4-3
        // so the PCURVE resolver can look up 2D curves it references.
        macro_rules! run_2d_pass {
            ($graph:expr, $ctx:expr, $( $name:literal => $method:ident ),+ $(,)?) => {
                for (&id, entity) in &$graph.entities {
                    if !$ctx.pcurve_subtree_ids.contains(&id) { continue; }
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
        // 4a-1: 2D points / directions
        run_2d_pass!(graph, self,
            "CARTESIAN_POINT" => convert_cartesian_point_2d,
            "DIRECTION" => convert_direction_2d);
        // 4a-2: 2D vectors + axis2_placement_2d
        run_2d_pass!(graph, self,
            "VECTOR" => convert_vector_2d,
            "AXIS2_PLACEMENT_2D" => convert_axis2_placement_2d);
        // 4a-3: 2D curves
        run_2d_pass!(graph, self,
            "LINE" => convert_line_2d,
            "CIRCLE" => convert_circle_2d,
            "ELLIPSE" => convert_ellipse_2d,
            "B_SPLINE_CURVE_WITH_KNOTS" => convert_bspline_curve_2d_with_knots);

        // Pass 4-3: SURFACE_CURVE / SEAM_CURVE — alias to curve_3d so edges
        // that reference these see the underlying 3D curve. Must come after
        // 4-1 and 4-2 (curve_3d usually resolves to a simple or rational curve).
        run_pass!(graph, self,
            "SURFACE_CURVE" => convert_surface_curve,
            "SEAM_CURVE" => convert_seam_curve);

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
                self.collect_surface_curve_pcurves(id, attrs, graph);
            }
        }

        // Pass 4-4A: derived surfaces that wrap a curve (swept curve / axis
        // of revolution / extrusion vector). Single sweep — no dependency
        // on other derived surfaces.
        run_pass!(graph, self,
            "SURFACE_OF_REVOLUTION" => convert_surface_of_revolution,
            "SURFACE_OF_LINEAR_EXTRUSION" => convert_surface_of_linear_extrusion);

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

        // Special pass for FACE_BOUND/FACE_OUTER_BOUND (extra bool param).
        macro_rules! run_face_bound_pass {
            ($graph:expr, $ctx:expr) => {
                for (&id, entity) in &$graph.entities {
                    if $ctx.pcurve_subtree_ids.contains(&id) {
                        continue;
                    }
                    let (name, attrs) = match entity {
                        RawEntity::Simple {
                            name, attributes, ..
                        } => (name.as_str(), attributes.as_slice()),
                        RawEntity::Complex { .. } => continue,
                    };
                    let result = match name {
                        "FACE_BOUND" => $ctx.convert_face_bound(id, attrs, false),
                        "FACE_OUTER_BOUND" => $ctx.convert_face_bound(id, attrs, true),
                        _ => continue,
                    };
                    if let Err(e) = result {
                        $ctx.warnings.push(e);
                    }
                }
            };
        }

        // Special pass for ADVANCED_FACE / FACE_SURFACE (extra FaceKind param).
        macro_rules! run_face_pass {
            ($graph:expr, $ctx:expr) => {
                for (&id, entity) in &$graph.entities {
                    if $ctx.pcurve_subtree_ids.contains(&id) {
                        continue;
                    }
                    let (name, attrs) = match entity {
                        RawEntity::Simple {
                            name, attributes, ..
                        } => (name.as_str(), attributes.as_slice()),
                        RawEntity::Complex { .. } => continue,
                    };
                    let result = match name {
                        "ADVANCED_FACE" => $ctx.convert_face(id, attrs, FaceKind::Advanced),
                        "FACE_SURFACE" => $ctx.convert_face(id, attrs, FaceKind::General),
                        _ => continue,
                    };
                    if let Err(e) = result {
                        $ctx.warnings.push(e);
                    }
                }
            };
        }

        // Pass 5-1: vertices
        run_pass!(graph, self, "VERTEX_POINT" => convert_vertex_point);
        // Pass 5-2: edges (depend on vertices + curves)
        run_pass!(graph, self, "EDGE_CURVE" => convert_edge_curve);
        // Pass 5-3: oriented edges (intermediate, depend on edges)
        run_pass!(graph, self, "ORIENTED_EDGE" => convert_oriented_edge);
        // Pass 5-4: edge loops (intermediate, depend on oriented edges).
        //   Also handles VERTEX_LOOP (degenerate single-vertex boundary
        //   used by spheres). VERTEX_LOOP is stored as an empty edge list
        //   so FACE_BOUND resolves uniformly.
        run_pass!(graph, self,
            "EDGE_LOOP" => convert_edge_loop,
            "VERTEX_LOOP" => convert_vertex_loop);
        // Pass 5-5: face bounds / outer bounds (depend on edge loops)
        run_face_bound_pass!(graph, self);
        // Pass 5-6: faces (ADVANCED_FACE + FACE_SURFACE, depend on face bounds + surfaces)
        run_face_pass!(graph, self);
        // Pass 5-7a: shells (depend on faces)
        run_pass!(graph, self,
            "CLOSED_SHELL" => convert_closed_shell,
            "OPEN_SHELL" => convert_open_shell);
        // Pass 5-7b: oriented shells (depend on CLOSED_SHELL already
        // in the arena; kept separate because ORIENTED_CLOSED_SHELL
        // entities may precede their referenced CLOSED_SHELL in #N order).
        run_pass!(graph, self, "ORIENTED_CLOSED_SHELL" => convert_oriented_closed_shell);
        // Pass 5-8: solids (depend on shells + oriented shells)
        run_pass!(graph, self,
            "MANIFOLD_SOLID_BREP" => convert_manifold_solid_brep,
            "BREP_WITH_VOIDS" => convert_brep_with_voids);
    }

    /// Pass 6: assembly/product graph (Phase A — PRODUCT chain + shape
    /// classification; Phase B adds instances, transforms, tree root).
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
        run_pass!(graph, self, "PRODUCT" => convert_product);
        // Pass 6-2: PRODUCT_DEFINITION_FORMATION [+ AP203 WITH_SPECIFIED_SOURCE]
        run_pass!(graph, self,
            "PRODUCT_DEFINITION_FORMATION" => convert_product_definition_formation,
            "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE" => convert_product_definition_formation);
        // Pass 6-3: PRODUCT_DEFINITION → pdef_to_product
        run_pass!(graph, self, "PRODUCT_DEFINITION" => convert_product_definition);
        // Pass 6-4: SBSM (surface body shell list) — must precede MSSR.
        run_pass!(graph, self,
            "SHELL_BASED_SURFACE_MODEL" => convert_shell_based_surface_model);
        // Pass 6-4a: ABSR + MSSR + plain SR (shape representations).
        // `run_pass!` exact-matches entity names; ABSR/MSSR subtypes carry
        // distinct concrete names in STEP files so the plain `SHAPE_REPRESENTATION`
        // entry below catches only the bare type used by Group products and the
        // outer wrapper of Fusion 360 / CATIA indirect-SR chains.
        run_pass!(graph, self,
            "ADVANCED_BREP_SHAPE_REPRESENTATION" => convert_advanced_brep_shape_representation,
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION" => convert_manifold_surface_shape_representation,
            "SHAPE_REPRESENTATION" => convert_plain_shape_representation);
        // Pass 6-4f: GEOMETRIC_(CURVE_)SET — must precede GBWSR/GBSSR so the
        // wireframe converters can look up the curve-set payload.
        run_pass!(graph, self,
            "GEOMETRIC_CURVE_SET" => convert_geometric_curve_set,
            "GEOMETRIC_SET" => convert_geometric_curve_set);
        // Pass 6-4g: GBWSR / GBSSR — wraps GEOMETRIC_(CURVE_)SETs into a
        // shape representation. Both forms reach the same IR variant
        // (`ProductContent::Wireframe`) with a `repr_kind` flag preserving
        // the source spelling.
        run_pass!(graph, self,
            "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION" => convert_gbwsr,
            "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION" => convert_gbssr);
        // Pass 6-4b: simple SHAPE_REPRESENTATION_RELATIONSHIP (indirection
        // from plain SR to ABSR / MSSR / wireframe). Must run after the
        // shape-representation passes above so the is-target lookup sees
        // populated maps.
        run_pass!(graph, self,
            "SHAPE_REPRESENTATION_RELATIONSHIP" => convert_shape_representation_relationship);

        // Pre-Pass 6-5: classify PRODUCT_DEFINITION_SHAPE entities by whether
        // their `definition` target is a PRODUCT_DEFINITION (product-owned)
        // versus a NAUO (instance-owned — Phase B).
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
        run_pass!(graph, self,
            "SHAPE_DEFINITION_REPRESENTATION" => convert_shape_definition_representation);

        // Pass 6-6: ITEM_DEFINED_TRANSFORMATION — build transform_map.
        run_pass!(graph, self,
            "ITEM_DEFINED_TRANSFORMATION" => convert_item_defined_transformation);

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
        run_pass!(graph, self,
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => convert_next_assembly_usage_occurrence);
    }
}
