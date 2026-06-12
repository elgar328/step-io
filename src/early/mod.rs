//! `EarlyModel` — the early-bound, schema-faithful **L1** IR layer.
//!
//! Sits between the late-bound parser ([`EntityGraph`](crate::parser)) and the
//! ergonomic, kernel-facing **L2** ([`StepModel`](crate::ir)). The 2-layer
//! direction (step-io `internal/IR_LAYERING_DIRECTION.md`) splits the single
//! IR's two jobs — faithful schema union vs. kernel-friendly flatten — into
//! two layers:
//!
//! ```text
//! STEP ─parse─▶ EntityGraph ─bind─▶ EarlyModel(L1) ─lower─▶ StepModel(L2)
//!               (late-bound)         (early-bound, this)     (public, kept)
//! ```
//!
//! - [`bind`] : `EntityGraph` → L1 (mechanical attribute extraction).
//! - [`lower`]: L1 → L2 (the hand-written semantic glue; flatten / SELECT
//!   resolution / normalization live here).
//! - [`lift`]: L2 → L1 (write-side synthesis, the inverse of `lower`).
//! - [`serialize`]: L1 → Part21 text (mechanical).
//!
//! **Status:** the founded-item pilot cluster (6 visualization entities) runs
//! the full 2-layer path read+write, which eliminated all seven bespoke
//! `viz_*_id_map` reader fields (typed `Early*Id` `id_cache` keys instead).
//! `bind`/`serialize`/types/`Early*Id`s are **generated** by `gen-early`
//! (capable of the whole 2195-entity schema union; see `gen-early`'s docs),
//! while `lower`/`lift` stay hand-written, one submodule per domain. All other
//! entities keep the existing direct-`EntityGraph` handlers and coexist
//! unchanged until their cluster migrates.
//!
//! `EarlyModel` is internal and **transient**: it lives on
//! [`ReaderContext`](crate::reader::ReaderContext) only for the duration of a
//! parse, holding just the read-side L1→L2 correspondence that a later entity
//! in the same topological pass needs. It is not a persistent store.

pub(crate) mod bind;
pub(crate) mod generated;
pub(crate) mod lift;
pub(crate) mod lower;
pub(crate) mod model;
pub(crate) mod serialize;

pub(crate) use model::EarlyModel;
