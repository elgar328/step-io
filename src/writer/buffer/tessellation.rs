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
    // Snapshot indices are arena indices, which fit u32 by the arena's own
    // overflow guard — the `idx as u32` id reconstructions are safe.
    #[allow(clippy::cast_possible_truncation)]
    pub(in crate::writer::buffer) fn emit_tessellation(&mut self) -> Result<(), WriteError> {
        // Snapshot to release the &model borrow before per-entity emission.
        let items: Vec<_> = self.model.tessellated_items.iter().cloned().collect();
        // Phase 1: base items — faces depend on their `coordinates`.
        for (idx, item) in items.iter().enumerate() {
            match item {
                TessellatedItem::CoordinatesList(c) => {
                    let __sid = CoordinatesListHandler::write(self, c.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
                }
                TessellatedItem::TessellatedCurveSet(t) => {
                    let __sid = TessellatedCurveSetHandler::write(self, t.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
                }
                TessellatedItem::RepositionedTessellatedItem(r) => {
                    use crate::entities::tessellation::RepositionedTessellatedItemHandler;
                    let __sid = RepositionedTessellatedItemHandler::write(self, r.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
                }
                TessellatedItem::TessellatedGeometricSet(_)
                | TessellatedItem::TessellatedSolid(_)
                | TessellatedItem::TessellatedShell(_)
                | TessellatedItem::RepositionedTessellatedGeometricSet(_) => {}
            }
        }
        // Phase 2: faces + surface-sets — geometric sets reference these.
        let faces: Vec<_> = self.model.tessellated_faces.iter().cloned().collect();
        for (idx, face) in faces.into_iter().enumerate() {
            let __sid = ComplexTriangulatedFaceHandler::write(self, face)?;
            self.set_step_id(crate::ir::id::TessellatedFaceId(idx as u32), __sid);
        }
        let surface_sets: Vec<_> = self
            .model
            .tessellated_surface_sets
            .iter()
            .cloned()
            .collect();
        for (idx, set) in surface_sets.into_iter().enumerate() {
            let __sid = ComplexTriangulatedSurfaceSetHandler::write(self, set)?;
            self.set_step_id(crate::ir::id::TessellatedSurfaceSetId(idx as u32), __sid);
        }
        // Phase 3: composite items — their refs resolve through phases 1-2.
        for (idx, item) in items.iter().enumerate() {
            match item {
                TessellatedItem::TessellatedGeometricSet(g) => {
                    let __sid = TessellatedGeometricSetHandler::write(self, g.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
                }
                TessellatedItem::TessellatedSolid(s) => {
                    let __sid = TessellatedSolidHandler::write(self, s.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
                }
                TessellatedItem::TessellatedShell(s) => {
                    let __sid = TessellatedShellHandler::write(self, s.clone())?;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
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
                    let __sid = n;
                    self.set_step_id(crate::ir::id::TessellatedItemId(idx as u32), __sid);
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
            TessellatedItemRef::Item(id) => self.step_id(id),
            TessellatedItemRef::Face(id) => self.step_id(id),
            TessellatedItemRef::SurfaceSet(id) => self.step_id(id),
        }
    }
}
