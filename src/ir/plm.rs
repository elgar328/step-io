//! Product-lifecycle metadata pool.
//!
//! Hosts the AP214 "plm" entities — Person/Org/Date/Approval/Security
//! metadata that travels alongside the geometric/topological IR but
//! carries no shape semantics. Phase plm-1a covers the Date/Time
//! primitives (`CALENDAR_DATE`, `LOCAL_TIME`,
//! `COORDINATED_UNIVERSAL_TIME_OFFSET`, `DATE_AND_TIME`,
//! `DATE_TIME_ROLE`); subsequent phases extend the pool with assignment
//! enums and the Person / Approval / Security clusters.

use super::arena::Arena;
use super::id::{
    ApplicationContextId, ApprovalId, ApprovalRoleId, ApprovalStatusId,
    CoordinatedUniversalTimeOffsetId, DateAndTimeId, DateId, DateTimeRoleId, DocumentId,
    DocumentReferenceId, DocumentTypeId, ExternalSourceId, GroupId, IdentificationRoleId,
    LocalTimeId, ObjectRoleId, OrganizationId, PersonAndOrganizationId,
    PersonAndOrganizationRoleId, PersonId, ProductId, SecurityClassificationId,
    SecurityClassificationLevelId,
};
use step_io_macros::StepSelect;

/// Top-level container for plm-domain entities. `None` on
/// [`crate::ir::StepModel`] means the source file had no plm metadata
/// (or kernel adapter omitted it).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlmPool {
    /// `CALENDAR_DATE` entries — year/month/day triples.
    pub dates: Arena<CalendarDate>,
    /// `LOCAL_TIME` entries — hour/minute/second tuples with UTC zone.
    pub local_times: Arena<LocalTime>,
    /// `COORDINATED_UNIVERSAL_TIME_OFFSET` entries — leaf data.
    pub utc_offsets: Arena<CoordinatedUniversalTimeOffset>,
    /// `DATE_AND_TIME` entries — pair of (`calendar_date`, `local_time`) ids.
    pub date_and_times: Arena<DateAndTime>,
    /// `DATE_TIME_ROLE` entries — label entities (`creation_date` etc.).
    pub date_time_roles: Arena<DateTimeRole>,
    /// `date_and_time_assignment` arena enum covering both
    /// `APPLIED_DATE_AND_TIME_ASSIGNMENT` and
    /// `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT`. Connects Date primitives to
    /// product targets via the AP214 `date_time_item` SELECT.
    pub date_and_time_assignments: Arena<DateAndTimeAssignment>,
    /// `PERSON` entries.
    pub persons: Arena<Person>,
    /// `ORGANIZATION` entries.
    pub organizations: Arena<Organization>,
    /// `PERSON_AND_ORGANIZATION` entries pairing one Person + one Organization.
    pub person_and_organizations: Arena<PersonAndOrganization>,
    /// `PERSON_AND_ORGANIZATION_ROLE` label entries.
    pub p_and_o_roles: Arena<PersonAndOrganizationRole>,
    /// `person_and_organization_assignment` arena enum covering both
    /// `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` and
    /// `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT`. Connects
    /// `PersonAndOrganization` to product targets via the AP214
    /// `person_organization_item` SELECT.
    pub person_and_organization_assignments: Arena<PersonAndOrganizationAssignment>,
    /// `APPROVAL_STATUS` label entries.
    pub approval_statuses: Arena<ApprovalStatus>,
    /// `APPROVAL_ROLE` label entries.
    pub approval_roles: Arena<ApprovalRole>,
    /// `APPROVAL` entries — pair of (status, level).
    pub approvals: Arena<Approval>,
    /// `APPROVAL_DATE_TIME` entries — link `DateAndTime` to an `Approval`
    /// via AP214 `date_time_select`.
    pub approval_date_times: Arena<ApprovalDateTime>,
    /// `APPROVAL_PERSON_ORGANIZATION` entries — link a
    /// `PersonAndOrganization` to an `Approval` via AP214
    /// `person_organization_select`, tagged by `ApprovalRole`.
    pub approval_person_organizations: Arena<ApprovalPersonOrganization>,
    /// `approval_assignment` arena enum covering both
    /// `APPLIED_APPROVAL_ASSIGNMENT` and `CC_DESIGN_APPROVAL`. Connects
    /// an `Approval` to product targets via the AP214 `approval_item`
    /// SELECT.
    pub approval_assignments: Arena<ApprovalAssignment>,
    /// `SECURITY_CLASSIFICATION_LEVEL` label entries.
    pub security_classification_levels: Arena<SecurityClassificationLevel>,
    /// `SECURITY_CLASSIFICATION` entries — composite of (name, purpose,
    /// `security_level`).
    pub security_classifications: Arena<SecurityClassification>,
    /// `security_classification_assignment` arena enum covering both
    /// `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT` and
    /// `CC_DESIGN_SECURITY_CLASSIFICATION`. Connects a
    /// `SecurityClassification` to product targets via the AP214
    /// `security_classification_item` SELECT.
    pub security_classification_assignments: Arena<SecurityClassificationAssignment>,
    /// `IDENTIFICATION_ROLE` label entries.
    pub identification_roles: Arena<IdentificationRole>,
    /// `EXTERNAL_SOURCE` entries.
    pub external_sources: Arena<ExternalSource>,
    /// `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT` entries. The
    /// blueprint scopes the `identification_assignment` arena to this
    /// single subtype only (no enum wrapper).
    pub identification_assignments: Arena<AppliedExternalIdentificationAssignment>,
    /// `DOCUMENT_TYPE` label entries.
    pub document_types: Arena<DocumentType>,
    /// `document` arena enum covering plain `DOCUMENT` (`Itself`) and
    /// `DOCUMENT_FILE` (`in_enum` variant). `AP214e3` `DOCUMENT_FILE` 6-arg
    /// instances drop on read (only the inherited 4 fields are modeled).
    pub documents: Arena<Document>,
    /// `DOCUMENT_REPRESENTATION_TYPE` entries.
    pub document_representation_types: Arena<DocumentRepresentationType>,
    /// `DOCUMENT_PRODUCT_EQUIVALENCE` entries.
    pub document_product_equivalences: Arena<DocumentProductEquivalence>,
    /// `APPLIED_DOCUMENT_REFERENCE` entries. Connects a `Document` to
    /// product targets via the AP214 `document_reference_item` SELECT.
    pub document_references: Arena<AppliedDocumentReference>,
    /// `GROUP` label entries.
    pub groups: Arena<Group>,
    /// `APPLIED_GROUP_ASSIGNMENT` entries. Connects a `Group` to product
    /// targets via the AP214 `group_item` SELECT.
    pub group_assignments: Arena<AppliedGroupAssignment>,
    /// `OBJECT_ROLE` label entries — used by `ROLE_ASSOCIATION` to tag a
    /// targeted entity with a role string.
    pub object_roles: Arena<ObjectRole>,
    /// `ROLE_ASSOCIATION` entries — bind an `ObjectRole` to a target via
    /// the AP214 `role_select` SELECT.
    pub role_associations: Arena<RoleAssociation>,
    /// `address` arena. `Itself` is plain `ADDRESS`; `PersonalAddress`
    /// is the `PERSONAL_ADDRESS` subtype binding to a set of Persons.
    pub addresses: Arena<Address>,
    /// `APPLICATION_CONTEXT` arena. The writer emits the first entry
    /// (if any) in the assembly chain; additional entries drop silently
    /// (single-AC assumption matches the assembly emit pattern).
    pub application_contexts: Arena<ApplicationContext>,
    /// `APPLICATION_PROTOCOL_DEFINITION` arena. Same single-entry
    /// emit constraint as `application_contexts`.
    pub application_protocol_definitions: Arena<ApplicationProtocolDefinition>,
}

/// `OBJECT_ROLE(name, description)`. `AP214e3` schema lines 7533–7536.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRole {
    pub name: String,
    pub description: Option<String>,
}

/// `ROLE_ASSOCIATION(role, item_with_role)`. `AP214e3` schema lines
/// 9773–9776. `item_with_role` is the AP214 `role_select` SELECT —
/// step-io currently scopes to `APPLIED_DOCUMENT_REFERENCE` only;
/// other variants drop silently on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleAssociation {
    pub role: ObjectRoleId,
    pub item_with_role: RoleSelect,
}

/// AP214 `role_select` SELECT — currently scoped to
/// `APPLIED_DOCUMENT_REFERENCE`. Other variants (Approval / DTA / POA
/// 등) drop on read; future enhancement phase may extend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, StepSelect)]
pub enum RoleSelect {
    DocumentReference(DocumentReferenceId),
}

/// `GROUP(name, description)`. `AP214e3` schema lines 5785–5792.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub description: Option<String>,
}

/// `APPLIED_GROUP_ASSIGNMENT(assigned_group, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedGroupAssignment {
    pub assigned_group: GroupId,
    pub items: Vec<GroupItem>,
}

/// AP214 `group_item` SELECT — step-io scopes to the product chain.
/// `ACTION` / `SHAPE_ASPECT` / `GEOMETRIC_REPRESENTATION_ITEM` 등 비-PD
/// targets drop silently on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupItem {
    Product(ProductId),
}

/// `DOCUMENT_TYPE(product_data_type)` — label entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentType {
    pub product_data_type: String,
}

/// `document` arena enum per ir.toml. The `Itself` variant covers a
/// plain `DOCUMENT` instance; `DocumentFile` covers the `DOCUMENT_FILE`
/// subtype carrying identical field shape (`AP214e3` multi-supertype
/// trailing fields drop silently — see plm-6 plan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document {
    Itself(DocumentData),
    DocumentFile(DocumentFile),
}

/// Carrier for a plain `DOCUMENT(id, name, description, kind)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: DocumentTypeId,
}

/// `DOCUMENT_FILE(id, name, description, kind, name, description)` —
/// `SUBTYPE OF (document, characterized_object)`. The first four fields
/// come from the `document` supertype, the last two from
/// `characterized_object` (STEP P21 encodes all six in inheritance order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentFile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: DocumentTypeId,
    /// `characterized_object.name` — the second supertype's name.
    pub characterized_object_name: String,
    /// `characterized_object.description` — optional.
    pub characterized_object_description: Option<String>,
}

/// `DOCUMENT_REPRESENTATION_TYPE(name, represented_document)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRepresentationType {
    pub name: String,
    pub represented_document: DocumentId,
}

/// `DOCUMENT_PRODUCT_EQUIVALENCE(name, description, relating_document, related_product)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentProductEquivalence {
    pub name: String,
    pub description: Option<String>,
    pub relating_document: DocumentId,
    pub related_product: DocumentProductItem,
}

/// AP214 `product_or_formation_or_definition` SELECT — step-io scopes
/// to the product chain (`Product` / `Formation`). Other variants drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentProductItem {
    Product(ProductId),
    /// `related_product` pointed at a `PRODUCT_DEFINITION_FORMATION` directly
    /// — preserved so the writer re-emits the same formation ref the source
    /// used (rather than collapsing to the product).
    Formation(crate::ir::ProductDefinitionFormationId),
}

/// `APPLIED_DOCUMENT_REFERENCE(assigned_document, source, items)`.
/// `source` (string) is inherited from `document_reference` supertype
/// per `AP214e3` schema (lines 4163–4165).
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedDocumentReference {
    pub assigned_document: DocumentId,
    pub source: String,
    pub items: Vec<DocumentReferenceItem>,
}

/// AP214 `document_reference_item` SELECT — step-io scopes to the
/// product chain (`Product` only). Other variants drop silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentReferenceItem {
    Product(ProductId),
}

/// `IDENTIFICATION_ROLE(name, description)`. `description` is `opt_string`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentificationRole {
    pub name: String,
    pub description: Option<String>,
}

/// `EXTERNAL_SOURCE(source_id)`. AP214 `source_item` SELECT — step-io
/// supports the `IDENTIFIER` variant only; other variants drop on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSource {
    pub source_id: ExternalSourceItem,
}

/// AP214 `source_item` SELECT — single variant currently supported.
/// Corpus observation = `IDENTIFIER('...')` typed-value. `MESSAGE` and
/// other variants drop silently on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalSourceItem {
    Identifier(String),
}

/// `APPLIED_EXTERNAL_IDENTIFICATION_ASSIGNMENT(assigned_id, role, source, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedExternalIdentificationAssignment {
    pub assigned_id: String,
    pub role: IdentificationRoleId,
    pub source: ExternalSourceId,
    pub items: Vec<IdentificationItem>,
}

/// AP214 `identification_item` SELECT — same product-chain pattern as
/// `DateTimeItem` / `ApprovalItem` / `SecurityClassificationItem`. Other
/// targets (`APPLIED_ORGANIZATION_ASSIGNMENT`, `DOCUMENT_FILE`, ...)
/// drop silently on read pending later plm phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentificationItem {
    Product(ProductId),
}

/// `SECURITY_CLASSIFICATION_LEVEL(name)` — label entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityClassificationLevel {
    pub name: String,
}

/// `SECURITY_CLASSIFICATION(name, purpose, security_level)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityClassification {
    pub name: String,
    pub purpose: String,
    pub security_level: SecurityClassificationLevelId,
}

/// `security_classification_assignment` arena enum per ir.toml. Two
/// variants with identical field shape but distinct STEP entity names —
/// the `CcDesign` variant's STEP name lacks the `_ASSIGNMENT` suffix.
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityClassificationAssignment {
    Applied(AppliedSecurityClassificationAssignment),
    CcDesign(CcDesignSecurityClassification),
}

/// `APPLIED_SECURITY_CLASSIFICATION_ASSIGNMENT(assigned_security_classification, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedSecurityClassificationAssignment {
    pub assigned_security_classification: SecurityClassificationId,
    pub items: Vec<SecurityClassificationItem>,
}

/// `CC_DESIGN_SECURITY_CLASSIFICATION(assigned_security_classification, items)`.
/// STEP entity name lacks the `_ASSIGNMENT` suffix carried by the Applied sibling.
#[derive(Debug, Clone, PartialEq)]
pub struct CcDesignSecurityClassification {
    pub assigned_security_classification: SecurityClassificationId,
    pub items: Vec<SecurityClassificationItem>,
}

/// One element of a Security assignment's `items` set. Maps the AP214
/// `security_classification_item` SELECT — currently scoped to
/// `PRODUCT_DEFINITION` / `PRODUCT_DEFINITION_FORMATION` / `PRODUCT`
/// (all collapsed to the underlying `ProductId` via the assembly
/// product chain). Other variants drop silently on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityClassificationItem {
    Product(ProductId),
}

/// `approval_assignment` arena enum per ir.toml. Two variants with
/// identical field shape but distinct STEP entity names — the
/// `CcDesign` variant's STEP name lacks the `_ASSIGNMENT` suffix.
#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalAssignment {
    Applied(AppliedApprovalAssignment),
    CcDesign(CcDesignApproval),
}

/// `APPLIED_APPROVAL_ASSIGNMENT(assigned_approval, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedApprovalAssignment {
    pub assigned_approval: ApprovalId,
    pub items: Vec<ApprovalItem>,
}

/// `CC_DESIGN_APPROVAL(assigned_approval, items)`. STEP entity name lacks
/// the `_ASSIGNMENT` suffix that the Applied variant carries.
#[derive(Debug, Clone, PartialEq)]
pub struct CcDesignApproval {
    pub assigned_approval: ApprovalId,
    pub items: Vec<ApprovalItem>,
}

/// One element of an Approval assignment's `items` set. Maps the AP214
/// `approval_item` SELECT — currently scoped to `PRODUCT_DEFINITION` /
/// `PRODUCT` (resolved through the assembly product chain).
/// `PRODUCT_DEFINITION_FORMATION_*` / `SECURITY_CLASSIFICATION` /
/// `DOCUMENT` direct targets drop silently on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalItem {
    Product(ProductId),
}

/// `APPROVAL_STATUS(name)` — label entity (e.g. `"approved"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalStatus {
    pub name: String,
}

/// `APPROVAL_ROLE(role)` — label entity (e.g. `"approver"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRole {
    pub role: String,
}

/// `APPROVAL(status, level)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    pub status: ApprovalStatusId,
    pub level: String,
}

/// `APPROVAL_DATE_TIME(date_time, dated_approval)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalDateTime {
    pub date_time: ApprovalDateTimeSelect,
    pub dated_approval: ApprovalId,
}

/// AP214 `date_time_select` — step-io currently models the
/// `DATE_AND_TIME` variant only. Direct `CALENDAR_DATE` / `LOCAL_TIME`
/// targets drop silently at read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, StepSelect)]
pub enum ApprovalDateTimeSelect {
    DateAndTime(DateAndTimeId),
}

/// `APPROVAL_PERSON_ORGANIZATION(person_organization, authorized_approval, role)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalPersonOrganization {
    pub person_organization: PersonOrganizationSelect,
    pub authorized_approval: ApprovalId,
    pub role: ApprovalRoleId,
}

/// AP214 `person_organization_select` — step-io currently models the
/// `PERSON_AND_ORGANIZATION` variant only. Direct `PERSON` / `ORGANIZATION`
/// targets drop silently at read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonOrganizationSelect {
    PersonAndOrganization(PersonAndOrganizationId),
}

/// `person_and_organization_assignment` arena enum per ir.toml. The two
/// variants share field shape but differ in AP214 `ApplicationContext`.
#[derive(Debug, Clone, PartialEq)]
pub enum PersonAndOrganizationAssignment {
    Applied(AppliedPersonAndOrganizationAssignment),
    CcDesign(CcDesignPersonAndOrganizationAssignment),
}

/// `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT(assigned_person_and_organization,
/// role, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedPersonAndOrganizationAssignment {
    pub assigned_person_and_organization: PersonAndOrganizationId,
    pub role: PersonAndOrganizationRoleId,
    pub items: Vec<PersonOrganizationItem>,
}

/// `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(assigned_person_and_organization,
/// role, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct CcDesignPersonAndOrganizationAssignment {
    pub assigned_person_and_organization: PersonAndOrganizationId,
    pub role: PersonAndOrganizationRoleId,
    pub items: Vec<PersonOrganizationItem>,
}

/// One element of P&O assignment `items`. AP214 `person_organization_item`
/// SELECT — currently scoped to `PRODUCT_DEFINITION` / `PRODUCT` (resolved
/// through the assembly product chain). PDFWSS / `SECURITY_CLASSIFICATION`
/// / `APPROVAL` / `DOCUMENT` targets drop silently; future plm phases
/// extend this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonOrganizationItem {
    Product(ProductId),
}

/// `PERSON(id, last_name, first_name, middle_names, prefix_titles, suffix_titles)`.
/// `id` is required; the five trailing fields are STEP optionals (`$` → `None`,
/// `''`/`('')` → `Some("")` / `Some(vec![""])`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub last_name: Option<String>,
    pub first_name: Option<String>,
    pub middle_names: Option<Vec<String>>,
    pub prefix_titles: Option<Vec<String>>,
    pub suffix_titles: Option<Vec<String>>,
}

/// `ORGANIZATION(id, name, description)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
}

/// `PERSON_AND_ORGANIZATION(the_person, the_organization)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonAndOrganization {
    pub the_person: PersonId,
    pub the_organization: OrganizationId,
}

/// `PERSON_AND_ORGANIZATION_ROLE(name)` — label entity
/// (`design_owner`, `creator`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonAndOrganizationRole {
    pub name: String,
}

/// `date_and_time_assignment` arena enum per ir.toml. The two variants
/// carry identical field shape but differ in AP214 `ApplicationContext`.
#[derive(Debug, Clone, PartialEq)]
pub enum DateAndTimeAssignment {
    Applied(AppliedDateAndTimeAssignment),
    CcDesign(CcDesignDateAndTimeAssignment),
}

/// `APPLIED_DATE_AND_TIME_ASSIGNMENT(assigned_date_and_time, role, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedDateAndTimeAssignment {
    pub assigned_date_and_time: DateAndTimeId,
    pub role: DateTimeRoleId,
    pub items: Vec<DateTimeItem>,
}

/// `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT(assigned_date_and_time, role, items)`.
#[derive(Debug, Clone, PartialEq)]
pub struct CcDesignDateAndTimeAssignment {
    pub assigned_date_and_time: DateAndTimeId,
    pub role: DateTimeRoleId,
    pub items: Vec<DateTimeItem>,
}

/// One element of an assignment's `items` set. Maps the AP214
/// `date_time_item` SELECT — currently scoped to `PRODUCT_DEFINITION`
/// (resolved to the assembly pool's [`ProductId`]). Other source-side
/// variants (`SECURITY_CLASSIFICATION`, `APPROVAL`, `DOCUMENT`, ...) are
/// silently dropped on read; future plm phases extend this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeItem {
    Product(ProductId),
}

/// `CALENDAR_DATE(year_component, month_component, day_component)`.
/// All three are STEP `INTEGER`; carried as `i64` to match the parser's
/// `Attribute::Integer` width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDate {
    pub year_component: i64,
    pub month_component: i64,
    pub day_component: i64,
}

/// `COORDINATED_UNIVERSAL_TIME_OFFSET(hour_offset, minute_offset, sense)`.
/// `minute_offset` is `opt_integer` (`$` permitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatedUniversalTimeOffset {
    pub hour_offset: i64,
    pub minute_offset: Option<i64>,
    pub sense: AheadOrBehind,
}

/// `ahead_or_behind` enum for `COORDINATED_UNIVERSAL_TIME_OFFSET.sense`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AheadOrBehind {
    Ahead,
    Behind,
    Exact,
}

/// `LOCAL_TIME(hour_component, minute_component, second_component, zone)`.
/// `minute_component` and `second_component` are optional per the schema.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTime {
    pub hour_component: i64,
    pub minute_component: Option<i64>,
    pub second_component: Option<f64>,
    pub zone: CoordinatedUniversalTimeOffsetId,
}

/// `DATE_AND_TIME(date_component, time_component)` — references one
/// [`CalendarDate`] arena entry and one [`LocalTime`] arena entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateAndTime {
    pub date_component: DateId,
    pub time_component: LocalTimeId,
}

/// `DATE_TIME_ROLE(name)` — pure label entity (e.g. `"creation_date"`,
/// `"classification_date"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeRole {
    pub name: String,
}

/// `ADDRESS` inherited shape (12 OPTIONAL string fields, `AP214e3`
/// schema lines 175–192). Shared by `Address::Itself` and
/// `PersonalAddress`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddressData {
    pub internal_location: Option<String>,
    pub street_number: Option<String>,
    pub street: Option<String>,
    pub postal_box: Option<String>,
    pub town: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub facsimile_number: Option<String>,
    pub telephone_number: Option<String>,
    pub electronic_mail_address: Option<String>,
    pub telex_number: Option<String>,
}

/// `PERSONAL_ADDRESS` adds `(people, description)` to the inherited
/// `ADDRESS` fields. `AP214e3` schema; `description` is required text
/// and `people` is `SET[1:?] OF person`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalAddress {
    pub inherited: AddressData,
    pub people: Vec<PersonId>,
    pub description: String,
}

/// `address` arena enum per ir.toml. `Itself` covers a plain `ADDRESS`
/// instance (concrete supertype); `PersonalAddress` covers the
/// `PERSONAL_ADDRESS` subtype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    Itself(AddressData),
    PersonalAddress(PersonalAddress),
}

/// `APPLICATION_CONTEXT(application)`. `AP214e3` schema; the
/// `application` string is the free-form context description
/// (e.g. `"Core Data for Automotive Mechanical Design Process"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationContext {
    pub application: String,
}

/// `APPLICATION_PROTOCOL_DEFINITION(status,
/// application_interpreted_model_schema_name, application_protocol_year,
/// application)`. References one `ApplicationContext` arena entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationProtocolDefinition {
    pub status: String,
    pub application_interpreted_model_schema_name: String,
    pub application_protocol_year: i64,
    pub application: ApplicationContextId,
}
