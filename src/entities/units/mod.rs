//! Units-group entity handlers (Pass 0 — runs before geometry passes).
//!
//! Plan 5.6 migrates the bespoke `run_unit_pass` (Pass 0-1 unit leaves
//! / Pass 0-1b uncertainty / Pass 0-2 GUAC) into per-entity handler
//! modules. Each module impls one of the registry traits and submits
//! itself via `#[linkme::distributed_slice(ENTITY_HANDLERS)]`.
//!
//! Scope spans two catalog groups:
//! - `units` group (`length_unit` / `plane_angle_unit` /
//!   `solid_angle_unit`, plus the part-only `si_unit` / `named_unit` /
//!   `conversion_based_unit` that the leaves inspect internally).
//! - `shape_rep` group's `global_unit_assigned_context` and
//!   `uncertainty_measure_with_unit` (`DOMAIN_TBD` markers attached) —
//!   they share Pass 0 dispatch with the unit leaves.

pub mod global_unit_assigned_context;
pub mod length_unit;
pub mod plane_angle_unit;
mod shared;
pub mod solid_angle_unit;
pub mod uncertainty_measure_with_unit;
