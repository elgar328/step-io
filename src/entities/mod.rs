//! Per-entity self-contained handlers + compile-time dispatch registry.
//!
//! Each entity lives in `src/entities/<group>/<name>.rs` and impls one of
//! [`SimpleEntityHandler`] (single-line `RawEntity::Simple`) or
//! [`ComplexEntityHandler`] (multi-part `RawEntity::Complex`). Plan 3 added
//! the complex variant so registry dispatch can cover the rational B-spline
//! family. Writer dispatch still goes through hand-rolled emit methods.

use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub mod geometry;

/// Reader pass ordering. Lower variants run first.
///
/// Plan 3 wires Pass1 (DIRECTION) / Pass2 (VECTOR) / `Pass4Rational`
/// (`RATIONAL_B_SPLINE_CURVE`). Other passes land here as Plan 4~7 walks
/// the remaining `run_pass!` blocks in `src/reader/passes.rs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassLevel {
    /// `CARTESIAN_POINT`, `DIRECTION` — no entity-ref dependencies.
    Pass1,
    /// `VECTOR` — depends on Pass1 entities.
    Pass2,
    /// `RATIONAL_B_SPLINE_CURVE` / `RATIONAL_B_SPLINE_SURFACE` — Pass 4-2,
    /// complex entities depending on Pass 4-1 leaf curves/surfaces.
    #[allow(dead_code)] // wired in Plan 3 stage 3 (RATIONAL_B_SPLINE_CURVE migration)
    Pass4Rational,
}

/// Handler for a [`RawEntity::Simple`] STEP entity. Reader receives a flat
/// attribute list; writer takes a per-entity [`Self::WriteInput`].
pub(crate) trait SimpleEntityHandler {
    /// Uppercase STEP entity name (e.g. `"DIRECTION"`).
    const NAME: &'static str;

    /// Reader pass level. See [`PassLevel`].
    const PASS_LEVEL: PassLevel;

    /// Writer input. Differs per entity (e.g. `DirectionId` for a directly
    /// stored arena entry, `(DirectionId, f64)` for vectors stored as a
    /// tuple).
    type WriteInput;

    /// Read STEP attributes into the reader context. Body mirrors the
    /// legacy `convert_*` method one-to-one — `&mut self` becomes `ctx`.
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input. Returns the freshly-allocated
    /// STEP entity id.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Handler for a [`RawEntity::Complex`] STEP entity. The reader receives a
/// list of [`RawEntityPart`] and dispatch keys on [`Self::REQUIRED_PARTS`]
/// (every listed part name must be present).
#[allow(dead_code)] // first impl lands in Plan 3 stage 3 (RATIONAL_B_SPLINE_CURVE)
pub(crate) trait ComplexEntityHandler {
    /// Metadata-only label (e.g. `"RATIONAL_B_SPLINE_CURVE"`). Dispatch
    /// keys on [`Self::REQUIRED_PARTS`], not on this name.
    const NAME: &'static str;

    /// Reader pass level. See [`PassLevel`].
    const PASS_LEVEL: PassLevel;

    /// Names that every matching `RawEntity::Complex` must contain among
    /// its parts. Order is irrelevant; a superset is fine.
    const REQUIRED_PARTS: &'static [&'static str];

    /// Writer input. Same role as the simple-handler associated type.
    type WriteInput;

    /// Read the complex parts into the reader context.
    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Distinguishes the two reader entry shapes inside a single
/// [`EntityHandlerEntry`] / [`ENTITY_HANDLERS`] slice.
#[allow(dead_code)] // variants and fields read by dispatch_entry in src/reader/passes.rs
pub(crate) enum ReadKind {
    /// Matches `RawEntity::Simple` whose name equals the entry's `name`.
    Simple {
        read: fn(&mut ReaderContext, u64, &[Attribute]) -> Result<(), ConvertError>,
    },
    /// Matches `RawEntity::Complex` whose parts contain every name in
    /// `required_parts`.
    Complex {
        required_parts: &'static [&'static str],
        read: fn(&mut ReaderContext, u64, &[RawEntityPart]) -> Result<(), ConvertError>,
    },
}

/// Reader-side registry entry. Each handler module submits one via
/// `#[linkme::distributed_slice(ENTITY_HANDLERS)]`. Writer is intentionally
/// absent — see Plan 3 for the type-erasure trade-off.
#[allow(dead_code)] // Fields are read by dispatch_entry in src/reader/passes.rs
pub(crate) struct EntityHandlerEntry {
    pub name: &'static str,
    pub pass_level: PassLevel,
    pub kind: ReadKind,
}

/// Compile-time registry of entity handlers contributing to reader
/// dispatch. Populated at link time by `#[distributed_slice]` on each
/// handler module's `static` entry.
#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice]
pub(crate) static ENTITY_HANDLERS: [EntityHandlerEntry] = [..];
