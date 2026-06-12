//! PLM-domain `lower` fns (metadata leaf batch: roles, statuses, types,
//! contexts, groups). See the [module docs](super) for the lowering contract.
//!
//! All are 1:1 pass-throughs: the L2 types mirror the L1 shapes exactly
//! (including faithful `Option` descriptions), so each lower is a pool push
//! plus the `id_cache` registration. No `record_lowered` — consumers resolve
//! these through `id_cache` typed arena ids directly.

use crate::early::model::{
    EarlyApplicationContext, EarlyApprovalRole, EarlyApprovalStatus, EarlyDateTimeRole,
    EarlyDocumentType, EarlyGroup, EarlyIdentificationRole, EarlyObjectRole,
    EarlyPersonAndOrganizationRole, EarlySecurityClassificationLevel,
};
use crate::ir::plm::{
    ApplicationContext, ApprovalRole, ApprovalStatus, DateTimeRole, DocumentType, Group,
    IdentificationRole, ObjectRole, PersonAndOrganizationRole, PlmPool,
    SecurityClassificationLevel,
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
