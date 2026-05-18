//! Units-group entity handlers (Pass 0 — runs before geometry passes).
//! Pass 0-1 unit leaves + Pass 0-1b uncertainty. Each handler impls
//! [`crate::entities::SimpleEntityHandler`] or `ComplexEntityHandler` and
//! registers via the `#[step_entity]` / `#[step_entity_complex]`
//! proc-macro attributes from `step_io_macros`.
//!
//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` (Pass 0-2 orchestrator) lives in
//! `entities/shape_rep/` per the ir.toml blueprint — its arena is
//! `representation_context` even though dispatch runs at Pass 0.
//!
//! `uncertainty_measure_with_unit` keeps its `DOMAIN_TBD` marker — the
//! catalog `ENTITY_GROUPS.md` labels it under `shape_rep` but the
//! ir.toml pool is `units` (arena `measure_with_unit`), so the file
//! stays here. Catalog reconciliation is a separate task.

pub mod derived_unit;
pub mod derived_unit_element;
pub mod length_measure_with_unit;
pub mod length_unit;
pub mod mass_measure_with_unit;
pub mod mass_unit;
pub mod plane_angle_measure_with_unit;
pub mod plane_angle_unit;
pub mod ratio_measure_with_unit;
mod shared;
pub mod solid_angle_unit;
pub mod uncertainty_measure_with_unit;
