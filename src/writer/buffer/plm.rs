//! plm-domain emission. Phase plm-1a emits the Date/Time primitives
//! in dependency order so each entry resolves its inner refs through
//! the cached step-id vectors on `WriteBuffer`.

use super::WriteBuffer;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_plm_if_set(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_date_and_time_assignment::AppliedDateAndTimeAssignmentHandler;
        use crate::entities::plm::applied_person_and_organization_assignment::AppliedPersonAndOrganizationAssignmentHandler;
        use crate::entities::plm::calendar_date::CalendarDateHandler;
        use crate::entities::plm::cc_design_date_and_time_assignment::CcDesignDateAndTimeAssignmentHandler;
        use crate::entities::plm::cc_design_person_and_organization_assignment::CcDesignPersonAndOrganizationAssignmentHandler;
        use crate::entities::plm::coordinated_universal_time_offset::CoordinatedUniversalTimeOffsetHandler;
        use crate::entities::plm::date_and_time::DateAndTimeHandler;
        use crate::entities::plm::date_time_role::DateTimeRoleHandler;
        use crate::entities::plm::local_time::LocalTimeHandler;
        use crate::entities::plm::organization::OrganizationHandler;
        use crate::entities::plm::person::PersonHandler;
        use crate::entities::plm::person_and_organization::PersonAndOrganizationHandler;
        use crate::entities::plm::person_and_organization_role::PersonAndOrganizationRoleHandler;
        use crate::ir::plm::{DateAndTimeAssignment, PersonAndOrganizationAssignment};
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
        // Person/Org leaves. PersonAndOrganization needs Person + Organization
        // caches; PersonAndOrganizationRole is independent.
        self.plm_person_step_ids = Vec::with_capacity(plm.persons.len());
        for p in plm.persons.iter() {
            let id = PersonHandler::write(self, p.clone())?;
            self.plm_person_step_ids.push(id);
        }
        self.plm_organization_step_ids = Vec::with_capacity(plm.organizations.len());
        for o in plm.organizations.iter() {
            let id = OrganizationHandler::write(self, o.clone())?;
            self.plm_organization_step_ids.push(id);
        }
        self.plm_p_and_o_role_step_ids = Vec::with_capacity(plm.p_and_o_roles.len());
        for r in plm.p_and_o_roles.iter() {
            let id = PersonAndOrganizationRoleHandler::write(self, r.clone())?;
            self.plm_p_and_o_role_step_ids.push(id);
        }
        self.plm_p_and_o_step_ids = Vec::with_capacity(plm.person_and_organizations.len());
        for po in plm.person_and_organizations.iter() {
            let id = PersonAndOrganizationHandler::write(self, *po)?;
            self.plm_p_and_o_step_ids.push(id);
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
        // Person-and-organization assignments — top-level (no consumers),
        // same shape/pattern as DTA. Reads plm_p_and_o_step_ids +
        // plm_p_and_o_role_step_ids + product_def_ids.
        for poa in plm.person_and_organization_assignments.iter() {
            match poa {
                PersonAndOrganizationAssignment::Applied(a) => {
                    AppliedPersonAndOrganizationAssignmentHandler::write(self, a.clone())?;
                }
                PersonAndOrganizationAssignment::CcDesign(c) => {
                    CcDesignPersonAndOrganizationAssignmentHandler::write(self, c.clone())?;
                }
            }
        }
        self.emit_approval_cluster(&plm)?;
        self.emit_security_cluster(&plm)?;
        Ok(())
    }

    /// Emit the Security cluster (level leaf → classification →
    /// assignments). Split out of `emit_plm_if_set` for line-budget
    /// reasons, mirroring `emit_approval_cluster`.
    fn emit_security_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_security_classification_assignment::AppliedSecurityClassificationAssignmentHandler;
        use crate::entities::plm::cc_design_security_classification::CcDesignSecurityClassificationHandler;
        use crate::entities::plm::security_classification::SecurityClassificationHandler;
        use crate::entities::plm::security_classification_level::SecurityClassificationLevelHandler;
        use crate::ir::plm::SecurityClassificationAssignment;
        self.plm_security_level_step_ids =
            Vec::with_capacity(plm.security_classification_levels.len());
        for l in plm.security_classification_levels.iter() {
            let id = SecurityClassificationLevelHandler::write(self, l.clone())?;
            self.plm_security_level_step_ids.push(id);
        }
        self.plm_security_classification_step_ids =
            Vec::with_capacity(plm.security_classifications.len());
        for s in plm.security_classifications.iter() {
            let id = SecurityClassificationHandler::write(self, s.clone())?;
            self.plm_security_classification_step_ids.push(id);
        }
        for sca in plm.security_classification_assignments.iter() {
            match sca {
                SecurityClassificationAssignment::Applied(a) => {
                    AppliedSecurityClassificationAssignmentHandler::write(self, a.clone())?;
                }
                SecurityClassificationAssignment::CcDesign(c) => {
                    CcDesignSecurityClassificationHandler::write(self, c.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Emit the Approval cluster (status / role leaves -> `Approval` ->
    /// linkers). Split out of `emit_plm_if_set` for line-budget reasons.
    fn emit_approval_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_approval_assignment::AppliedApprovalAssignmentHandler;
        use crate::entities::plm::approval::ApprovalHandler;
        use crate::entities::plm::approval_date_time::ApprovalDateTimeHandler;
        use crate::entities::plm::approval_person_organization::ApprovalPersonOrganizationHandler;
        use crate::entities::plm::approval_role::ApprovalRoleHandler;
        use crate::entities::plm::approval_status::ApprovalStatusHandler;
        use crate::entities::plm::cc_design_approval::CcDesignApprovalHandler;
        use crate::ir::plm::ApprovalAssignment;
        self.plm_approval_status_step_ids = Vec::with_capacity(plm.approval_statuses.len());
        for s in plm.approval_statuses.iter() {
            let id = ApprovalStatusHandler::write(self, s.clone())?;
            self.plm_approval_status_step_ids.push(id);
        }
        self.plm_approval_role_step_ids = Vec::with_capacity(plm.approval_roles.len());
        for r in plm.approval_roles.iter() {
            let id = ApprovalRoleHandler::write(self, r.clone())?;
            self.plm_approval_role_step_ids.push(id);
        }
        self.plm_approval_step_ids = Vec::with_capacity(plm.approvals.len());
        for a in plm.approvals.iter() {
            let id = ApprovalHandler::write(self, a.clone())?;
            self.plm_approval_step_ids.push(id);
        }
        self.plm_approval_date_time_step_ids = Vec::with_capacity(plm.approval_date_times.len());
        for a in plm.approval_date_times.iter() {
            let id = ApprovalDateTimeHandler::write(self, *a)?;
            self.plm_approval_date_time_step_ids.push(id);
        }
        self.plm_approval_person_organization_step_ids =
            Vec::with_capacity(plm.approval_person_organizations.len());
        for a in plm.approval_person_organizations.iter() {
            let id = ApprovalPersonOrganizationHandler::write(self, *a)?;
            self.plm_approval_person_organization_step_ids.push(id);
        }
        // Approval assignments — top-level (no consumers), emit and forget.
        for aa in plm.approval_assignments.iter() {
            match aa {
                ApprovalAssignment::Applied(a) => {
                    AppliedApprovalAssignmentHandler::write(self, a.clone())?;
                }
                ApprovalAssignment::CcDesign(c) => {
                    CcDesignApprovalHandler::write(self, c.clone())?;
                }
            }
        }
        Ok(())
    }
}
