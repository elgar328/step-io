//! Visualization emission entry point. Plan 7 stage C2~C4 lifted every
//! emit body into `entities/visualization/<name>.rs` (the per-entity
//! handler chain). This file remains as a single dispatcher so
//! `emit_all` keeps a stable entry — analogous to the `emit_unit_context`
//! / `emit_face` wrappers in units / topology.

use super::WriteBuffer;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::mdgpr::MdgprHandler;
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        for mdgpr in viz.mdgprs {
            MdgprHandler::write(self, mdgpr)?;
        }
        Ok(())
    }
}
