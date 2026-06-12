//! PLM-domain `lower` fns (metadata leaf batch: roles, statuses, types,
//! contexts, groups). See the [module docs](super) for the lowering contract.
//!
//! All are 1:1 pass-throughs: the L2 types mirror the L1 shapes exactly
//! (including faithful `Option` descriptions), so each lower is a pool push
//! plus the `id_cache` registration. No `record_lowered` — consumers resolve
//! these through `id_cache` typed arena ids directly.

use crate::early::model::{
    EarlyAddress, EarlyApplicationContext, EarlyApplicationProtocolDefinition, EarlyApproval,
    EarlyApprovalDateTime, EarlyApprovalPersonOrganization, EarlyApprovalRole, EarlyApprovalStatus,
    EarlyCalendarDate, EarlyCoordinatedUniversalTimeOffset, EarlyDateAndTime, EarlyDateTimeRole,
    EarlyDocument, EarlyDocumentFile, EarlyDocumentProductEquivalence,
    EarlyDocumentRepresentationType, EarlyDocumentType, EarlyGroup, EarlyIdentificationRole,
    EarlyLocalTime, EarlyObjectRole, EarlyOrganization, EarlyPerson,
    EarlyPersonAndOrganizationRole, EarlyPersonalAddress, EarlyRoleAssociation,
    EarlySecurityClassification, EarlySecurityClassificationLevel,
};
use crate::ir::error::ConvertError;
use crate::ir::plm::{
    Address, AddressData, ApplicationContext, ApplicationProtocolDefinition, Approval,
    ApprovalDateTime, ApprovalDateTimeSelect, ApprovalPersonOrganization, ApprovalRole,
    ApprovalStatus, CalendarDate, CoordinatedUniversalTimeOffset, DateAndTime, DateTimeRole,
    Document, DocumentData, DocumentFile, DocumentProductEquivalence, DocumentProductItem,
    DocumentRepresentationType, DocumentType, Group, IdentificationRole, LocalTime, ObjectRole,
    Organization, Person, PersonAndOrganizationRole, PersonOrganizationSelect, PersonalAddress,
    PlmPool, RoleAssociation, RoleSelect, SecurityClassification, SecurityClassificationLevel,
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
