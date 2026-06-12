//! PLM-domain `lift` fns (metadata leaf batch). See the [module docs](super)
//! for the lifting contract. All are faithful pass-throughs — the L2 types
//! keep the schema's `Option` descriptions, so no synthesis is needed.

use crate::early::model::{
    EarlyApplicationContext, EarlyApproval, EarlyApprovalDateTime, EarlyApprovalPersonOrganization,
    EarlyApprovalRole, EarlyApprovalStatus, EarlyCalendarDate, EarlyDateAndTime, EarlyDateTimeRole,
    EarlyDocument, EarlyDocumentRepresentationType, EarlyDocumentType, EarlyGroup,
    EarlyIdentificationRole, EarlyObjectRole, EarlyOrganization, EarlyPersonAndOrganizationRole,
    EarlySecurityClassification, EarlySecurityClassificationLevel,
};
use crate::ir::plm::{
    ApplicationContext, ApprovalRole, ApprovalStatus, CalendarDate, DateTimeRole, DocumentData,
    DocumentType, Group, IdentificationRole, ObjectRole, Organization, PersonAndOrganizationRole,
    SecurityClassification, SecurityClassificationLevel,
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

/// Lift one `APPROVAL` (status pre-resolved to its output step id).
pub(crate) fn lift_approval(status: u64, level: String) -> EarlyApproval {
    EarlyApproval { status, level }
}

/// Lift one `CALENDAR_DATE` — by-name pass-through (the L2 labels were
/// corrected to the EXPRESS order when the read side migrated; serialize
/// re-emits the schema slots, so output bytes are unchanged).
pub(crate) fn lift_calendar_date(d: CalendarDate) -> EarlyCalendarDate {
    EarlyCalendarDate {
        year_component: d.year_component,
        day_component: d.day_component,
        month_component: d.month_component,
    }
}

/// Lift one `DOCUMENT_REPRESENTATION_TYPE` (document pre-resolved).
pub(crate) fn lift_document_representation_type(
    name: String,
    represented_document: u64,
) -> EarlyDocumentRepresentationType {
    EarlyDocumentRepresentationType {
        name,
        represented_document,
    }
}

/// Lift one `ORGANIZATION` (faithful optional `id`; legacy always emitted
/// `description` as a String, never `$`).
pub(crate) fn lift_organization(o: Organization) -> EarlyOrganization {
    EarlyOrganization {
        id: o.id,
        name: o.name,
        description: Some(o.description),
    }
}

/// Lift one `DATE_AND_TIME` (both refs pre-resolved).
pub(crate) fn lift_date_and_time(date_component: u64, time_component: u64) -> EarlyDateAndTime {
    EarlyDateAndTime {
        date_component,
        time_component,
    }
}

/// Lift one `SECURITY_CLASSIFICATION` (level pre-resolved).
pub(crate) fn lift_security_classification(
    s: SecurityClassification,
    security_level: u64,
) -> EarlySecurityClassification {
    EarlySecurityClassification {
        name: s.name,
        purpose: s.purpose,
        security_level,
    }
}

/// Lift one plain `DOCUMENT` (kind pre-resolved; legacy always emitted
/// `description` as a String, never `$`).
pub(crate) fn lift_document(d: DocumentData, kind: u64) -> EarlyDocument {
    EarlyDocument {
        id: d.id,
        name: d.name,
        description: Some(d.description),
        kind,
    }
}

/// Lift one `APPROVAL_DATE_TIME` (both refs pre-resolved — the SELECT side
/// via `emit_select`).
pub(crate) fn lift_approval_date_time(
    date_time: u64,
    dated_approval: u64,
) -> EarlyApprovalDateTime {
    EarlyApprovalDateTime {
        date_time,
        dated_approval,
    }
}

/// Lift one `APPROVAL_PERSON_ORGANIZATION` (all three refs pre-resolved).
pub(crate) fn lift_approval_person_organization(
    person_organization: u64,
    authorized_approval: u64,
    role: u64,
) -> EarlyApprovalPersonOrganization {
    EarlyApprovalPersonOrganization {
        person_organization,
        authorized_approval,
        role,
    }
}
