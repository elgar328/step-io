//! Shape representation entity handlers (Pass 6-4 ~ 6-6).
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

pub mod advanced_brep_shape_representation;
pub mod gbssr;
pub mod gbwsr;
pub mod geometric_curve_set;
pub mod geometric_set;
pub mod item_defined_transformation;
pub mod manifold_surface_shape_representation;
pub mod shape_definition_representation;
pub mod shape_representation;
pub mod shape_representation_relationship;
pub mod shell_based_surface_model;
