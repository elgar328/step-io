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
    CoordinatedUniversalTimeOffsetId, DateAndTimeId, DateId, DateTimeRoleId, LocalTimeId,
    OrganizationId, PersonId, ProductId,
};

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
