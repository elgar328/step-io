//! Units-group entity handlers (Pass 0 — runs before geometry passes).
//!
//! Plan 5.6 migrates the bespoke `run_unit_pass` (Pass 0-1 unit leaves
//! / Pass 0-1b uncertainty) into per-entity handler modules. Each module
//! impls one of the registry traits and submits itself via
//! `#[linkme::distributed_slice(ENTITY_HANDLERS)]`.
//!
//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` (Pass 0-2 orchestrator) lives in
//! `entities/shape_rep/` per the ir.toml blueprint — its arena is
//! `representation_context` even though dispatch runs at Pass 0.
//!
//! `uncertainty_measure_with_unit` keeps its `DOMAIN_TBD` marker — the
//! catalog `ENTITY_GROUPS.md` labels it under `shape_rep` but the
//! ir.toml pool is `units` (arena `measure_with_unit`), so the file
//! stays here. Catalog reconciliation is a separate task.

pub mod length_unit;
pub mod plane_angle_unit;
mod shared;
pub mod solid_angle_unit;
pub mod uncertainty_measure_with_unit;
