//! Shape representation entity handlers (Pass 6-4 ~ 6-6, plus MDGPR at Pass 7-11).
//!
//! Plan 6 migrates the bespoke `run_pass!` blocks in `run_assembly_passes`
//! (`src/reader/passes.rs` line 195~256) into per-entity handlers. The
//! folder is new in Plan 6 — no carry-over from prior plans. Stubs land in
//! stage C1 and gain bodies in stages C4~C6.
//!
//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` and the
//! `PRODUCT_DEFINITION_SHAPE` classifier remain hand-rolled in
//! `src/reader/{assembly,passes}.rs` (`DOMAIN_TBD` markers there) — see Plan 6
//! `Hand-rolled 유지` table for the rationale.
//!
//! `MDGPR` (Pass 7-11) lives here despite being a visualization wrapper:
//! its dispatch boundary is `representation`, so the pool sits in
//! `shape_rep` per the ir.toml blueprint.

pub mod advanced_brep_shape_representation;
pub mod gbssr;
pub mod gbwsr;
pub mod item_defined_transformation;
pub mod manifold_surface_shape_representation;
pub mod mdgpr;
pub mod shape_representation;
pub mod shape_representation_relationship;
