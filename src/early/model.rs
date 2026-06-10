//! L1 entity types + the transient `EarlyModel` store. See [module docs](super).
//!
//! The L1 types mirror the EXPRESS schema shape faithfully (no flatten / no
//! unification) — verified against `early.toml` (blueprint `l1_export`).

use crate::ir::id::FoundedItemId;

/// L1 `SURFACE_STYLE_FILL_AREA(fill_area)`.
///
/// `fill_area` is the raw file id (`#N`) of the referenced `FILL_AREA_STYLE`.
/// It is a typed `EarlyFillAreaStyleId` in the fully-migrated design; during
/// this transition slice `fill_area_style` is not yet migrated, so the ref
/// stays a raw `#N` that [`lower`](super::lower) resolves through the reader's
/// still-present `viz_fas_id_map`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlySurfaceStyleFillArea {
    pub(crate) fill_area: u64,
}

/// L1 `SURFACE_SIDE_STYLE(name, styles)`.
///
/// `styles` are raw file refs; [`lower`](super::lower) disambiguates each by
/// L1 type (a `FillArea` member resolves through the typed
/// [`EarlySurfaceStyleFillAreaId`] cache bucket, a `Rendering` member through
/// the existing `SurfaceStyleRenderingId` bucket).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlySurfaceSideStyle {
    pub(crate) name: String,
    pub(crate) styles: Vec<u64>,
}

/// Typed key for the reader's `id_cache`, registered per migrated
/// `SURFACE_STYLE_FILL_AREA` by its file id. Its `index()` addresses
/// [`EarlyModel::ssfa_lowered`].
///
/// This is the crux of the read-side win: the seven `viz_*_id_map` named
/// fields could not fold into the generic `id_cache` because their value type
/// (`FoundedItemId`) collides in a single `TypeId` bucket. Keying by this
/// distinct L1 type resolves the collision — `surface_side_style` probes
/// `id_cache.get::<EarlySurfaceStyleFillAreaId>(r)` exactly as `StepSelect`
/// would generate, no bespoke map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EarlySurfaceStyleFillAreaId(pub u32);

impl crate::ir::arena::ArenaId for EarlySurfaceStyleFillAreaId {
    fn index(&self) -> usize {
        self.0 as usize
    }
    fn from_index(index: u32) -> Self {
        Self(index)
    }
}

/// Transient L1 store, held on [`ReaderContext`](crate::reader::ReaderContext)
/// for one parse. Only entities migrated to the 2-layer path appear here.
#[derive(Default)]
pub(crate) struct EarlyModel {
    /// Read-side L1→L2 correspondence for migrated `SURFACE_STYLE_FILL_AREA`:
    /// indexed by [`EarlySurfaceStyleFillAreaId`] (which `id_cache` maps a file
    /// id to), each entry is the `FoundedItemId` that [`lower`](super::lower)
    /// produced. `surface_side_style` consults this to resolve a `FillArea`
    /// member by L1 type, replacing `viz_ssfa_id_map`.
    pub(crate) ssfa_lowered: Vec<FoundedItemId>,
}
