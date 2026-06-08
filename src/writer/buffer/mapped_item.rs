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
use crate::ir::id::{MappedItemId, RepresentationMapId};
use crate::ir::representation_item::RepresentationItemRef;
use crate::ir::shape_rep::{MappedItem, MappedRepresentationRef, RepresentationMap};
use crate::writer::WriteError;

// Snapshot indices are arena indices, which fit u32 by the arena's own
// overflow guard — the `idx as u32` id reconstructions are safe.
#[allow(clippy::cast_possible_truncation)]
impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_mapped_items(&mut self) -> Result<(), WriteError> {
        // Snapshot each arena to release the &model borrow before emission
        // (handler.write mutates self via push_simple). Only `Itself`
        // variants emit here — `CameraUsage` entries leave a 0 placeholder
        // and are filled in by `emit_camera_usage_arena` after
        // `emit_draughting_models` populates the DM slot of
        // `representation_step_ids`.
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        for (idx, rmap) in rmaps.into_iter().enumerate() {
            // A camera-origin Itself rmap needs `viz_camera_model_step_ids`
            // (deferred to `emit_camera_origin_mapped_items`); a
            // presentation-mapped one needs `presentation_representation_step_ids`
            // (deferred to `emit_presentation_mapped_items`). Leave the 0
            // placeholder here for both.
            if matches!(&rmap, RepresentationMap::Itself(d)
                if !matches!(d.mapping_origin, RepresentationItemRef::CameraModel(_))
                    && !matches!(d.mapped_representation, MappedRepresentationRef::Presentation(_)))
            {
                let step_id = RepresentationMapHandler::write(self, rmap)?;
                self.set_step_id(RepresentationMapId(idx as u32), step_id);
            }
        }
        // Only `Itself` MAPPED_ITEM emits here; CameraImage variants
        // forward-reference CameraUsage, so they emit later via
        // `emit_camera_image_arena` after the CameraUsage slot of
        // `representation_map_step_ids` is populated. A MAPPED_ITEM whose
        // source rmap is camera-origin is likewise deferred (its source slot is
        // still 0 here).
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for (idx, mi) in mitems.into_iter().enumerate() {
            if matches!(&mi, MappedItem::Itself(_))
                && !self.is_camera_sourced(&mi)
                && !self.is_presentation_sourced(&mi)
            {
                let step_id = MappedItemHandler::write(self, mi)?;
                self.set_step_id(MappedItemId(idx as u32), step_id);
            }
        }
        Ok(())
    }

    /// True iff `mi` is an `Itself` `MAPPED_ITEM` whose `mapping_source` rmap has
    /// a camera `mapping_origin` — such items are deferred to
    /// `emit_camera_origin_mapped_items` (their source rmap emits after cameras).
    fn is_camera_sourced(&self, mi: &MappedItem) -> bool {
        let MappedItem::Itself(d) = mi else {
            return false;
        };
        matches!(
            &self.model.representation_maps[d.mapping_source],
            RepresentationMap::Itself(rd)
                if matches!(rd.mapping_origin, RepresentationItemRef::CameraModel(_))
        )
    }

    /// True iff `mi` is an `Itself` `MAPPED_ITEM` whose `mapping_source` rmap
    /// maps a presentation representation — deferred to
    /// `emit_presentation_mapped_items` (its source rmap emits after the
    /// presentation cluster's `PRESENTATION_VIEW`s are stepped).
    fn is_presentation_sourced(&self, mi: &MappedItem) -> bool {
        let MappedItem::Itself(d) = mi else {
            return false;
        };
        matches!(
            &self.model.representation_maps[d.mapping_source],
            RepresentationMap::Itself(rd)
                if matches!(rd.mapped_representation, MappedRepresentationRef::Presentation(_))
        )
    }

    /// Delayed emission of presentation-mapped (`Itself`) `REPRESENTATION_MAP`s
    /// and the `MAPPED_ITEM`s sourced from them. Called from
    /// `emit_presentation_repr_cluster` after the `PRESENTATION_VIEW`s are
    /// stepped (so a `Presentation` `mapped_representation` resolves) and before
    /// the `PRESENTATION_AREA`s (whose `items` reference these `MAPPED_ITEM`s).
    /// Fills the 0 placeholders left by `emit_mapped_items`; rmaps first.
    pub(in crate::writer::buffer) fn emit_presentation_mapped_items(
        &mut self,
    ) -> Result<(), WriteError> {
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        for (idx, rmap) in rmaps.into_iter().enumerate() {
            if self.step_id(RepresentationMapId(idx as u32)) == 0
                && matches!(&rmap, RepresentationMap::Itself(d)
                    if matches!(d.mapped_representation, MappedRepresentationRef::Presentation(_)))
            {
                let step_id = RepresentationMapHandler::write(self, rmap)?;
                self.set_step_id(RepresentationMapId(idx as u32), step_id);
            }
        }
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for (idx, mi) in mitems.into_iter().enumerate() {
            if self.step_id(MappedItemId(idx as u32)) == 0 && self.is_presentation_sourced(&mi) {
                let step_id = MappedItemHandler::write(self, mi)?;
                self.set_step_id(MappedItemId(idx as u32), step_id);
            }
        }
        Ok(())
    }

    /// Delayed emission of camera-origin plain (`Itself`) `REPRESENTATION_MAP`s
    /// and the `MAPPED_ITEM`s sourced from them. Runs after
    /// `emit_visualization_if_set`
    /// (so a `CameraModel` origin resolves through `viz_camera_model_step_ids`)
    /// and before `emit_draughting_models` (whose MBD complex items index
    /// `mapped_item_step_ids`). Fills the 0 placeholders left by
    /// `emit_mapped_items`; rmaps first so each `MAPPED_ITEM`'s source resolves.
    pub(in crate::writer::buffer) fn emit_camera_origin_mapped_items(
        &mut self,
    ) -> Result<(), WriteError> {
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        for (idx, rmap) in rmaps.into_iter().enumerate() {
            if self.step_id(RepresentationMapId(idx as u32)) == 0
                && matches!(&rmap, RepresentationMap::Itself(d)
                    if matches!(d.mapping_origin, RepresentationItemRef::CameraModel(_)))
            {
                let step_id = RepresentationMapHandler::write(self, rmap)?;
                self.set_step_id(RepresentationMapId(idx as u32), step_id);
            }
        }
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for (idx, mi) in mitems.into_iter().enumerate() {
            if self.step_id(MappedItemId(idx as u32)) == 0 && self.is_camera_sourced(&mi) {
                let step_id = MappedItemHandler::write(self, mi)?;
                self.set_step_id(MappedItemId(idx as u32), step_id);
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
                self.set_step_id(RepresentationMapId(idx as u32), step_id);
            }
        }
        Ok(())
    }

    /// Delayed emission of `CAMERA_IMAGE` / `CAMERA_IMAGE_3D_WITH_SCALE` —
    /// runs after `emit_camera_usage_arena` so each entity's
    /// `mapping_source` (a `CameraUsage` slot) resolves to a non-zero STEP id.
    pub(in crate::writer::buffer) fn emit_camera_image_arena(&mut self) -> Result<(), WriteError> {
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for (idx, mi) in mitems.into_iter().enumerate() {
            match mi {
                MappedItem::Itself(_) => {}
                MappedItem::CameraImage(ci) => {
                    let step_id = CameraImageHandler::write(self, ci)?;
                    self.set_step_id(MappedItemId(idx as u32), step_id);
                }
                MappedItem::CameraImage3dWithScale(ci) => {
                    let step_id = CameraImage3dWithScaleHandler::write(self, ci)?;
                    self.set_step_id(MappedItemId(idx as u32), step_id);
                }
            }
        }
        Ok(())
    }
}
