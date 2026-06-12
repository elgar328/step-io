//! PLM-domain `lift` fns (metadata leaf batch). See the [module docs](super)
//! for the lifting contract. All are faithful pass-throughs — the L2 types
//! keep the schema's `Option` descriptions, so no synthesis is needed.

use crate::early::model::{
    EarlyAddress, EarlyApplicationContext, EarlyApplicationProtocolDefinition,
    EarlyAppliedApprovalAssignment, EarlyAppliedDateAndTimeAssignment,
    EarlyAppliedDocumentReference, EarlyAppliedExternalIdentificationAssignment,
    EarlyAppliedGroupAssignment, EarlyAppliedPersonAndOrganizationAssignment,
    EarlyAppliedSecurityClassificationAssignment, EarlyApproval, EarlyApprovalDateTime,
    EarlyApprovalPersonOrganization, EarlyApprovalRole, EarlyApprovalStatus, EarlyCalendarDate,
    EarlyCcDesignApproval, EarlyCcDesignDateAndTimeAssignment,
    EarlyCcDesignPersonAndOrganizationAssignment, EarlyCcDesignSecurityClassification,
    EarlyCoordinatedUniversalTimeOffset, EarlyDateAndTime, EarlyDateTimeRole, EarlyDocument,
    EarlyDocumentFile, EarlyDocumentProductEquivalence, EarlyDocumentRepresentationType,
    EarlyDocumentType, EarlyGroup, EarlyIdentificationRole, EarlyLocalTime, EarlyObjectRole,
    EarlyOrganization, EarlyPerson, EarlyPersonAndOrganization, EarlyPersonAndOrganizationRole,
    EarlyPersonalAddress, EarlyRoleAssociation, EarlySecurityClassification,
    EarlySecurityClassificationLevel,
};
use crate::ir::plm::{
    AddressData, ApplicationContext, ApprovalRole, ApprovalStatus, CalendarDate,
    CoordinatedUniversalTimeOffset, DateTimeRole, DocumentData, DocumentFile, DocumentType, Group,
    IdentificationRole, LocalTime, ObjectRole, Organization, Person, PersonAndOrganizationRole,
    PersonalAddress, SecurityClassification, SecurityClassificationLevel,
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

/// Lift one `LOCAL_TIME` (zone pre-resolved; optional minute/second pass
/// through faithfully).
pub(crate) fn lift_local_time(lt: LocalTime, zone: u64) -> EarlyLocalTime {
    EarlyLocalTime {
        hour_component: lt.hour_component,
        minute_component: lt.minute_component,
        second_component: lt.second_component,
        zone,
    }
}

/// Lift one `COORDINATED_UNIVERSAL_TIME_OFFSET` (pure pass-through; the
/// L2 already holds the `AheadOrBehind` enum the serialize hint emits).
pub(crate) fn lift_coordinated_universal_time_offset(
    utc: CoordinatedUniversalTimeOffset,
) -> EarlyCoordinatedUniversalTimeOffset {
    EarlyCoordinatedUniversalTimeOffset {
        hour_offset: utc.hour_offset,
        minute_offset: utc.minute_offset,
        sense: utc.sense,
    }
}

/// Lift one `PERSON` (faithful all-optional pass-through).
pub(crate) fn lift_person(p: Person) -> EarlyPerson {
    EarlyPerson {
        id: p.id,
        last_name: p.last_name,
        first_name: p.first_name,
        middle_names: p.middle_names,
        prefix_titles: p.prefix_titles,
        suffix_titles: p.suffix_titles,
    }
}

/// Lift one `ROLE_ASSOCIATION` (both refs pre-resolved — the SELECT side
/// via `emit_select`).
pub(crate) fn lift_role_association(role: u64, item_with_role: u64) -> EarlyRoleAssociation {
    EarlyRoleAssociation {
        role,
        item_with_role,
    }
}

/// Lift one `DOCUMENT_PRODUCT_EQUIVALENCE` (refs pre-resolved; the writer's
/// formation-or-product fallback runs before this).
pub(crate) fn lift_document_product_equivalence(
    name: String,
    description: Option<String>,
    relating_document: u64,
    related_product: u64,
) -> EarlyDocumentProductEquivalence {
    EarlyDocumentProductEquivalence {
        name,
        description,
        relating_document,
        related_product,
    }
}

/// Lift one plain `ADDRESS` (faithful 12-optional pass-through).
pub(crate) fn lift_address(d: AddressData) -> EarlyAddress {
    EarlyAddress {
        internal_location: d.internal_location,
        street_number: d.street_number,
        street: d.street,
        postal_box: d.postal_box,
        town: d.town,
        region: d.region,
        postal_code: d.postal_code,
        country: d.country,
        facsimile_number: d.facsimile_number,
        telephone_number: d.telephone_number,
        electronic_mail_address: d.electronic_mail_address,
        telex_number: d.telex_number,
    }
}

/// Lift one `PERSONAL_ADDRESS` (people pre-resolved to output step ids;
/// legacy always emitted `description` as a String, never `$`).
pub(crate) fn lift_personal_address(pa: PersonalAddress, people: Vec<u64>) -> EarlyPersonalAddress {
    let d = pa.inherited;
    EarlyPersonalAddress {
        internal_location: d.internal_location,
        street_number: d.street_number,
        street: d.street,
        postal_box: d.postal_box,
        town: d.town,
        region: d.region,
        postal_code: d.postal_code,
        country: d.country,
        facsimile_number: d.facsimile_number,
        telephone_number: d.telephone_number,
        electronic_mail_address: d.electronic_mail_address,
        telex_number: d.telex_number,
        people,
        description: Some(pa.description),
    }
}

/// Lift one `APPLICATION_PROTOCOL_DEFINITION` from pre-resolved fields (the
/// writer's context emitter resolves the AC step id; the synthesised
/// no-IR-context fallbacks construct the same shape).
pub(crate) fn lift_application_protocol_definition(
    status: String,
    application_interpreted_model_schema_name: String,
    application_protocol_year: i64,
    application: u64,
) -> EarlyApplicationProtocolDefinition {
    EarlyApplicationProtocolDefinition {
        status,
        application_interpreted_model_schema_name,
        application_protocol_year,
        application,
    }
}

/// Lift one `DOCUMENT_FILE` (kind pre-resolved; the `characterized_object`
/// slots map back to the flattened `name_2`/`description_2`).
pub(crate) fn lift_document_file(d: DocumentFile, kind: u64) -> EarlyDocumentFile {
    EarlyDocumentFile {
        id: d.id,
        name: d.name,
        description: Some(d.description),
        kind,
        name_2: d.characterized_object_name,
        description_2: d.characterized_object_description,
    }
}

/// Lift one `PERSON_AND_ORGANIZATION` (both refs pre-resolved).
pub(crate) fn lift_person_and_organization(
    the_person: u64,
    the_organization: u64,
) -> EarlyPersonAndOrganization {
    EarlyPersonAndOrganization {
        the_person,
        the_organization,
    }
}

/// Lift one `CC_DESIGN_APPROVAL` (refs pre-resolved).
pub(crate) fn lift_cc_design_approval(
    assigned_approval: u64,
    items: Vec<u64>,
) -> EarlyCcDesignApproval {
    EarlyCcDesignApproval {
        assigned_approval,
        items,
    }
}

/// Lift one `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_cc_design_date_and_time_assignment(
    assigned_date_and_time: u64,
    role: u64,
    items: Vec<u64>,
) -> EarlyCcDesignDateAndTimeAssignment {
    EarlyCcDesignDateAndTimeAssignment {
        assigned_date_and_time,
        role,
        items,
    }
}

/// Lift one `CC_DESIGN_SECURITY_CLASSIFICATION` (refs pre-resolved).
pub(crate) fn lift_cc_design_security_classification(
    assigned_security_classification: u64,
    items: Vec<u64>,
) -> EarlyCcDesignSecurityClassification {
    EarlyCcDesignSecurityClassification {
        assigned_security_classification,
        items,
    }
}

/// Lift one `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_cc_design_person_and_organization_assignment(
    assigned_person_and_organization: u64,
    role: u64,
    items: Vec<u64>,
) -> EarlyCcDesignPersonAndOrganizationAssignment {
    EarlyCcDesignPersonAndOrganizationAssignment {
        assigned_person_and_organization,
        role,
        items,
    }
}

/// Lift one `APPLIED_APPROVAL_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_approval_assignment(
    assigned_approval: u64,
    items: Vec<u64>,
) -> EarlyAppliedApprovalAssignment {
    EarlyAppliedApprovalAssignment {
        assigned_approval,
        items,
    }
}

/// Lift one `APPLIED_GROUP_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_group_assignment(
    assigned_group: u64,
    items: Vec<u64>,
) -> EarlyAppliedGroupAssignment {
    EarlyAppliedGroupAssignment {
        assigned_group,
        items,
    }
}

/// Lift one `APPLIED_DATE_AND_TIME_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_date_and_time_assignment(
    assigned_date_and_time: u64,
    role: u64,
    items: Vec<u64>,
) -> EarlyAppliedDateAndTimeAssignment {
    EarlyAppliedDateAndTimeAssignment {
        assigned_date_and_time,
        role,
        items,
    }
}

/// Lift one `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_security_classification_assignment(
    assigned_security_classification: u64,
    items: Vec<u64>,
) -> EarlyAppliedSecurityClassificationAssignment {
    EarlyAppliedSecurityClassificationAssignment {
        assigned_security_classification,
        items,
    }
}

/// Lift one `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_person_and_organization_assignment(
    assigned_person_and_organization: u64,
    role: u64,
    items: Vec<u64>,
) -> EarlyAppliedPersonAndOrganizationAssignment {
    EarlyAppliedPersonAndOrganizationAssignment {
        assigned_person_and_organization,
        role,
        items,
    }
}

/// Lift one `APPLIED_DOCUMENT_REFERENCE` (refs pre-resolved).
pub(crate) fn lift_applied_document_reference(
    assigned_document: u64,
    source: String,
    items: Vec<u64>,
) -> EarlyAppliedDocumentReference {
    EarlyAppliedDocumentReference {
        assigned_document,
        source,
        items,
    }
}

/// Lift one `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` (refs pre-resolved).
pub(crate) fn lift_applied_external_identification_assignment(
    role: u64,
    source: u64,
    assigned_id: String,
    items: Vec<u64>,
) -> EarlyAppliedExternalIdentificationAssignment {
    EarlyAppliedExternalIdentificationAssignment {
        assigned_id,
        role,
        source,
        items,
    }
}
