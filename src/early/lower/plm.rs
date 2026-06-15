//! PLM-domain `lower` fns (metadata leaf batch: roles, statuses, types,
//! contexts, groups). See the [module docs](super) for the lowering contract.
//!
//! All are 1:1 pass-throughs: the L2 types mirror the L1 shapes exactly
//! (including faithful `Option` descriptions), so each lower is a pool push
//! plus the `id_cache` registration. No `record_lowered` — consumers resolve
//! these through `id_cache` typed arena ids directly.

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
    EarlyDocumentType, EarlyExternalSource, EarlyGroup, EarlyIdentificationRole, EarlyLocalTime,
    EarlyObjectRole, EarlyOrganization, EarlyPerson, EarlyPersonAndOrganization,
    EarlyPersonAndOrganizationRole, EarlyPersonalAddress, EarlyRoleAssociation,
    EarlySecurityClassification, EarlySecurityClassificationLevel, EarlySourceItem,
};
use crate::ir::error::ConvertError;
use crate::ir::plm::{
    Address, AddressData, ApplicationContext, ApplicationProtocolDefinition,
    AppliedApprovalAssignment, AppliedDateAndTimeAssignment, AppliedDocumentReference,
    AppliedExternalIdentificationAssignment, AppliedGroupAssignment,
    AppliedPersonAndOrganizationAssignment, AppliedSecurityClassificationAssignment, Approval,
    ApprovalAssignment, ApprovalDateTime, ApprovalDateTimeSelect, ApprovalItem,
    ApprovalPersonOrganization, ApprovalRole, ApprovalStatus, CalendarDate, CcDesignApproval,
    CcDesignDateAndTimeAssignment, CcDesignPersonAndOrganizationAssignment,
    CcDesignSecurityClassification, CoordinatedUniversalTimeOffset, DateAndTime,
    DateAndTimeAssignment, DateTimeItem, DateTimeRole, Document, DocumentData, DocumentFile,
    DocumentProductEquivalence, DocumentProductItem, DocumentReferenceItem,
    DocumentRepresentationType, DocumentType, ExternalSource, ExternalSourceItem, Group, GroupItem,
    IdentificationItem, IdentificationRole, LocalTime, ObjectRole, Organization, Person,
    PersonAndOrganization, PersonAndOrganizationAssignment, PersonAndOrganizationRole,
    PersonOrganizationItem, PersonOrganizationSelect, PersonalAddress, PlmPool, RoleAssociation,
    RoleSelect, SecurityClassification, SecurityClassificationAssignment,
    SecurityClassificationItem, SecurityClassificationLevel,
};
use crate::reader::ReaderContext;

/// Lower one `APPROVAL_ROLE`.
pub(crate) fn lower_approval_role(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApprovalRole,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.approval_roles.push(ApprovalRole { role: early.role });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPROVAL_STATUS`.
pub(crate) fn lower_approval_status(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApprovalStatus,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .approval_statuses
        .push(ApprovalStatus { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DATE_TIME_ROLE`.
pub(crate) fn lower_date_time_role(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDateTimeRole,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.date_time_roles.push(DateTimeRole { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PERSON_AND_ORGANIZATION_ROLE`.
pub(crate) fn lower_person_and_organization_role(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPersonAndOrganizationRole,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .p_and_o_roles
        .push(PersonAndOrganizationRole { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DOCUMENT_TYPE`.
pub(crate) fn lower_document_type(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDocumentType,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.document_types.push(DocumentType {
        product_data_type: early.product_data_type,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SECURITY_CLASSIFICATION_LEVEL`.
pub(crate) fn lower_security_classification_level(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySecurityClassificationLevel,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .security_classification_levels
        .push(SecurityClassificationLevel { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLICATION_CONTEXT`.
pub(crate) fn lower_application_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApplicationContext,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.application_contexts.push(ApplicationContext {
        application: early.application,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `OBJECT_ROLE`.
pub(crate) fn lower_object_role(ctx: &mut ReaderContext, entity_id: u64, early: EarlyObjectRole) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.object_roles.push(ObjectRole {
        name: early.name,
        description: early.description,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `IDENTIFICATION_ROLE`.
pub(crate) fn lower_identification_role(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyIdentificationRole,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.identification_roles.push(IdentificationRole {
        name: early.name,
        description: early.description,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `GROUP`.
pub(crate) fn lower_group(ctx: &mut ReaderContext, entity_id: u64, early: EarlyGroup) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.groups.push(Group {
        name: early.name,
        description: early.description,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPROVAL` (unresolved status = silent drop, legacy leniency).
pub(crate) fn lower_approval(ctx: &mut ReaderContext, entity_id: u64, early: EarlyApproval) {
    let Some(status) = ctx
        .id_cache
        .get::<crate::ir::ApprovalStatusId>(early.status)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.approvals.push(Approval {
        status,
        level: early.level,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CALENDAR_DATE`. The L1 is schema-ordered (year, day, month);
/// the legacy handler read attr\[1\] as "month" and attr\[2\] as "day" — a
/// label swap vs EXPRESS. Mapping by *name* here fixes the L2 labels while
/// every emitted slot stays put (serialize re-emits in schema order), so the
/// output is byte-identical.
pub(crate) fn lower_calendar_date(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCalendarDate,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.dates.push(CalendarDate {
        year_component: early.year_component,
        month_component: early.month_component,
        day_component: early.day_component,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DOCUMENT_REPRESENTATION_TYPE` (unresolved document = silent drop).
pub(crate) fn lower_document_representation_type(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDocumentRepresentationType,
) {
    let Some(represented_document) = ctx
        .id_cache
        .get::<crate::ir::DocumentId>(early.represented_document)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .document_representation_types
        .push(DocumentRepresentationType {
            name: early.name,
            represented_document,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ORGANIZATION` (`id` stays faithfully optional; the legacy
/// read collapsed a `$` description to "" — L2 keeps a String).
pub(crate) fn lower_organization(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyOrganization,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let o_id = pool.organizations.push(Organization {
        id: early.id,
        name: early.name,
        description: early.description.unwrap_or_default(),
    });
    ctx.id_cache.insert(entity_id, o_id);
}

/// Lower one `DATE_AND_TIME` (either side unresolved = silent drop).
pub(crate) fn lower_date_and_time(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDateAndTime,
) {
    let Some(date_component) = ctx.id_cache.get::<crate::ir::DateId>(early.date_component) else {
        return;
    };
    let Some(time_component) = ctx
        .id_cache
        .get::<crate::ir::LocalTimeId>(early.time_component)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.date_and_times.push(DateAndTime {
        date_component,
        time_component,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `SECURITY_CLASSIFICATION` (unresolved level = silent drop).
pub(crate) fn lower_security_classification(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySecurityClassification,
) {
    let Some(security_level) = ctx
        .id_cache
        .get::<crate::ir::SecurityClassificationLevelId>(early.security_level)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.security_classifications.push(SecurityClassification {
        name: early.name,
        purpose: early.purpose,
        security_level,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DOCUMENT` (`Itself` carrier; unresolved kind = silent
/// drop. `DOCUMENT_FILE` keeps its own handler).
pub(crate) fn lower_document(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDocument) {
    let Some(kind) = ctx.id_cache.get::<crate::ir::DocumentTypeId>(early.kind) else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.documents.push(Document::Itself(DocumentData {
        id: early.id,
        name: early.name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
        kind,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPROVAL_DATE_TIME`. Members resolved by the `StepSelect`
/// derive; an unsupported variant (direct `CALENDAR_DATE` / `LOCAL_TIME`)
/// resolves to `None` and drops the entity (legacy leniency).
pub(crate) fn lower_approval_date_time(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApprovalDateTime,
) {
    let Some(date_time) = ApprovalDateTimeSelect::resolve_select(ctx, early.date_time) else {
        return;
    };
    let Some(dated_approval) = ctx
        .id_cache
        .get::<crate::ir::ApprovalId>(early.dated_approval)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.approval_date_times.push(ApprovalDateTime {
        date_time,
        dated_approval,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPROVAL_PERSON_ORGANIZATION`. A P&O dropped as a
/// dangling-reference cascade surfaces a `MissingReference` so the
/// dispatcher reclassifies this approval the same way
/// (NS-dangling-reference-drop); otherwise an unsupported SELECT variant
/// (direct PERSON / ORGANIZATION) drops silently.
pub(crate) fn lower_approval_person_organization(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApprovalPersonOrganization,
) -> Result<(), ConvertError> {
    let po_ref = early.person_organization;
    let Some(po_id) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationId>(po_ref)
    else {
        if ctx.nonstandard_dropped_refs.contains(&po_ref) {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: po_ref,
                field_name: "person_organization",
            });
        }
        return Ok(());
    };
    let Some(authorized_approval) = ctx
        .id_cache
        .get::<crate::ir::ApprovalId>(early.authorized_approval)
    else {
        return Ok(());
    };
    let Some(role) = ctx.id_cache.get::<crate::ir::ApprovalRoleId>(early.role) else {
        return Ok(());
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .approval_person_organizations
        .push(ApprovalPersonOrganization {
            person_organization: PersonOrganizationSelect::PersonAndOrganization(po_id),
            authorized_approval,
            role,
        });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `LOCAL_TIME` (faithful optional minute/second; unresolved zone
/// = silent drop, legacy leniency).
pub(crate) fn lower_local_time(ctx: &mut ReaderContext, entity_id: u64, early: EarlyLocalTime) {
    let Some(zone) = ctx
        .id_cache
        .get::<crate::ir::CoordinatedUniversalTimeOffsetId>(early.zone)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.local_times.push(LocalTime {
        hour_component: early.hour_component,
        minute_component: early.minute_component,
        second_component: early.second_component,
        zone,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COORDINATED_UNIVERSAL_TIME_OFFSET`. The `sense` token is
/// already an [`AheadOrBehind`](crate::ir::plm::AheadOrBehind) — the bind's
/// enum hint converts it (an unknown token now errors in bind, where the
/// legacy read silently dropped; no corpus file carries one).
pub(crate) fn lower_coordinated_universal_time_offset(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCoordinatedUniversalTimeOffset,
) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.utc_offsets.push(CoordinatedUniversalTimeOffset {
        hour_offset: early.hour_offset,
        minute_offset: early.minute_offset,
        sense: early.sense,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PERSON` (all-optional name parts pass through faithfully).
pub(crate) fn lower_person(ctx: &mut ReaderContext, entity_id: u64, early: EarlyPerson) {
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let p_id = pool.persons.push(Person {
        id: early.id,
        last_name: early.last_name,
        first_name: early.first_name,
        middle_names: early.middle_names,
        prefix_titles: early.prefix_titles,
        suffix_titles: early.suffix_titles,
    });
    ctx.id_cache.insert(entity_id, p_id);
}

/// Lower one `ROLE_ASSOCIATION`. Members resolved by the `StepSelect`
/// derive; an unsupported variant (Approval/DTA/etc.) resolves to `None`
/// and drops the entity (legacy leniency).
pub(crate) fn lower_role_association(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRoleAssociation,
) {
    let Some(role) = ctx.id_cache.get::<crate::ir::ObjectRoleId>(early.role) else {
        return;
    };
    let Some(item_with_role) = RoleSelect::resolve_select(ctx, early.item_with_role) else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.role_associations.push(RoleAssociation {
        role,
        item_with_role,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DOCUMENT_PRODUCT_EQUIVALENCE`. `related_product` may reference
/// a `PRODUCT_DEFINITION_FORMATION` directly (common for document-equivalence
/// links) — preserved so the formation ref round-trips; otherwise it
/// collapses to the product (unresolved = silent drop, legacy leniency).
pub(crate) fn lower_document_product_equivalence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDocumentProductEquivalence,
) {
    let Some(relating_document) = ctx
        .id_cache
        .get::<crate::ir::DocumentId>(early.relating_document)
    else {
        return;
    };
    let related_product = if let Some(fid) = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionFormationId>(early.related_product)
    {
        DocumentProductItem::Formation(fid)
    } else {
        let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, early.related_product)
        else {
            return;
        };
        DocumentProductItem::Product(pid)
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .document_product_equivalences
        .push(DocumentProductEquivalence {
            name: early.name,
            description: early.description,
            relating_document,
            related_product,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `ADDRESS` (`Itself` carrier — 12 faithful optional
/// fields; `PERSONAL_ADDRESS` keeps its own handler).
pub(crate) fn lower_address(ctx: &mut ReaderContext, entity_id: u64, early: EarlyAddress) {
    let data = AddressData {
        internal_location: early.internal_location,
        street_number: early.street_number,
        street: early.street,
        postal_box: early.postal_box,
        town: early.town,
        region: early.region,
        postal_code: early.postal_code,
        country: early.country,
        facsimile_number: early.facsimile_number,
        telephone_number: early.telephone_number,
        electronic_mail_address: early.electronic_mail_address,
        telex_number: early.telex_number,
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.addresses.push(Address::Itself(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PERSONAL_ADDRESS`. Schema mandates `people` SET\[1:?\]; an
/// empty resolved set means no Person ref survived (forward-ref drop or
/// unsupported variant) — drop the entry rather than emit a violating
/// empty (legacy leniency).
pub(crate) fn lower_personal_address(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPersonalAddress,
) {
    let people: Vec<crate::ir::PersonId> = early
        .people
        .iter()
        .filter_map(|r| ctx.id_cache.get::<crate::ir::PersonId>(*r))
        .collect();
    if people.is_empty() {
        return;
    }
    let inherited = AddressData {
        internal_location: early.internal_location,
        street_number: early.street_number,
        street: early.street,
        postal_box: early.postal_box,
        town: early.town,
        region: early.region,
        postal_code: early.postal_code,
        country: early.country,
        facsimile_number: early.facsimile_number,
        telephone_number: early.telephone_number,
        electronic_mail_address: early.electronic_mail_address,
        telex_number: early.telex_number,
    };
    let pa = PersonalAddress {
        inherited,
        people,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.addresses.push(Address::PersonalAddress(pa));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLICATION_PROTOCOL_DEFINITION` (unresolved AC = silent drop,
/// legacy leniency).
pub(crate) fn lower_application_protocol_definition(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApplicationProtocolDefinition,
) {
    let Some(application) = ctx
        .id_cache
        .get::<crate::ir::ApplicationContextId>(early.application)
    else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .application_protocol_definitions
        .push(ApplicationProtocolDefinition {
            status: early.status,
            application_interpreted_model_schema_name: early
                .application_interpreted_model_schema_name,
            application_protocol_year: early.application_protocol_year,
            application,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DOCUMENT_FILE` (`document` and `characterized_object`
/// multiple inheritance — the flattened L1 `name_2`/`description_2` are
/// the `characterized_object` slots; unresolved kind = silent drop).
pub(crate) fn lower_document_file(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDocumentFile,
) {
    let Some(kind) = ctx.id_cache.get::<crate::ir::DocumentTypeId>(early.kind) else {
        return;
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.documents.push(Document::DocumentFile(DocumentFile {
        id: early.id,
        name: early.name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
        kind,
        characterized_object_name: early.name_2,
        characterized_object_description: early.description_2,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PERSON_AND_ORGANIZATION`. A required person/organization ref
/// that dangles (anonymizers / grabcad scrub sentinels) surfaces as a
/// `MissingReference` so the dispatcher drops it as a dangling-reference
/// normalization and cascades (NS-dangling-reference-drop); a ref that *is*
/// defined but unmodelled is a separate gap (silent skip). The graph probe
/// stays in the handler (lower takes no `&EntityGraph`) — this fn receives
/// the pre-computed danglers.
pub(crate) fn lower_person_and_organization(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPersonAndOrganization,
    person_dangling: bool,
    org_dangling: bool,
) -> Result<(), ConvertError> {
    let person = ctx.id_cache.get::<crate::ir::PersonId>(early.the_person);
    let org = ctx
        .id_cache
        .get::<crate::ir::OrganizationId>(early.the_organization);
    let (Some(the_person), Some(the_organization)) = (person, org) else {
        if person_dangling {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: early.the_person,
                field_name: "the_person",
            });
        }
        if org_dangling {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: early.the_organization,
                field_name: "the_organization",
            });
        }
        return Ok(());
    };
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.person_and_organizations.push(PersonAndOrganization {
        the_person,
        the_organization,
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CC_DESIGN_APPROVAL` (unresolved approval = silent drop;
/// unresolved item members skip individually — legacy leniency).
pub(crate) fn lower_cc_design_approval(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCcDesignApproval,
) {
    let Some(assigned_approval) = ctx
        .id_cache
        .get::<crate::ir::ApprovalId>(early.assigned_approval)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(ApprovalItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .approval_assignments
        .push(ApprovalAssignment::CcDesign(CcDesignApproval {
            assigned_approval,
            items,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` (same leniencies).
pub(crate) fn lower_cc_design_date_and_time_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCcDesignDateAndTimeAssignment,
) {
    let Some(assigned_date_and_time) = ctx
        .id_cache
        .get::<crate::ir::DateAndTimeId>(early.assigned_date_and_time)
    else {
        return;
    };
    let Some(role) = ctx.id_cache.get::<crate::ir::DateTimeRoleId>(early.role) else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(DateTimeItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .date_and_time_assignments
        .push(DateAndTimeAssignment::CcDesign(
            CcDesignDateAndTimeAssignment {
                assigned_date_and_time,
                role,
                items,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CC_DESIGN_SECURITY_CLASSIFICATION` (same leniencies).
pub(crate) fn lower_cc_design_security_classification(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCcDesignSecurityClassification,
) {
    let Some(assigned_security_classification) = ctx
        .id_cache
        .get::<crate::ir::SecurityClassificationId>(early.assigned_security_classification)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(SecurityClassificationItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id =
        pool.security_classification_assignments
            .push(SecurityClassificationAssignment::CcDesign(
                CcDesignSecurityClassification {
                    assigned_security_classification,
                    items,
                },
            ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT`. A P&O dropped
/// as a dangling-reference cascade surfaces a `MissingReference`
/// (NS-dangling-reference-drop); otherwise unresolved = silent skip.
pub(crate) fn lower_cc_design_person_and_organization_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCcDesignPersonAndOrganizationAssignment,
) -> Result<(), ConvertError> {
    let po_ref = early.assigned_person_and_organization;
    let Some(assigned_person_and_organization) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationId>(po_ref)
    else {
        if ctx.nonstandard_dropped_refs.contains(&po_ref) {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: po_ref,
                field_name: "assigned_person_and_organization",
            });
        }
        return Ok(());
    };
    let Some(role) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationRoleId>(early.role)
    else {
        return Ok(());
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(PersonOrganizationItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id =
        pool.person_and_organization_assignments
            .push(PersonAndOrganizationAssignment::CcDesign(
                CcDesignPersonAndOrganizationAssignment {
                    assigned_person_and_organization,
                    role,
                    items,
                },
            ));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `APPLIED_APPROVAL_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_approval_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedApprovalAssignment,
) {
    let Some(assigned_approval) = ctx
        .id_cache
        .get::<crate::ir::ApprovalId>(early.assigned_approval)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(ApprovalItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id =
        pool.approval_assignments
            .push(ApprovalAssignment::Applied(AppliedApprovalAssignment {
                assigned_approval,
                items,
            }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_GROUP_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_group_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedGroupAssignment,
) {
    let Some(assigned_group) = ctx.id_cache.get::<crate::ir::GroupId>(early.assigned_group) else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(GroupItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.group_assignments.push(AppliedGroupAssignment {
        assigned_group,
        items,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_DATE_AND_TIME_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_date_and_time_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedDateAndTimeAssignment,
) {
    let Some(assigned_date_and_time) = ctx
        .id_cache
        .get::<crate::ir::DateAndTimeId>(early.assigned_date_and_time)
    else {
        return;
    };
    let Some(role) = ctx.id_cache.get::<crate::ir::DateTimeRoleId>(early.role) else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(DateTimeItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .date_and_time_assignments
        .push(DateAndTimeAssignment::Applied(
            AppliedDateAndTimeAssignment {
                assigned_date_and_time,
                role,
                items,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_security_classification_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedSecurityClassificationAssignment,
) {
    let Some(assigned_security_classification) = ctx
        .id_cache
        .get::<crate::ir::SecurityClassificationId>(early.assigned_security_classification)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(SecurityClassificationItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id =
        pool.security_classification_assignments
            .push(SecurityClassificationAssignment::Applied(
                AppliedSecurityClassificationAssignment {
                    assigned_security_classification,
                    items,
                },
            ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_person_and_organization_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedPersonAndOrganizationAssignment,
) {
    let Some(assigned_person_and_organization) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationId>(early.assigned_person_and_organization)
    else {
        return;
    };
    let Some(role) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationRoleId>(early.role)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(PersonOrganizationItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id =
        pool.person_and_organization_assignments
            .push(PersonAndOrganizationAssignment::Applied(
                AppliedPersonAndOrganizationAssignment {
                    assigned_person_and_organization,
                    role,
                    items,
                },
            ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_DOCUMENT_REFERENCE` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_document_reference(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedDocumentReference,
) {
    let Some(assigned_document) = ctx
        .id_cache
        .get::<crate::ir::DocumentId>(early.assigned_document)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(DocumentReferenceItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool.document_references.push(AppliedDocumentReference {
        assigned_document,
        source: early.source,
        items,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` (unresolved refs = silent drop; unresolved item
/// members skip individually — legacy leniency).
pub(crate) fn lower_applied_external_identification_assignment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAppliedExternalIdentificationAssignment,
) {
    let Some(role) = ctx
        .id_cache
        .get::<crate::ir::IdentificationRoleId>(early.role)
    else {
        return;
    };
    let Some(source) = ctx
        .id_cache
        .get::<crate::ir::ExternalSourceId>(early.source)
    else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(pid) = crate::entities::plm::resolve_date_time_item(ctx, r) {
            items.push(IdentificationItem::Product(pid));
        }
    }
    let pool = ctx.plm.get_or_insert_with(PlmPool::default);
    let id = pool
        .identification_assignments
        .push(AppliedExternalIdentificationAssignment {
            role,
            source,
            assigned_id: early.assigned_id,
            items,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `EXTERNAL_SOURCE`. The `source_item` SELECT models only the
/// `Identifier` member in L2; a `Message` member (unmodelled) drops the entity,
/// symmetric on re-read. Verbatim port of the legacy read.
pub(crate) fn lower_external_source(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyExternalSource,
) {
    let source_id = match &early.source_id {
        EarlySourceItem::Identifier(s) => ExternalSourceItem::Identifier(s.clone()),
        EarlySourceItem::Message(_) => return,
    };
    let id = ctx
        .plm
        .get_or_insert_with(PlmPool::default)
        .external_sources
        .push(ExternalSource { source_id });
    ctx.id_cache.insert(entity_id, id);
}
