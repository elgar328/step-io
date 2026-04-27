//! Per-entity self-contained handlers + compile-time dispatch registry.
//!
//! Each entity lives in `src/entities/<group>/<name>.rs` and impls
//! [`EntityHandler`]. Plan 2 wires reader-side dispatch through the
//! [`ENTITY_HANDLERS`] distributed slice; writer dispatch still goes
//! through hand-rolled emit methods until Plan 3 decides how to
//! type-erase the per-entity `WriteInput`.

use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub mod geometry;

/// Reader pass ordering. Lower variants run first.
///
/// Plan 2 only wires Pass1 / Pass2 (DIRECTION / VECTOR). Later passes
/// land here as the migration walks the existing `run_pass!` blocks in
/// `src/reader/passes.rs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassLevel {
    /// `CARTESIAN_POINT`, `DIRECTION` — no entity-ref dependencies.
    Pass1,
    /// `VECTOR` — depends on Pass1 entities.
    Pass2,
}

/// Reader / writer logic + metadata for a single STEP entity.
///
/// During Plan 2 the trait is consumed in two paths:
/// 1. Reader: handlers contribute an [`EntityHandlerEntry`] to
///    [`ENTITY_HANDLERS`]; `src/reader/passes.rs` iterates the slice
///    per pass level.
/// 2. Writer: emit methods call `<Handler>::write` directly; the
///    registry does not yet cover the writer side.
pub(crate) trait EntityHandler {
    /// Uppercase STEP entity name (e.g. `"DIRECTION"`).
    const NAME: &'static str;

    /// Reader pass level. See [`PassLevel`].
    const PASS_LEVEL: PassLevel;

    /// Writer input. Differs per entity (e.g. `DirectionId` for a
    /// directly-stored arena entry, `(DirectionId, f64)` for a vector
    /// stored as an intermediate tuple).
    type WriteInput;

    /// Read STEP attributes into the reader context. The body mirrors
    /// the legacy `convert_*` method one-to-one — `&mut self` becomes
    /// `ctx`.
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input. Returns the freshly-allocated
    /// STEP entity id. Mirrors the legacy `emit_*` method body.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Reader-side registry entry. Each handler module submits one of
/// these via `#[linkme::distributed_slice(ENTITY_HANDLERS)]`.
///
/// Writer is intentionally absent — the per-entity `WriteInput`
/// associated type defies const fn-pointer storage. Plan 3 decides
/// the type-erasure approach (macro / generic / enum tag).
#[allow(dead_code)] // Fields are read by dispatch_registry once Plan 2 commit 2 lands.
pub(crate) struct EntityHandlerEntry {
    pub name: &'static str,
    pub pass_level: PassLevel,
    pub read: fn(&mut ReaderContext, u64, &[Attribute]) -> Result<(), ConvertError>,
}

/// Compile-time registry of entity handlers contributing to reader
/// dispatch. Populated at link time by the `#[distributed_slice]`
/// attribute on each handler module's `static` entry.
#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice]
pub(crate) static ENTITY_HANDLERS: [EntityHandlerEntry] = [..];
