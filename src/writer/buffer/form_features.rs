//! `form_features` pool emission. Currently emits only `STEP` (the single
//! `single_struct` entity in the pool); future Phase 4 expands with the
//! remaining manufacturing-feature entities.

use super::WriteBuffer;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_form_features_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::form_features::step::StepHandler;
        let Some(pool) = self.model.form_features.clone() else {
            return Ok(());
        };
        for s in pool.feature_definitions.iter() {
            StepHandler::write(self, s.clone())?;
        }
        Ok(())
    }
}
