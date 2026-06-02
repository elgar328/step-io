//! `REPRESENTATION_MAP` + `MAPPED_ITEM` emission (phase mapped-item).
//!
//! Both arenas emit standalone after the visualization pass, in arena
//! order — `REPRESENTATION_MAP` first so each `MAPPED_ITEM` resolves its
//! `mapping_source` through the `representation_map_step_ids` cache. Running
//! after `emit_visualization_if_set` guarantees `representation_step_ids`
//! covers every representation (geometry reps + MDGPR slots) before a
//! `mapped_representation` ref is indexed.

use super::WriteBuffer;
use crate::entities::shape_rep::mapped_item::{MappedItemHandler, RepresentationMapHandler};
use crate::entities::visualization::camera_image::{
    CameraImage3dWithScaleHandler, CameraImageHandler,
};
use crate::entities::visualization::camera_usage::CameraUsageHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::shape_rep::{MappedItem, RepresentationMap};
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
        // Only `Itself` MAPPED_ITEM emits here; CameraImage variants
        // forward-reference CameraUsage, so they emit later via
        // `emit_camera_image_arena` after the CameraUsage slot of
        // `representation_map_step_ids` is populated.
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        self.mapped_item_step_ids = vec![0; mitems.len()];
        for (idx, mi) in mitems.into_iter().enumerate() {
            if matches!(mi, MappedItem::Itself(_)) {
                let step_id = MappedItemHandler::write(self, mi)?;
                self.mapped_item_step_ids[idx] = step_id;
            }
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

    /// Delayed emission of `CAMERA_IMAGE` / `CAMERA_IMAGE_3D_WITH_SCALE` —
    /// runs after `emit_camera_usage_arena` so each entity's
    /// `mapping_source` (a `CameraUsage` slot) resolves to a non-zero STEP id.
    pub(in crate::writer::buffer) fn emit_camera_image_arena(&mut self) -> Result<(), WriteError> {
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        if self.mapped_item_step_ids.len() != mitems.len() {
            self.mapped_item_step_ids.resize(mitems.len(), 0);
        }
        for (idx, mi) in mitems.into_iter().enumerate() {
            match mi {
                MappedItem::Itself(_) => {}
                MappedItem::CameraImage(ci) => {
                    let step_id = CameraImageHandler::write(self, ci)?;
                    self.mapped_item_step_ids[idx] = step_id;
                }
                MappedItem::CameraImage3dWithScale(ci) => {
                    let step_id = CameraImage3dWithScaleHandler::write(self, ci)?;
                    self.mapped_item_step_ids[idx] = step_id;
                }
            }
        }
        Ok(())
    }
}
