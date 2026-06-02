//! Per-entity self-contained handlers + compile-time dispatch registry.
//!
//! Each entity lives in `src/entities/<group>/<name>.rs` and impls one of
//! [`SimpleEntityHandler`] (single-line `RawEntity::Simple`) or
//! [`ComplexEntityHandler`] (multi-part `RawEntity::Complex`). The complex
//! variant lets registry dispatch cover the rational B-spline family.
//! Writer dispatch still goes through hand-rolled emit methods.

use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub mod assembly_product;
pub mod form_features;
pub mod geometry;
pub mod plm;
pub mod pmi;
pub mod property;
pub mod shape_rep;
pub mod tessellation;
pub mod topology;
pub mod units;
pub mod visualization;

/// Handler for a [`RawEntity::Simple`] STEP entity. Reader receives a flat
/// attribute list; writer takes a per-entity [`Self::WriteInput`].
pub(crate) trait SimpleEntityHandler {
    /// Uppercase STEP entity name (e.g. `"DIRECTION"`).
    const NAME: &'static str;

    /// Writer input. Differs per entity (e.g. `DirectionId` for a directly
    /// stored arena entry, `(DirectionId, f64)` for vectors stored as a
    /// tuple).
    type WriteInput;

    /// Read STEP attributes into the reader context. Body mirrors the
    /// legacy `convert_*` method one-to-one — `&mut self` becomes `ctx`.
    ///
    /// `graph` exposes the raw entity graph for sub-entity / ref-list
    /// resolution. Pass-produced results (other handlers' IR output, per-pass
    /// maps) must be reached through `ctx`, not through `graph`; consulting
    /// the graph for another entity's *result* (vs its raw attributes) would
    /// breach pass ordering.
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input. Returns the freshly-allocated
    /// STEP entity id.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Handler for a [`RawEntity::Complex`] STEP entity. The reader receives a
/// list of [`RawEntityPart`] and dispatch keys on [`Self::REQUIRED_PARTS`]
/// (every listed part name must be present).
pub(crate) trait ComplexEntityHandler {
    /// Metadata-only label (e.g. `"RATIONAL_B_SPLINE_CURVE"`). Dispatch
    /// keys on [`Self::REQUIRED_PARTS`], not on this name.
    const NAME: &'static str;

    /// Writer input. Same role as the simple-handler associated type.
    type WriteInput;

    /// Read the complex parts into the reader context. See
    /// [`SimpleEntityHandler::read`] for the `graph` usage contract.
    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Distinguishes the two reader entry shapes inside a single
/// [`EntityHandlerEntry`] / [`ENTITY_HANDLERS`] slice.
#[allow(dead_code)] // variants and fields read by dispatch in src/reader/dispatch.rs
pub(crate) enum ReadKind {
    /// Matches `RawEntity::Simple` whose name equals the entry's `name`.
    Simple {
        read: fn(&mut ReaderContext, u64, &[Attribute], &EntityGraph) -> Result<(), ConvertError>,
    },
    /// Matches `RawEntity::Complex` whose distinct part-set EQUALS one of
    /// `cases` (exact-case matching).
    Complex {
        cases: &'static [&'static [&'static str]],
        read:
            fn(&mut ReaderContext, u64, &[RawEntityPart], &EntityGraph) -> Result<(), ConvertError>,
    },
}

/// Reader-side registry entry. Each handler module submits one via
/// `#[linkme::distributed_slice(ENTITY_HANDLERS)]`. Writer is intentionally
/// absent — the writer uses hand-rolled emit methods (type-erasure trade-off).
#[allow(dead_code)] // Fields are read by dispatch in src/reader/dispatch.rs
pub(crate) struct EntityHandlerEntry {
    pub name: &'static str,
    /// True for 2D parameter-space handlers (those reading pcurve-subtree
    /// geometry). Dispatch exempts them from the pcurve-subtree skip; all
    /// other (3D) handlers leave this false.
    pub is_2d: bool,
    pub kind: ReadKind,
}

/// Compile-time registry of entity handlers contributing to reader
/// dispatch. Populated at link time by `#[distributed_slice]` on each
/// handler module's `static` entry.
#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice]
pub(crate) static ENTITY_HANDLERS: [EntityHandlerEntry] = [..];
