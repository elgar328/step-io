//! PLM-domain `lift` fns (metadata leaf batch). See the [module docs](super)
//! for the lifting contract. All are faithful pass-throughs — the L2 types
//! keep the schema's `Option` descriptions, so no synthesis is needed.

use crate::early::model::{
    EarlyApplicationContext, EarlyApprovalRole, EarlyApprovalStatus, EarlyDateTimeRole,
    EarlyDocumentType, EarlyGroup, EarlyIdentificationRole, EarlyObjectRole,
    EarlyPersonAndOrganizationRole, EarlySecurityClassificationLevel,
};
use crate::ir::plm::{
    ApplicationContext, ApprovalRole, ApprovalStatus, DateTimeRole, DocumentType, Group,
    IdentificationRole, ObjectRole, PersonAndOrganizationRole, SecurityClassificationLevel,
};

/// Lift one `APPROVAL_ROLE` from its arena entry.
pub(crate) fn lift_approval_role(v: ApprovalRole) -> EarlyApprovalRole {
    EarlyApprovalRole { role: v.role }
}

/// Lift one `APPROVAL_STATUS` from its arena entry.
pub(crate) fn lift_approval_status(v: ApprovalStatus) -> EarlyApprovalStatus {
    EarlyApprovalStatus { name: v.name }
}

/// Lift one `DATE_TIME_ROLE` from its arena entry.
pub(crate) fn lift_date_time_role(v: DateTimeRole) -> EarlyDateTimeRole {
    EarlyDateTimeRole { name: v.name }
}

/// Lift one `PERSON_AND_ORGANIZATION_ROLE` from its arena entry.
pub(crate) fn lift_person_and_organization_role(
    v: PersonAndOrganizationRole,
) -> EarlyPersonAndOrganizationRole {
    EarlyPersonAndOrganizationRole { name: v.name }
}

/// Lift one `DOCUMENT_TYPE` from its arena entry.
pub(crate) fn lift_document_type(v: DocumentType) -> EarlyDocumentType {
    EarlyDocumentType {
        product_data_type: v.product_data_type,
    }
}

/// Lift one `SECURITY_CLASSIFICATION_LEVEL` from its arena entry.
pub(crate) fn lift_security_classification_level(
    v: SecurityClassificationLevel,
) -> EarlySecurityClassificationLevel {
    EarlySecurityClassificationLevel { name: v.name }
}

/// Lift one `APPLICATION_CONTEXT` from its arena entry.
pub(crate) fn lift_application_context(v: ApplicationContext) -> EarlyApplicationContext {
    EarlyApplicationContext {
        application: v.application,
    }
}

/// Lift one `OBJECT_ROLE` from its arena entry.
pub(crate) fn lift_object_role(v: ObjectRole) -> EarlyObjectRole {
    EarlyObjectRole {
        name: v.name,
        description: v.description,
    }
}

/// Lift one `IDENTIFICATION_ROLE` from its arena entry.
pub(crate) fn lift_identification_role(v: IdentificationRole) -> EarlyIdentificationRole {
    EarlyIdentificationRole {
        name: v.name,
        description: v.description,
    }
}

/// Lift one `GROUP` from its arena entry.
pub(crate) fn lift_group(v: Group) -> EarlyGroup {
    EarlyGroup {
        name: v.name,
        description: v.description,
    }
}
