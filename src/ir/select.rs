//! Single-source plumbing for "simple" STEP `SELECT` enums.
//!
//! A STEP EXPRESS `SELECT` over arena-id newtypes is modeled in step-io as an
//! enum whose every variant is `Variant(XId)`. `#[derive(StepSelect)]`
//! (step-io-macros) generates the reader `resolve_select` and writer
//! `emit_select` directly from that enum, so the member list lives in exactly
//! one place and the read side can no longer silently drift from the write
//! side (a missing reader probe used to drop a member with no compiler error).
//!
//! These two traits are the thin seam the generated code calls through. They
//! keep the `ir` layer free of direct `reader` / `writer` type names: the
//! generated impl lives next to the enum (in `ir`) and only ever names these
//! traits, while [`ReaderContext`](crate::reader::ReaderContext) and
//! [`WriteBuffer`](crate::writer::buffer::WriteBuffer) provide the impls.
//!
//! # Why some `Variant(XId)` SELECTs are NOT derived
//!
//! A few enums *look* derivable (every variant is `Variant(XId)`) but their
//! reader does more than sequential `id_cache.get::<XId>` probes, so the macro
//! would silently change behaviour. They keep hand-written resolvers:
//!
//! - **TypeId-collision named maps** — a member resolves through a named
//!   `HashMap<u64, XId>` field on `ReaderContext` (not `id_cache`) because its
//!   value type collides with a sibling map's in one `TypeId` bucket. The value
//!   *is* a plain arena id, so these are derivable only if `StepSelect` grows a
//!   per-variant "resolve via this map" escape. Affected:
//!   `Axis2Placement` / `PlanarBoxPlacement` (`placement_map`),
//!   `SurfaceSideStyleEntry` (`viz_ssfa_id_map`).
//! - **Typed-correspondence indirection** — a member resolves to a *different*
//!   arena type than its probe key: the L1 `product_of_pdef` / `product_of_pds`
//!   probes are `id_cache.get::<Early*Id>` + `lookup_lowered` → `ProductId`,
//!   not a plain `id_cache.get::<XId>`. (The former raw 2-hop step-id maps
//!   `pdef_to_product` / `formation_to_product` / `pdef_shape_to_pdef` are
//!   gone, but the one-probe form is still not the macro's sequential-probe
//!   shape.) Affected: `NameAttributeItem` (the `ProductDefinition` member).
//! - **Resolver-side logic / no benefit** — single-variant SELECTs carry no
//!   drift risk (nothing to forget), so deriving them is pure churn; some also
//!   wrap extra logic (`warnings.push`, dangling-ref checks). Affected:
//!   `PersonOrganizationSelect`, `DerivedDefinitionItem`, and the plm
//!   `Product(ProductId)` SELECTs (`GroupItem`, `ApprovalItem`, `DateTimeItem`,
//!   `IdentificationItem`, `DocumentReferenceItem`, `PersonOrganizationItem`,
//!   `SecurityClassificationItem` — these also resolve via the typed
//!   correspondence above).

use crate::ir::arena::{ArenaId, AsArenaId};

/// Reader seam: resolve a STEP file id (`#N`) to an arena id of type `K`.
/// Implemented by `ReaderContext` as a delegate to its `IdMapCache`.
pub(crate) trait IdResolver {
    fn resolve_arena_id<K: ArenaId>(&self, file_id: u64) -> Option<K>;
}

/// Writer seam: the emitted STEP id of an arena entry (`0` if not yet
/// emitted). Implemented by `WriteBuffer` as a delegate to its `StepIdCache`.
/// The bound mirrors `WriteBuffer::step_id` exactly (`AsArenaId`, accepting an
/// id by value or by reference).
pub(crate) trait StepResolver {
    fn step_of<K: AsArenaId>(&self, id: K) -> u64;
}
