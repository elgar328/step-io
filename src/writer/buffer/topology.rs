//! Topology pool emission: vertices, edges, wires, faces, shells, solids.

use super::WriteBuffer;
use crate::ir::{EdgeId, FaceId, Orientation, ShellId, SolidId, VertexId, Wire, WireId};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(crate) fn emit_vertex(&mut self, id: VertexId) -> Result<u64, WriteError> {
        // Dispatch through the EntityHandler trait. Body
        // lives in `src/entities/geometry/vertex_point.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::vertex_point::VertexPointHandler::write(self, id)
    }

    pub(crate) fn emit_edge(&mut self, id: EdgeId) -> Result<u64, WriteError> {
        // Dispatch through the EntityHandler trait. Body
        // lives in `src/entities/topology/edge_curve.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::edge_curve::EdgeCurveHandler::write(self, id)
    }

    pub(crate) fn emit_wire(&mut self, id: WireId) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        let cached = self.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let w: Wire = self
            .model
            .topology
            .wires
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("WireId({})", id.0),
            })?;
        // FACE_BOUND / FACE_OUTER_BOUND wrapper now lives in dedicated
        // handlers; pick the matching one by the `Wire` variant.
        let is_outer = matches!(w, Wire::FaceOuterBound(_));
        let data = match w {
            Wire::FaceBound(d) | Wire::FaceOuterBound(d) => d,
        };
        // EDGE_LOOP (common case) or VERTEX_LOOP (degenerate — a single
        // vertex, used by some revolutions and sphere poles). Reader stores
        // either `edges` or `vertex`, never both.
        let loop_id = if let Some(vid) = data.vertex {
            crate::entities::topology::vertex_loop::VertexLoopHandler::write(self, vid)?
        } else {
            crate::entities::topology::edge_loop::EdgeLoopHandler::write(self, data.edges)?
        };
        let bound_id = if is_outer {
            crate::entities::topology::face_outer_bound::FaceOuterBoundHandler::write(
                self,
                (loop_id, data.orientation),
            )?
        } else {
            crate::entities::topology::face_bound::FaceBoundHandler::write(
                self,
                (loop_id, data.orientation),
            )?
        };
        self.set_step_id(id, bound_id);
        Ok(bound_id)
    }

    pub(crate) fn emit_face(&mut self, id: FaceId) -> Result<u64, WriteError> {
        // Delegated to the AdvancedFace/FaceSurface
        // handlers (write body keys off the IR-stored Face variant).
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::advanced_face::AdvancedFaceHandler::write(self, id)
    }

    pub(crate) fn emit_shell(&mut self, id: ShellId) -> Result<u64, WriteError> {
        // Write body keys off the IR-stored `is_open` to
        // pick CLOSED_SHELL or OPEN_SHELL. Helper lives in
        // `entities/topology/closed_shell.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::closed_shell::ClosedShellHandler::write(self, id)
    }

    pub(crate) fn emit_oriented_closed_shell(
        &mut self,
        closed_shell_ref: u64,
        orientation: Orientation,
    ) -> Result<u64, WriteError> {
        // Dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::oriented_closed_shell::OrientedClosedShellHandler::write(
            self,
            (closed_shell_ref, orientation),
        )
    }

    pub(crate) fn emit_solid(&mut self, id: SolidId) -> Result<u64, WriteError> {
        // Solids dispatch by `Solid` variant: ManifoldSolidBrep →
        // MANIFOLD_SOLID_BREP, BrepWithVoids → BREP_WITH_VOIDS.
        use crate::entities::SimpleEntityHandler;
        let solid = self
            .model
            .topology
            .solids
            .iter()
            .nth(id.0 as usize)
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("SolidId({})", id.0),
            })?;
        match solid {
            crate::ir::Solid::ManifoldSolidBrep { .. } => {
                crate::entities::topology::manifold_solid_brep::ManifoldSolidBrepHandler::write(
                    self, id,
                )
            }
            crate::ir::Solid::BrepWithVoids { .. } => {
                crate::entities::topology::brep_with_voids::BrepWithVoidsHandler::write(self, id)
            }
        }
    }
}

pub(crate) fn orientation_bool(o: Orientation) -> Attribute {
    match o {
        Orientation::Forward => Attribute::Enum("T".into()),
        Orientation::Reversed => Attribute::Enum("F".into()),
    }
}
