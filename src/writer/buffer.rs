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
pub(crate) mod geometry;
pub(crate) mod pmi;
pub(crate) mod property;
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
    /// Cache of `LENGTH_UNIT` complex entities, keyed by the
    /// defining IR fields `(length, length_cbu_wrapped)`. Sharing across
    /// contexts that pick the same length unit; distinct keys (e.g. an inch
    /// ctx + a mm ctx in the same file) emit separate leaf entities.
    pub(crate) length_unit_ids: HashMap<(crate::ir::LengthUnit, bool), u64>,
    /// Same idea for plane-angle units.
    pub(crate) angle_unit_ids: HashMap<(crate::ir::AngleUnit, bool), u64>,
    /// Same idea for solid-angle units.
    pub(crate) solid_angle_unit_ids: HashMap<crate::ir::SolidAngleUnit, u64>,
    /// STEP entity ids of every emitted `REPRESENTATION_CONTEXT` complex
    /// entity, indexed by `UnitContextId.0`. Populated up-front in `emit_all`
    /// so every representation emitter can resolve its `Option<UnitContextId>`
    /// to a cached id.
    pub(crate) unit_context_ids: Vec<u64>,
    /// `ProductId → PRODUCT_DEFINITION step id`. Populated by
    /// `emit_assembly_chain`; consumed by the property emitter so a
    /// `Property.target` can be resolved to a STEP ref. Empty when the
    /// model has no assembly (kernel-built IR with properties only — the
    /// property emitter silently skips in that case).
    pub(crate) product_def_ids: std::collections::HashMap<ProductId, u64>,
    /// `ProductId → PRODUCT_DEFINITION_SHAPE step id`. Same role as
    /// `product_def_ids` but for the PDS sibling — consumed by the PMI
    /// emitter to resolve `ShapeAspect.target` (SAs reference PDS, not PD).
    pub(crate) product_def_shape_ids: std::collections::HashMap<ProductId, u64>,
    /// Cached `DIMENSIONAL_EXPONENTS(1,0,0,0,0,0,0)` — length dimension.
    /// Re-used by any `CONVERSION_BASED_UNIT` with a length flavour.
    pub(crate) length_dim_exp_id: Option<u64>,
    /// Cached `DIMENSIONAL_EXPONENTS(0,0,0,0,0,0,0)` — dimensionless.
    /// Re-used by the degree `CONVERSION_BASED_UNIT`.
    pub(crate) dimensionless_dim_exp_id: Option<u64>,
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
            length_unit_ids: HashMap::new(),
            angle_unit_ids: HashMap::new(),
            solid_angle_unit_ids: HashMap::new(),
            unit_context_ids: Vec::new(),
            product_def_ids: std::collections::HashMap::new(),
            product_def_shape_ids: std::collections::HashMap::new(),
            length_dim_exp_id: None,
            dimensionless_dim_exp_id: None,
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
        // Emit one REPRESENTATION_CONTEXT per IR `UnitContext`. The cached
        // STEP ids land in `unit_context_ids` so each downstream emitter can
        // resolve `Option<UnitContextId>` with a single index lookup.
        //
        // Leaf-unit caches in `WriteBuffer` are keyed by their defining IR
        // fields (e.g. `(LengthUnit, length_cbu_wrapped)`) so two contexts
        // with identical units share the same `LENGTH_UNIT` entity, while
        // contexts with different units emit separate leaves.
        self.unit_context_ids = Vec::with_capacity(self.model.units.len());
        for ctx in self.model.units.iter() {
            let id = self.emit_unit_context(ctx.clone())?;
            self.unit_context_ids.push(id);
        }
        self.emit_product_chain_if_eligible()?;
        self.emit_pmi_if_set();
        self.emit_visualization_if_set()?;
        self.emit_properties_if_set();
        Ok(())
    }

    pub(crate) fn fresh(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}
