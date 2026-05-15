//! Assembly + product entity handlers (Pass 6-1 ~ 6-3, 6-8).
//!
//! Plan 6 migrates the bespoke `run_pass!` blocks in `run_assembly_passes`
//! (`src/reader/passes.rs` line 178~282) into per-entity handlers. Each
//! module impls one of the registry traits and submits itself via
//! `#[linkme::distributed_slice(ENTITY_HANDLERS)]`. Stubs land in stage C1
//! (skeleton commit) and gain bodies in stages C2~C7.

pub mod next_assembly_usage_occurrence;
pub mod product;
pub mod product_category;
pub mod product_category_relationship;
pub mod product_definition;
pub mod product_definition_formation;
pub mod product_definition_formation_with_source;
pub mod product_definition_with_associated_documents;
pub mod product_related_product_category;
pub(crate) mod shared;
