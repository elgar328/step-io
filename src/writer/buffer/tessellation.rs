//! Tessellation emission — `COORDINATES_LIST` + `TESSELLATED_CURVE_SET` +
//! `COMPLEX_TRIANGULATED_FACE` + `COMPLEX_TRIANGULATED_SURFACE_SET`. The
//! `tessellated_item` arena emits first (reader fills `COORDINATES_LIST`
//! ahead of `TESSELLATED_CURVE_SET`), populating the
//! `tessellated_item_step_ids` cache so every face / surface-set / curve-set
//! resolves its `coordinates` ref through it. Standalone orphan emit.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::tessellation::{
    ComplexTriangulatedFaceHandler, ComplexTriangulatedSurfaceSetHandler, CoordinatesListHandler,
    TessellatedCurveSetHandler,
};
use crate::ir::tessellation::TessellatedItem;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_tessellation(&mut self) -> Result<(), WriteError> {
        // Snapshot to release the &model borrow before per-entity emission.
        let items: Vec<_> = self.model.tessellated_items.iter().cloned().collect();
        self.tessellated_item_step_ids = Vec::with_capacity(items.len());
        for item in items {
            let step_id = match item {
                TessellatedItem::CoordinatesList(c) => CoordinatesListHandler::write(self, c)?,
                TessellatedItem::TessellatedCurveSet(t) => {
                    TessellatedCurveSetHandler::write(self, t)?
                }
            };
            self.tessellated_item_step_ids.push(step_id);
        }
        let faces: Vec<_> = self.model.tessellated_faces.iter().cloned().collect();
        for face in faces {
            ComplexTriangulatedFaceHandler::write(self, face)?;
        }
        let surface_sets: Vec<_> = self
            .model
            .tessellated_surface_sets
            .iter()
            .cloned()
            .collect();
        for set in surface_sets {
            ComplexTriangulatedSurfaceSetHandler::write(self, set)?;
        }
        Ok(())
    }
}
