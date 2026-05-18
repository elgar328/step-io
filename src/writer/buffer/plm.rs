//! plm-domain emission. Phase plm-1a emits the Date/Time primitives
//! in dependency order so each entry resolves its inner refs through
//! the cached step-id vectors on `WriteBuffer`.

use super::WriteBuffer;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_plm_if_set(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_date_and_time_assignment::AppliedDateAndTimeAssignmentHandler;
        use crate::entities::plm::calendar_date::CalendarDateHandler;
        use crate::entities::plm::cc_design_date_and_time_assignment::CcDesignDateAndTimeAssignmentHandler;
        use crate::entities::plm::coordinated_universal_time_offset::CoordinatedUniversalTimeOffsetHandler;
        use crate::entities::plm::date_and_time::DateAndTimeHandler;
        use crate::entities::plm::date_time_role::DateTimeRoleHandler;
        use crate::entities::plm::local_time::LocalTimeHandler;
        use crate::ir::plm::DateAndTimeAssignment;
        let Some(plm) = self.model.plm.clone() else {
            return Ok(());
        };
        // UTC offsets first — LocalTime carries a ref into this cache.
        self.plm_utc_step_ids = Vec::with_capacity(plm.utc_offsets.len());
        for utc in plm.utc_offsets.iter() {
            let id = CoordinatedUniversalTimeOffsetHandler::write(self, *utc)?;
            self.plm_utc_step_ids.push(id);
        }
        // Calendar dates — DateAndTime carries a ref into this cache.
        self.plm_date_step_ids = Vec::with_capacity(plm.dates.len());
        for d in plm.dates.iter() {
            let id = CalendarDateHandler::write(self, *d)?;
            self.plm_date_step_ids.push(id);
        }
        // Date-time roles — no consumers in plm-1a; cache populated for
        // Phase plm-1b's assignment writers.
        self.plm_date_time_role_step_ids = Vec::with_capacity(plm.date_time_roles.len());
        for role in plm.date_time_roles.iter() {
            let id = DateTimeRoleHandler::write(self, role.clone())?;
            self.plm_date_time_role_step_ids.push(id);
        }
        // Local times — read plm_utc_step_ids for the zone ref.
        self.plm_local_time_step_ids = Vec::with_capacity(plm.local_times.len());
        for lt in plm.local_times.iter() {
            let id = LocalTimeHandler::write(self, *lt)?;
            self.plm_local_time_step_ids.push(id);
        }
        // Date-and-time pairs — read plm_date_step_ids + plm_local_time_step_ids.
        self.plm_date_and_time_step_ids = Vec::with_capacity(plm.date_and_times.len());
        for dt in plm.date_and_times.iter() {
            let id = DateAndTimeHandler::write(self, *dt)?;
            self.plm_date_and_time_step_ids.push(id);
        }
        // Date-and-time assignments — top-level (no consumers), emit and
        // forget. Reads plm_date_and_time_step_ids + plm_date_time_role_step_ids
        // + product_def_ids for the items SELECT.
        for dta in plm.date_and_time_assignments.iter() {
            match dta {
                DateAndTimeAssignment::Applied(a) => {
                    AppliedDateAndTimeAssignmentHandler::write(self, a.clone())?;
                }
                DateAndTimeAssignment::CcDesign(c) => {
                    CcDesignDateAndTimeAssignmentHandler::write(self, c.clone())?;
                }
            }
        }
        Ok(())
    }
}
