//! Per-entity self-contained handlers (Step 1 pilot).
//!
//! Each entity lives in `src/entities/<group>/<name>.rs` and impls
//! [`EntityHandler`]. The trait carries reader + writer logic + minimal
//! metadata needed by future dispatch / registry machinery.
//!
//! Step 1 of `INFRA_PLAN.md` introduces this scaffolding with two pilot
//! entities (DIRECTION + VECTOR) to validate the trait shape against a
//! simple no-dependency case and a single-dependency intermediate-map
//! case before Plan 2 wires up a compile-time registry.

use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub mod geometry;

/// Reader / writer logic + metadata for a single STEP entity.
///
/// During Step 1 the trait is consumed manually inside
/// `src/reader/passes.rs` (the `run_pass!` macro) and
/// `src/writer/buffer/*.rs`. Plan 2 introduces a compile-time registry
/// that uses [`Self::NAME`] / [`Self::PASS_LEVEL`] for automatic dispatch.
pub(crate) trait EntityHandler {
    /// Uppercase STEP entity name (e.g. `"DIRECTION"`).
    /// Used by the Plan 2 registry; unused in Step 1 (allow until then).
    #[allow(dead_code)]
    const NAME: &'static str;

    /// Reader pass level. `1` = no dependency, `2` = depends on level-1
    /// entities, etc. Multi-round semantics may force this to evolve in
    /// Plan 2 (see `INFRA_PLAN.md` Step 1 evaluation note).
    #[allow(dead_code)]
    const PASS_LEVEL: u8;

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
