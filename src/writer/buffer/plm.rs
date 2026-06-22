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
        for (__aid, utc) in plm.utc_offsets.iter_with_ids() {
            let id = CoordinatedUniversalTimeOffsetHandler::write(self, *utc)?;
            self.set_step_id(__aid, id);
        }
        // Calendar dates — DateAndTime carries a ref into this cache.
        for (__aid, d) in plm.dates.iter_with_ids() {
            let id = CalendarDateHandler::write(self, *d)?;
            self.set_step_id(__aid, id);
        }
        // Date-time roles — no consumers in plm-1a; cache populated for
        // Phase plm-1b's assignment writers.
        for (__aid, role) in plm.date_time_roles.iter_with_ids() {
            let id = DateTimeRoleHandler::write(self, role.clone())?;
            self.set_step_id(__aid, id);
        }
        // Local times — read plm_utc_step_ids for the zone ref.
        for (__aid, lt) in plm.local_times.iter_with_ids() {
            let id = LocalTimeHandler::write(self, *lt)?;
            self.set_step_id(__aid, id);
        }
        // Date-and-time pairs — read plm_date_step_ids + plm_local_time_step_ids.
        for (__aid, dt) in plm.date_and_times.iter_with_ids() {
            let id = DateAndTimeHandler::write(self, *dt)?;
            self.set_step_id(__aid, id);
        }
        // Person/Org leaves. PersonAndOrganization needs Person + Organization
        // caches; PersonAndOrganizationRole is independent.
        for (__aid, p) in plm.persons.iter_with_ids() {
            let id = PersonHandler::write(self, p.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, o) in plm.organizations.iter_with_ids() {
            let id = OrganizationHandler::write(self, o.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, r) in plm.p_and_o_roles.iter_with_ids() {
            let id = PersonAndOrganizationRoleHandler::write(self, r.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, po) in plm.person_and_organizations.iter_with_ids() {
            let id = PersonAndOrganizationHandler::write(self, *po)?;
            self.set_step_id(__aid, id);
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
        self.emit_identification_cluster(&plm)?;
        self.emit_document_cluster(&plm)?;
        self.emit_group_cluster(&plm)?;
        self.emit_role_cluster(&plm)?;
        self.emit_address_cluster(&plm)?;
        Ok(())
    }

    /// Emit the Address cluster (`ADDRESS` Itself + `PERSONAL_ADDRESS`).
    /// Both variants live in the same arena and emit top-level; their
    /// step ids are cached for future enhancement consumers.
    fn emit_address_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::address::AddressHandler;
        use crate::entities::plm::personal_address::PersonalAddressHandler;
        use crate::ir::plm::Address;
        for (__aid, addr) in plm.addresses.iter_with_ids() {
            let id = match addr {
                Address::Itself(_) => AddressHandler::write(self, addr.clone())?,
                Address::PersonalAddress(_) => PersonalAddressHandler::write(self, addr.clone())?,
            };
            self.set_step_id(__aid, id);
        }
        Ok(())
    }

    /// Emit the Group cluster (Group leaf → AGA). Split for line-budget
    /// reasons, mirroring the other cluster helpers.
    fn emit_group_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_group_assignment::AppliedGroupAssignmentHandler;
        use crate::entities::plm::group::GroupHandler;
        for (__aid, g) in plm.groups.iter_with_ids() {
            let id = GroupHandler::write(self, g.clone())?;
            self.set_step_id(__aid, id);
        }
        for aga in plm.group_assignments.iter() {
            AppliedGroupAssignmentHandler::write(self, aga.clone())?;
        }
        Ok(())
    }

    /// Emit the Document cluster (type leaf → document arena → linkers).
    /// `DOCUMENT` instances emit as `DOCUMENT` or `DOCUMENT_FILE` per
    /// the arena enum variant. Split for line-budget reasons.
    /// Emit `DOCUMENT_TYPE` + `DOCUMENT` / `DOCUMENT_FILE` and cache their
    /// step ids. Split out of [`emit_document_cluster`] so it can run before
    /// `emit_property_definitions_non_pds` — a PD.definition may target a
    /// `DOCUMENT_FILE`, so `plm_document_step_ids` must be filled first.
    pub(in crate::writer::buffer) fn emit_documents_prepass(
        &mut self,
        plm: &crate::ir::PlmPool,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::document::DocumentHandler;
        use crate::entities::plm::document_file::DocumentFileHandler;
        use crate::entities::plm::document_type::DocumentTypeHandler;
        use crate::ir::plm::Document;
        for (__aid, t) in plm.document_types.iter_with_ids() {
            let id = DocumentTypeHandler::write(self, t.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, d) in plm.documents.iter_with_ids() {
            let id = match d {
                Document::Itself(data) => DocumentHandler::write(self, data.clone())?,
                Document::DocumentFile(file) => DocumentFileHandler::write(self, file.clone())?,
            };
            self.set_step_id(__aid, id);
        }
        Ok(())
    }

    /// Emit the document linkers. `DOCUMENT_TYPE` / `DOCUMENT` / `DOCUMENT_FILE`
    /// step ids are filled earlier by [`emit_documents_prepass`].
    fn emit_document_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_document_reference::AppliedDocumentReferenceHandler;
        use crate::entities::plm::document_product_equivalence::DocumentProductEquivalenceHandler;
        use crate::entities::plm::document_representation_type::DocumentRepresentationTypeHandler;
        for (__aid, d) in plm.document_representation_types.iter_with_ids() {
            let id = DocumentRepresentationTypeHandler::write(self, d.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, d) in plm.document_product_equivalences.iter_with_ids() {
            let id = DocumentProductEquivalenceHandler::write(self, d.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, adr) in plm.document_references.iter_with_ids() {
            let id = AppliedDocumentReferenceHandler::write(self, adr.clone())?;
            self.set_step_id(__aid, id);
        }
        Ok(())
    }

    /// Emit the Role cluster (`ObjectRole` leaf -> `RoleAssociation`).
    /// `RoleAssociation` is top-level (no consumer); `ObjectRole`
    /// step-ids are cached for the association loop.
    fn emit_role_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::object_role::ObjectRoleHandler;
        use crate::entities::plm::role_association::RoleAssociationHandler;
        for (__aid, r) in plm.object_roles.iter_with_ids() {
            let id = ObjectRoleHandler::write(self, r.clone())?;
            self.set_step_id(__aid, id);
        }
        for ra in plm.role_associations.iter() {
            RoleAssociationHandler::write(self, *ra)?;
        }
        Ok(())
    }

    /// Emit the Identification cluster (role + `external_source` leaves →
    /// assignments). Split for line-budget reasons, mirroring the other
    /// cluster helpers.
    fn emit_identification_cluster(&mut self, plm: &crate::ir::PlmPool) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::applied_external_identification_assignment::AppliedExternalIdentificationAssignmentHandler;
        use crate::entities::plm::external_source::ExternalSourceHandler;
        use crate::entities::plm::identification_role::IdentificationRoleHandler;
        for (__aid, r) in plm.identification_roles.iter_with_ids() {
            let id = IdentificationRoleHandler::write(self, r.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, s) in plm.external_sources.iter_with_ids() {
            let id = ExternalSourceHandler::write(self, s.clone())?;
            self.set_step_id(__aid, id);
        }
        for ia in plm.identification_assignments.iter() {
            AppliedExternalIdentificationAssignmentHandler::write(self, ia.clone())?;
        }
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
        for (__aid, l) in plm.security_classification_levels.iter_with_ids() {
            let id = SecurityClassificationLevelHandler::write(self, l.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, s) in plm.security_classifications.iter_with_ids() {
            let id = SecurityClassificationHandler::write(self, s.clone())?;
            self.set_step_id(__aid, id);
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
        for (__aid, s) in plm.approval_statuses.iter_with_ids() {
            let id = ApprovalStatusHandler::write(self, s.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, r) in plm.approval_roles.iter_with_ids() {
            let id = ApprovalRoleHandler::write(self, r.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, a) in plm.approvals.iter_with_ids() {
            let id = ApprovalHandler::write(self, a.clone())?;
            self.set_step_id(__aid, id);
        }
        for (__aid, a) in plm.approval_date_times.iter_with_ids() {
            let id = ApprovalDateTimeHandler::write(self, *a)?;
            self.set_step_id(__aid, id);
        }
        for (__aid, a) in plm.approval_person_organizations.iter_with_ids() {
            let id = ApprovalPersonOrganizationHandler::write(self, *a)?;
            self.set_step_id(__aid, id);
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
