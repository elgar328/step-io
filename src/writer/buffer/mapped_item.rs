//! `REPRESENTATION_MAP` + `MAPPED_ITEM` emission (phase mapped-item).
//!
//! Both arenas emit standalone after the visualization pass, in arena
//! order — `REPRESENTATION_MAP` first so each `MAPPED_ITEM` resolves its
//! `mapping_source` through the `representation_map_step_ids` cache. Running
//! after `emit_visualization_if_set` guarantees `representation_step_ids`
//! covers every representation (geometry reps + MDGPR slots) before a
//! `mapped_representation` ref is indexed.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::mapped_item::{MappedItemHandler, RepresentationMapHandler};
use crate::entities::visualization::camera_usage::CameraUsageHandler;
use crate::ir::shape_rep::RepresentationMap;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_mapped_items(&mut self) -> Result<(), WriteError> {
        // Snapshot each arena to release the &model borrow before emission
        // (handler.write mutates self via push_simple). Only `Itself`
        // variants emit here — `CameraUsage` entries leave a 0 placeholder
        // and are filled in by `emit_camera_usage_arena` after
        // `emit_draughting_models` populates the DM slot of
        // `representation_step_ids`.
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        self.representation_map_step_ids = vec![0; rmaps.len()];
        for (idx, rmap) in rmaps.into_iter().enumerate() {
            if matches!(rmap, RepresentationMap::Itself(_)) {
                let step_id = RepresentationMapHandler::write(self, rmap)?;
                self.representation_map_step_ids[idx] = step_id;
            }
        }
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for mi in mitems {
            MappedItemHandler::write(self, mi)?;
        }
        Ok(())
    }

    /// Delayed emission of `CAMERA_USAGE` carriers — runs after
    /// `emit_draughting_models` so `representation_step_ids` covers DM
    /// slots before the `mapped_representation` index is resolved.
    /// Overwrites the 0 placeholder slot installed by `emit_mapped_items`.
    pub(in crate::writer::buffer) fn emit_camera_usage_arena(&mut self) -> Result<(), WriteError> {
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        for (idx, rmap) in rmaps.into_iter().enumerate() {
            if let RepresentationMap::CameraUsage(cu) = rmap {
                let step_id = CameraUsageHandler::write(self, cu)?;
                self.representation_map_step_ids[idx] = step_id;
            }
        }
        Ok(())
    }
}
