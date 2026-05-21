//! Tessellation emission — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`
//! (phase tessellation). `COORDINATES_LIST` emits first so each
//! `COMPLEX_TRIANGULATED_FACE` resolves its `coordinates` ref through the
//! `tessellated_item_step_ids` cache. Standalone orphan emit.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::tessellation::{ComplexTriangulatedFaceHandler, CoordinatesListHandler};
use crate::ir::tessellation::TessellatedItem;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_tessellation(&mut self) -> Result<(), WriteError> {
        // Snapshot to release the &model borrow before per-entity emission.
        let items: Vec<_> = self.model.tessellated_items.iter().cloned().collect();
        self.tessellated_item_step_ids = Vec::with_capacity(items.len());
        for item in items {
            match item {
                TessellatedItem::CoordinatesList(c) => {
                    let step_id = CoordinatesListHandler::write(self, c)?;
                    self.tessellated_item_step_ids.push(step_id);
                }
            }
        }
        let faces: Vec<_> = self.model.tessellated_faces.iter().cloned().collect();
        for face in faces {
            ComplexTriangulatedFaceHandler::write(self, face)?;
        }
        Ok(())
    }
}
