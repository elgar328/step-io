//! L1 entity types + the transient `EarlyModel` store. See [module docs](super).
//!
//! The L1 types mirror the EXPRESS schema shape faithfully (no flatten / no
//! unification) — verified against `early.toml` (blueprint `l1_export`).

use std::any::TypeId;
use std::collections::HashMap;

use crate::ir::arena::ArenaId;
use crate::ir::id::FoundedItemId;
use crate::ir::visualization::Projection;

/// Declare an L1 id newtype that is a **pure `id_cache` key** (no backing
/// arena — unlike `define_id!`). Used by migrated founded-item producers to
/// register `file id → typed L1 id`; the `index()` then addresses
/// [`EarlyModel`]'s L1→L2 correspondence bucket.
macro_rules! early_id {
    ($(#[$m:meta])* $id:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub(crate) struct $id(pub u32);

        impl ArenaId for $id {
            fn index(&self) -> usize {
                self.0 as usize
            }
            fn from_index(index: u32) -> Self {
                Self(index)
            }
        }
    };
}

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

early_id! {
    /// Typed `id_cache` key, registered per migrated `SURFACE_STYLE_FILL_AREA`.
    ///
    /// Crux of the read-side win: the seven `viz_*_id_map` named fields could
    /// not fold into the generic `id_cache` because their value type
    /// (`FoundedItemId`) collides in a single `TypeId` bucket. Keying by this
    /// distinct L1 type resolves the collision — `surface_side_style` probes
    /// `id_cache.get::<EarlySurfaceStyleFillAreaId>(r)` exactly as `StepSelect`
    /// would generate, no bespoke map.
    EarlySurfaceStyleFillAreaId
}

/// L1 `VIEW_VOLUME` (faithful 9 attrs). `projection_point` / `view_window` are
/// raw file refs (`#N`) resolved by [`lower`](super::lower) through the existing
/// `id_cache` (`PointId` / `PlanarExtentId`); the rest are owned primitives.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlyViewVolume {
    pub(crate) projection_type: Projection,
    pub(crate) projection_point: u64,
    pub(crate) view_plane_distance: f64,
    pub(crate) front_plane_distance: f64,
    pub(crate) front_plane_clipping: bool,
    pub(crate) back_plane_distance: f64,
    pub(crate) back_plane_clipping: bool,
    pub(crate) view_volume_sides_clipping: bool,
    pub(crate) view_window: u64,
}

early_id! {
    /// Typed `id_cache` key for migrated `VIEW_VOLUME` (replaces
    /// `viz_view_volume_id_map`; consumed by the camera-model handlers).
    EarlyViewVolumeId
}

/// Transient L1 store, held on [`ReaderContext`](crate::reader::ReaderContext)
/// for one parse. Holds the read-side **L1→L2 correspondence** for every
/// founded-item entity migrated to the 2-layer path.
///
/// All migrated founded-item producers lower to a `FoundedItemId`, so one
/// `TypeId`-keyed table suffices (mirroring the reader's `IdMapCache`): each L1
/// id type `K` gets its own bucket, and `K::index()` addresses the `Vec`. This
/// is exactly the collision-free keying that lets `id_cache` distinguish
/// members the bespoke `viz_*_id_map` fields could not.
#[derive(Default)]
pub(crate) struct EarlyModel {
    lowered: HashMap<TypeId, Vec<FoundedItemId>>,
}

impl EarlyModel {
    /// Record that a migrated founded-item entity lowered to `l2`, returning a
    /// fresh typed L1 id `K` addressing it. The caller registers the returned
    /// id in the reader's `id_cache` (keyed by file id) so a consumer can
    /// recover `l2` by L1 type via [`lookup_lowered`](Self::lookup_lowered).
    pub(crate) fn record_lowered<K: ArenaId>(&mut self, l2: FoundedItemId) -> K {
        let bucket = self.lowered.entry(TypeId::of::<K>()).or_default();
        let idx = bucket.len();
        bucket.push(l2);
        K::from_index(u32::try_from(idx).expect("early lowered arena overflow"))
    }

    /// Recover the L2 `FoundedItemId` a typed L1 id `K` was recorded for.
    pub(crate) fn lookup_lowered<K: ArenaId>(&self, id: K) -> FoundedItemId {
        self.lowered[&TypeId::of::<K>()][id.index()]
    }
}
