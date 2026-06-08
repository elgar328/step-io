//! Tessellation emission — `COORDINATES_LIST` + `TESSELLATED_CURVE_SET` +
//! `COMPLEX_TRIANGULATED_FACE` + `COMPLEX_TRIANGULATED_SURFACE_SET` +
//! `TESSELLATED_GEOMETRIC_SET`.
//!
//! Three-pass emit so each entity's refs resolve through a filled cache:
//! 1. base `tessellated_item`s (`COORDINATES_LIST` / `TESSELLATED_CURVE_SET`)
//!    — faces resolve their `coordinates` ref through `tessellated_item_step_ids`;
//! 2. faces + surface-sets — a geometric set resolves its `children` through
//!    `tessellated_face_step_ids` / `tessellated_surface_set_step_ids`;
//! 3. composite `tessellated_item`s (`TESSELLATED_GEOMETRIC_SET`).

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::tessellation::{
    ComplexTriangulatedFaceHandler, ComplexTriangulatedSurfaceSetHandler, CoordinatesListHandler,
    TessellatedCurveSetHandler, TessellatedGeometricSetHandler, TessellatedShellHandler,
    TessellatedSolidHandler,
};
use crate::ir::tessellation::{TessellatedItem, TessellatedItemRef};
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_tessellation(&mut self) -> Result<(), WriteError> {
        // Snapshot to release the &model borrow before per-entity emission.
        let items: Vec<_> = self.model.tessellated_items.iter().cloned().collect();
        self.tessellated_item_step_ids = vec![0; items.len()];
        // Phase 1: base items — faces depend on their `coordinates`.
        for (idx, item) in items.iter().enumerate() {
            match item {
                TessellatedItem::CoordinatesList(c) => {
                    self.tessellated_item_step_ids[idx] =
                        CoordinatesListHandler::write(self, c.clone())?;
                }
                TessellatedItem::TessellatedCurveSet(t) => {
                    self.tessellated_item_step_ids[idx] =
                        TessellatedCurveSetHandler::write(self, t.clone())?;
                }
                TessellatedItem::RepositionedTessellatedItem(r) => {
                    use crate::entities::tessellation::RepositionedTessellatedItemHandler;
                    self.tessellated_item_step_ids[idx] =
                        RepositionedTessellatedItemHandler::write(self, r.clone())?;
                }
                TessellatedItem::TessellatedGeometricSet(_)
                | TessellatedItem::TessellatedSolid(_)
                | TessellatedItem::TessellatedShell(_)
                | TessellatedItem::RepositionedTessellatedGeometricSet(_) => {}
            }
        }
        // Phase 2: faces + surface-sets — geometric sets reference these.
        let faces: Vec<_> = self.model.tessellated_faces.iter().cloned().collect();
        self.tessellated_face_step_ids = vec![0; faces.len()];
        for (idx, face) in faces.into_iter().enumerate() {
            self.tessellated_face_step_ids[idx] =
                ComplexTriangulatedFaceHandler::write(self, face)?;
        }
        let surface_sets: Vec<_> = self
            .model
            .tessellated_surface_sets
            .iter()
            .cloned()
            .collect();
        self.tessellated_surface_set_step_ids = vec![0; surface_sets.len()];
        for (idx, set) in surface_sets.into_iter().enumerate() {
            self.tessellated_surface_set_step_ids[idx] =
                ComplexTriangulatedSurfaceSetHandler::write(self, set)?;
        }
        // Phase 3: composite items — their refs resolve through phases 1-2.
        for (idx, item) in items.iter().enumerate() {
            match item {
                TessellatedItem::TessellatedGeometricSet(g) => {
                    self.tessellated_item_step_ids[idx] =
                        TessellatedGeometricSetHandler::write(self, g.clone())?;
                }
                TessellatedItem::TessellatedSolid(s) => {
                    self.tessellated_item_step_ids[idx] =
                        TessellatedSolidHandler::write(self, s.clone())?;
                }
                TessellatedItem::TessellatedShell(s) => {
                    self.tessellated_item_step_ids[idx] =
                        TessellatedShellHandler::write(self, s.clone())?;
                }
                TessellatedItem::RepositionedTessellatedGeometricSet(g) => {
                    use crate::parser::entity::Attribute;
                    use crate::writer::entity::{WriterBody, WriterEntity};
                    let location_step = self.emit_axis2_placement_3d(g.location)?;
                    let children: Vec<Attribute> = g
                        .children
                        .iter()
                        .map(|&r| Attribute::EntityRef(self.emit_tessellated_item_ref(r)))
                        .collect();
                    let n = self.fresh();
                    self.entities.push(WriterEntity {
                        id: n,
                        body: WriterBody::Complex {
                            parts: vec![
                                ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                                (
                                    "REPOSITIONED_TESSELLATED_ITEM".into(),
                                    vec![Attribute::EntityRef(location_step)],
                                ),
                                (
                                    "REPRESENTATION_ITEM".into(),
                                    vec![Attribute::String(g.name.clone())],
                                ),
                                (
                                    "TESSELLATED_GEOMETRIC_SET".into(),
                                    vec![Attribute::List(children)],
                                ),
                                ("TESSELLATED_ITEM".into(), vec![]),
                            ],
                        },
                    });
                    self.tessellated_item_step_ids[idx] = n;
                }
                TessellatedItem::CoordinatesList(_)
                | TessellatedItem::TessellatedCurveSet(_)
                | TessellatedItem::RepositionedTessellatedItem(_) => {}
            }
        }
        Ok(())
    }

    /// Resolve a [`TessellatedItemRef`] to its emitted STEP id — a cache
    /// lookup; `emit_tessellation` fills all three caches before any
    /// `TESSELLATED_GEOMETRIC_SET` (phase 3) emits.
    pub(crate) fn emit_tessellated_item_ref(&self, item: TessellatedItemRef) -> u64 {
        match item {
            TessellatedItemRef::Item(id) => self.tessellated_item_step_ids[id.0 as usize],
            TessellatedItemRef::Face(id) => self.tessellated_face_step_ids[id.0 as usize],
            TessellatedItemRef::SurfaceSet(id) => {
                self.tessellated_surface_set_step_ids[id.0 as usize]
            }
        }
    }
}
