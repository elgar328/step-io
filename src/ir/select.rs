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
