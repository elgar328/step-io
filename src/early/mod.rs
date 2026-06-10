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
//! - `serialize` / `lift` (write path) arrive in the Phase 4 write slice.
//!
//! **Status: first read slice — `{surface_style_fill_area, surface_side_style}`.**
//! These two entities migrate to the L1 path so that `surface_side_style` can
//! disambiguate its `FillArea` members by **L1 type** (a distinct
//! [`model::EarlySurfaceStyleFillAreaId`] bucket in the reader's `id_cache`)
//! instead of the bespoke `viz_ssfa_id_map` named field — eliminating the
//! TypeId-value collision that blocked `SurfaceSideStyleEntry` from
//! `#[derive(StepSelect)]` (the "#2 drift" pain). All other entities keep the
//! existing direct-`EntityGraph` read and coexist unchanged.
//!
//! `EarlyModel` is internal and **transient**: it lives on
//! [`ReaderContext`](crate::reader::ReaderContext) only for the duration of a
//! parse, holding just the read-side L1→L2 correspondence that a later entity
//! in the same topological pass needs. It is not a persistent store.

pub(crate) mod bind;
pub(crate) mod lower;
pub(crate) mod model;

pub(crate) use model::EarlyModel;
