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
use super::id::CoordinatedUniversalTimeOffsetId;
use super::id::{DateId, LocalTimeId};

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
