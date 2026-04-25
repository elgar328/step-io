//! Converts a raw [`EntityGraph`] into a typed [`StepModel`].
//!
//! This module is the boundary between the parser layer and the IR layer.
//! It uses multi-pass eager conversion: entities are processed in dependency
//! order so that referenced objects are always available when needed.

use std::collections::{HashMap, HashSet};

use crate::ir::arena::Arena;
use crate::ir::assembly::{AssemblyTree, Product, Transform3d};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Pcurve, TransitionCode};
use crate::ir::id::{
    Curve2dId, CurveId, Direction2dId, DirectionId, EdgeId, FaceId, Placement1dId, Placement2dId,
    Placement3dId, Point2dId, PointId, ProductId, ShellId, SolidId, SurfaceId, VertexId, WireId,
};
use crate::ir::model::{
    AngleUnit, GeometryPool, LengthUnit, SolidAngleUnit, StepModel, TopologyPool, UnitContext,
};
use crate::ir::topology::{Orientation, OrientedEdge};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

mod assembly;
mod geometry;
mod header;
mod passes;
mod topology;
mod units;

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
}

/// Accumulates converted IR objects and tracks the mapping from STEP entity
/// ids (`#N`) to typed arena Ids.
#[derive(Default)]
// `solid_angle_unit_map` below has a zero-sized value type; keep it as a
// map for symmetry with the other unit maps and to leave room for future
// `SolidAngleUnit` variants.
#[allow(clippy::zero_sized_map_values)]
pub struct ReaderContext {
    pub(super) geometry: GeometryPool,
    pub(super) topology: TopologyPool,
    pub(super) units: Option<UnitContext>,

    /// Entity ids inside any `DEFINITIONAL_REPRESENTATION` subtree (PCURVE
    /// parametric-space geometry). 3D passes skip them so their 2D
    /// `CARTESIAN_POINT` / `DIRECTION` / `LINE` / … don't collide with 3D
    /// conversion. Pass 4a then walks the same set to populate the 2D
    /// arenas (`points_2d`, `directions_2d`, `curves_2d`).
    pub(super) pcurve_subtree_ids: HashSet<u64>,

    // Unit entity maps: STEP #N → resolved unit variant.
    pub(super) length_unit_map: HashMap<u64, LengthUnit>,
    pub(super) angle_unit_map: HashMap<u64, AngleUnit>,
    pub(super) solid_angle_unit_map: HashMap<u64, SolidAngleUnit>,
    /// `UNCERTAINTY_MEASURE_WITH_UNIT #N → value` for uncertainty entities
    /// whose `unit_component` resolved to a length unit. Populated between
    /// Pass 0-1 (unit leaves) and Pass 0-2 (context assembly).
    pub(super) length_uncertainty_map: HashMap<u64, f64>,

    // Geometry maps: STEP #N → typed Id.
    pub(super) point_map: HashMap<u64, PointId>,
    pub(super) direction_map: HashMap<u64, DirectionId>,
    pub(super) surface_map: HashMap<u64, SurfaceId>,
    pub(super) curve_map: HashMap<u64, CurveId>,

    // Geometry intermediate maps.
    pub(super) placement_map: HashMap<u64, Placement3dId>,
    pub(super) vector_map: HashMap<u64, (DirectionId, f64)>,
    pub(super) axis1_map: HashMap<u64, Placement1dId>,

    // 2D geometry (PCURVE parametric space) maps.
    pub(super) point_2d_map: HashMap<u64, Point2dId>,
    pub(super) direction_2d_map: HashMap<u64, Direction2dId>,
    pub(super) curve_2d_map: HashMap<u64, Curve2dId>,
    pub(super) vector_2d_map: HashMap<u64, (Direction2dId, f64)>,
    pub(super) placement_2d_map: HashMap<u64, Placement2dId>,
    /// `SURFACE_CURVE / SEAM_CURVE #N → Vec<Pcurve>`. Populated during
    /// Pass 4-3 and consumed by `convert_edge_curve` to attach pcurves to
    /// each edge.
    pub(super) surface_curve_pcurves_map: HashMap<u64, Vec<Pcurve>>,

    // Topology maps: STEP #N → typed Id.
    pub(super) vertex_map: HashMap<u64, VertexId>,
    pub(super) edge_map: HashMap<u64, EdgeId>,
    pub(super) face_bound_map: HashMap<u64, WireId>,
    pub(super) face_map: HashMap<u64, FaceId>,
    pub(super) shell_map: HashMap<u64, ShellId>,
    pub(super) solid_map: HashMap<u64, SolidId>,

    // Topology intermediate maps.
    pub(super) oriented_edge_map: HashMap<u64, OrientedEdge>,
    pub(super) edge_loop_map: HashMap<u64, Vec<OrientedEdge>>,
    /// `VERTEX_LOOP #N → VertexId`. `FACE_BOUND` consults this when the
    /// loop ref is not in `edge_loop_map`.
    pub(super) vertex_loop_map: HashMap<u64, VertexId>,
    /// `ORIENTED_CLOSED_SHELL #N → (underlying CLOSED_SHELL's ShellId,
    /// wrapper orientation)`. Populated by Pass 5-7b, consumed by
    /// `convert_brep_with_voids` in Pass 5-8.
    pub(super) oriented_closed_shell_map: HashMap<u64, (ShellId, Orientation)>,

    // Assembly (Phase A): product arena + lookup maps populated by Pass 6.
    // `assembly` is filled in `convert()` after Pass 6 if any PRODUCT was seen.
    pub(super) assembly: Option<AssemblyTree>,
    pub(super) assembly_products: Arena<Product>,
    pub(super) product_arena_map: HashMap<u64, ProductId>,
    pub(super) formation_to_product: HashMap<u64, u64>,
    pub(super) pdef_to_product: HashMap<u64, u64>,
    pub(super) absr_solid_map: HashMap<u64, SolidId>,
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION #N → Placement3dId` for the first
    /// `AXIS2_PLACEMENT_3D` item in the ABSR's `items` list (its coordinate
    /// reference frame). Consumed by SDR conversion to populate
    /// `Product.shape_ref_frame`.
    pub(super) absr_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `SHELL_BASED_SURFACE_MODEL #N → resolved shell ids`. Populated in
    /// Pass 5-8b and consumed by MSSR conversion to flatten shells.
    pub(super) sbsm_shells_map: HashMap<u64, Vec<ShellId>>,
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION #N → flattened shell ids`
    /// pulled from the MSSR's referenced SBSM. Consumed by SDR conversion
    /// to populate `Product.content = SurfaceBody(..)`.
    pub(super) mssr_shells_map: HashMap<u64, Vec<ShellId>>,
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION #N → Placement3dId` — same
    /// role as `absr_ref_frame_map` but for the MSSR path. Optional because
    /// some writers omit the AXIS2 item.
    pub(super) mssr_ref_frame_map: HashMap<u64, Placement3dId>,
    /// `plain SHAPE_REPRESENTATION #N → target ABSR/MSSR #N` — built from
    /// simple `SHAPE_REPRESENTATION_RELATIONSHIP` entities where exactly one
    /// side resolves to a known ABSR/MSSR. Consumed by SDR conversion to
    /// follow the Fusion 360 / CATIA indirection chain
    /// `SDR → plain SR → SRR → ABSR/MSSR`.
    pub(super) srr_equiv_map: HashMap<u64, u64>,
    /// `plain SHAPE_REPRESENTATION #N → items[0] axis Placement3dId`. Captured
    /// during Pass 6-4 so SDR conversion can attach the plain SR's reference
    /// frame to `Product.outer_sr_frame` when the indirection chain is taken.
    pub(super) plain_sr_frame_map: HashMap<u64, Placement3dId>,
    /// `COMPOSITE_CURVE_SEGMENT #N → (transition, same_sense, parent_curve
    /// step id)`. Populated by Pass 4 immediately after `TRIMMED_CURVE` so
    /// `COMPOSITE_CURVE` conversion can resolve segments by entity ref.
    pub(super) composite_segment_map: HashMap<u64, (TransitionCode, bool, u64)>,
    /// `PRODUCT_DEFINITION_SHAPE #N → PRODUCT_DEFINITION #N` when the
    /// `pdef_shape` points at a product definition (not a `NAUO`).
    /// Populated before Pass 6-5.
    pub(super) pdef_shape_to_pdef: HashMap<u64, u64>,
    /// `PRODUCT_DEFINITION_SHAPE #N → NEXT_ASSEMBLY_USAGE_OCCURRENCE #N` when
    /// the `pdef_shape` points at a `NAUO` (instance-tagged). Populated
    /// alongside `pdef_shape_to_pdef` and consumed by Pass 6-7.
    pub(super) pdef_shape_to_nauo: HashMap<u64, u64>,
    pub(super) transform_map: HashMap<u64, Transform3d>,
    pub(super) nauo_transform_map: HashMap<u64, Transform3d>,

    pub(super) warnings: Vec<ConvertError>,
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
            },
            warnings: ctx.warnings,
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
            if let crate::ir::assembly::ProductContent::Group(instances) = &product.content {
                for inst in instances {
                    is_child.insert(inst.child);
                }
            }
        }
        // Arena<T>::iter() only hands out `&T`, so reconstruct ProductId
        // from the enumeration index. `push` assigns sequential ids from 0.
        // Product counts are dominated by STEP entity ids (u64) which fit
        // comfortably in u32 for any realistic file.
        #[allow(clippy::cast_possible_truncation)]
        let roots: Vec<ProductId> = self
            .assembly_products
            .iter()
            .enumerate()
            .map(|(i, _)| ProductId(i as u32))
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
        self.assembly = Some(AssemblyTree { products, root });
    }

    // ---------------------------------------------------------------------
    // Resolver helpers: look up a STEP entity id in one of the internal
    // maps and return the stored IR id (or a `MissingReference` error).
    // Each converter can collapse four lines of boilerplate into one call.
    // ---------------------------------------------------------------------

    pub(super) fn resolve_point(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<PointId, ConvertError> {
        self.point_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_direction(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<DirectionId, ConvertError> {
        self.direction_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_curve(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<CurveId, ConvertError> {
        self.curve_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_surface(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<SurfaceId, ConvertError> {
        self.surface_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_vertex(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<VertexId, ConvertError> {
        self.vertex_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_edge(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<EdgeId, ConvertError> {
        self.edge_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_face_bound(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<WireId, ConvertError> {
        self.face_bound_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_face(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<FaceId, ConvertError> {
        self.face_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_shell(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<ShellId, ConvertError> {
        self.shell_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    /// Two-step lookup `PRODUCT_DEFINITION #N → PRODUCT #N → ProductId`
    /// shared by Pass 6-8 (NAUO tree wiring).
    pub(super) fn resolve_product_by_pdef(
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

    pub(super) fn resolve_placement(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Placement3dId, ConvertError> {
        self.placement_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_vector(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<(DirectionId, f64), ConvertError> {
        self.vector_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_axis1(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Placement1dId, ConvertError> {
        self.axis1_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_oriented_edge(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<OrientedEdge, ConvertError> {
        self.oriented_edge_map
            .get(&to)
            .copied()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }

    pub(super) fn resolve_edge_loop(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<Vec<OrientedEdge>, ConvertError> {
        self.edge_loop_map
            .get(&to)
            .cloned()
            .ok_or(ConvertError::MissingReference {
                from,
                to,
                field_name,
            })
    }
}

// ---------------------------------------------------------------------------
// Free helpers (used by multiple submodules)
// ---------------------------------------------------------------------------

pub(super) fn bool_to_orientation(same_sense: bool) -> Orientation {
    if same_sense {
        Orientation::Forward
    } else {
        Orientation::Reversed
    }
}

/// Find a part by name in a complex entity's part list.
pub(super) fn find_part_attrs<'a>(
    parts: &'a [RawEntityPart],
    name: &str,
) -> Option<&'a [Attribute]> {
    parts
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.attributes.as_slice())
}

/// Find a required part by name. Returns an error if missing.
pub(super) fn require_part_attrs<'a>(
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
pub(super) fn has_all_parts(parts: &[RawEntityPart], required: &[&str]) -> bool {
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
pub(super) fn collect_pcurve_subtree_ids(graph: &EntityGraph) -> HashSet<u64> {
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
