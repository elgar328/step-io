//! Topology pool emission: vertices, edges, wires, faces, shells, solids.

use super::WriteBuffer;
use crate::ir::{
    EdgeId, Face, FaceId, FaceKind, Orientation, Shell, ShellId, Solid, SolidId, VertexId, Wire,
    WireId,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(crate) fn emit_vertex(&mut self, id: VertexId) -> Result<u64, WriteError> {
        // Plan 4 stage C2: dispatch through the EntityHandler trait. Body
        // lives in `src/entities/topology/vertex_point.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::vertex_point::VertexPointHandler::write(self, id)
    }

    pub(crate) fn emit_edge(&mut self, id: EdgeId) -> Result<u64, WriteError> {
        // Plan 4 stage C2: dispatch through the EntityHandler trait. Body
        // lives in `src/entities/topology/edge_curve.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::topology::edge_curve::EdgeCurveHandler::write(self, id)
    }

    pub(crate) fn emit_wire(&mut self, id: WireId) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        if let Some(&n) = self.wire_ids.get(&id) {
            return Ok(n);
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
        // EDGE_LOOP (common case) or VERTEX_LOOP (degenerate — a single
        // vertex, used by some revolutions and sphere poles). Reader stores
        // either `edges` or `vertex`, never both.
        let loop_id = if let Some(vid) = w.vertex {
            crate::entities::topology::vertex_loop::VertexLoopHandler::write(self, vid)?
        } else {
            crate::entities::topology::edge_loop::EdgeLoopHandler::write(self, w.edges)?
        };
        // FACE_BOUND / FACE_OUTER_BOUND wrapper still inlined here; moves
        // to dedicated handlers in Plan 4 stage C4.
        let wrapper = if w.is_outer {
            "FACE_OUTER_BOUND"
        } else {
            "FACE_BOUND"
        };
        let bound_id = self.fresh();
        self.entities.push(WriterEntity {
            id: bound_id,
            body: WriterBody::Simple {
                name: wrapper.into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loop_id),
                    orientation_bool(w.orientation),
                ],
            },
        });
        self.wire_ids.insert(id, bound_id);
        Ok(bound_id)
    }

    pub(crate) fn emit_face(&mut self, id: FaceId) -> Result<u64, WriteError> {
        if let Some(&n) = self.face_ids.get(&id) {
            return Ok(n);
        }
        let f: Face = self
            .model
            .topology
            .faces
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("FaceId({})", id.0),
            })?;
        let surface = self.emit_surface(f.surface)?;
        let mut bound_refs = Vec::with_capacity(f.bounds.len());
        for &wid in &f.bounds {
            bound_refs.push(self.emit_wire(wid)?);
        }
        let name = match f.kind {
            FaceKind::Advanced => "ADVANCED_FACE",
            FaceKind::General => "FACE_SURFACE",
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: name.into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(bound_refs.into_iter().map(Attribute::EntityRef).collect()),
                    Attribute::EntityRef(surface),
                    orientation_bool(f.orientation),
                ],
            },
        });
        self.face_ids.insert(id, n);
        Ok(n)
    }

    pub(crate) fn emit_shell(&mut self, id: ShellId) -> Result<u64, WriteError> {
        if let Some(&n) = self.shell_ids.get(&id) {
            return Ok(n);
        }
        let s: Shell = self
            .model
            .topology
            .shells
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("ShellId({})", id.0),
            })?;
        // `Shell.orientation` is intentionally ignored here — CLOSED_SHELL
        // carries no orientation attribute in STEP. Inner void shells get
        // their orientation via an ORIENTED_CLOSED_SHELL wrapper created
        // in `emit_solid`.
        let mut face_refs = Vec::with_capacity(s.faces.len());
        for &fid in &s.faces {
            face_refs.push(self.emit_face(fid)?);
        }
        let name = if s.is_open {
            "OPEN_SHELL"
        } else {
            "CLOSED_SHELL"
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: name.into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(face_refs.into_iter().map(Attribute::EntityRef).collect()),
                ],
            },
        });
        self.shell_ids.insert(id, n);
        Ok(n)
    }

    pub(crate) fn emit_oriented_closed_shell(
        &mut self,
        closed_shell_ref: u64,
        orientation: Orientation,
    ) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ORIENTED_CLOSED_SHELL".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::Derived,
                    Attribute::EntityRef(closed_shell_ref),
                    orientation_bool(orientation),
                ],
            },
        });
        n
    }

    pub(crate) fn emit_solid(&mut self, id: SolidId) -> Result<u64, WriteError> {
        if let Some(&n) = self.solid_ids.get(&id) {
            return Ok(n);
        }
        let s: Solid = self
            .model
            .topology
            .solids
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("SolidId({})", id.0),
            })?;
        let outer_id = *s.shells.first().ok_or_else(|| WriteError::DanglingId {
            detail: format!("SolidId({}) has no shells", id.0),
        })?;
        let outer_ref = self.emit_shell(outer_id)?;
        let name = s.name.clone().unwrap_or_default();
        let n = self.fresh();

        let body = if s.shells.len() == 1 {
            WriterBody::Simple {
                name: "MANIFOLD_SOLID_BREP".into(),
                attrs: vec![Attribute::String(name), Attribute::EntityRef(outer_ref)],
            }
        } else {
            let mut void_refs = Vec::with_capacity(s.shells.len() - 1);
            for &inner_id in &s.shells[1..] {
                let inner_shell: Shell = self
                    .model
                    .topology
                    .shells
                    .iter()
                    .nth(inner_id.0 as usize)
                    .cloned()
                    .ok_or_else(|| WriteError::DanglingId {
                        detail: format!("ShellId({}) void", inner_id.0),
                    })?;
                let inner_cs_ref = self.emit_shell(inner_id)?;
                let ocs_ref =
                    self.emit_oriented_closed_shell(inner_cs_ref, inner_shell.orientation);
                void_refs.push(Attribute::EntityRef(ocs_ref));
            }
            WriterBody::Simple {
                name: "BREP_WITH_VOIDS".into(),
                attrs: vec![
                    Attribute::String(name),
                    Attribute::EntityRef(outer_ref),
                    Attribute::List(void_refs),
                ],
            }
        };
        self.entities.push(WriterEntity { id: n, body });
        self.solid_ids.insert(id, n);
        Ok(n)
    }
}

pub(crate) fn orientation_bool(o: Orientation) -> Attribute {
    match o {
        Orientation::Forward => Attribute::Enum("T".into()),
        Orientation::Reversed => Attribute::Enum("F".into()),
    }
}
