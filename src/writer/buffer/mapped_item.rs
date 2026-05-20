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
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_mapped_items(&mut self) -> Result<(), WriteError> {
        // Snapshot each arena to release the &model borrow before emission
        // (handler.write mutates self via push_simple).
        let rmaps: Vec<_> = self.model.representation_maps.iter().cloned().collect();
        self.representation_map_step_ids = Vec::with_capacity(rmaps.len());
        for rmap in rmaps {
            let step_id = RepresentationMapHandler::write(self, rmap)?;
            self.representation_map_step_ids.push(step_id);
        }
        let mitems: Vec<_> = self.model.mapped_items.iter().cloned().collect();
        for mi in mitems {
            MappedItemHandler::write(self, mi)?;
        }
        Ok(())
    }
}
