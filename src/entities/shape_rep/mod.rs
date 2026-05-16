//! Shape representation entity handlers (Pass 6-4 ~ 6-6, plus
//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` at Pass 0-2, `MDGPR` at Pass 7-11, and
//! `SHAPE_ASPECT` at Pass 8-pre).
//!
//! Plan 6 migrates the bespoke `run_pass!` blocks in `run_assembly_passes`
//! (`src/reader/passes.rs` line 195~256) into per-entity handlers. The
//! folder is new in Plan 6 — no carry-over from prior plans. Stubs land in
//! stage C1 and gain bodies in stages C4~C6.
//!
//! `MDGPR` (Pass 7-11) lives here despite being a visualization wrapper:
//! its dispatch boundary is `representation`, so the pool sits in
//! `shape_rep` per the ir.toml blueprint.

pub mod advanced_brep_shape_representation;
pub mod gbssr;
pub mod gbwsr;
pub mod global_unit_assigned_context;
pub mod item_defined_transformation;
pub mod manifold_surface_shape_representation;
pub mod mdgpr;
pub mod shape_aspect;
pub mod shape_representation;
pub mod shape_representation_relationship;
