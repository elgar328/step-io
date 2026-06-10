//! L1 entity types + the transient `EarlyModel` store. See [module docs](super).
//!
//! The L1 types mirror the EXPRESS schema shape faithfully (no flatten / no
//! unification) — verified against `early.toml` (blueprint `l1_export`).

use std::any::TypeId;
use std::collections::HashMap;

use crate::ir::arena::ArenaId;
use crate::ir::id::FoundedItemId;
use crate::ir::visualization::{MarkerType, Projection, SurfaceSide};

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
/// `fill_area` is the raw file id (`#N`) of the referenced `FILL_AREA_STYLE`;
/// [`lower`](super::lower) resolves it through the typed
/// [`EarlyFillAreaStyleId`] bucket (`fill_area_style` is also migrated).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlySurfaceStyleFillArea {
    pub(crate) fill_area: u64,
}

/// L1 `FILL_AREA_STYLE(name, fill_styles)`. `fill_styles` are raw file refs to
/// `FILL_AREA_STYLE_COLOUR` entities, which the L2 inlines; [`lower`](super::lower)
/// resolves each through the reader's `viz_fasc_map` value-cache.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlyFillAreaStyle {
    pub(crate) name: String,
    pub(crate) fill_styles: Vec<u64>,
}

early_id! {
    /// Typed `id_cache` key for migrated `FILL_AREA_STYLE` (replaces
    /// `viz_fas_id_map`; consumed by `surface_style_fill_area`).
    EarlyFillAreaStyleId
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

early_id! {
    /// Typed `id_cache` key for migrated `SURFACE_SIDE_STYLE` (replaces
    /// `viz_sss_id_map`; consumed by `surface_style_usage`). `surface_side_style`
    /// itself already migrated (read slice 1); this only adds the typed bucket so
    /// its consumer can resolve by L1 type.
    EarlySurfaceSideStyleId
}

/// L1 `SURFACE_STYLE_USAGE(side, style)`. `style` is a raw ref to a
/// `SURFACE_SIDE_STYLE` founded item ([`lower`](super::lower) resolves it via
/// the typed [`EarlySurfaceSideStyleId`] bucket).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlySurfaceStyleUsage {
    pub(crate) side: SurfaceSide,
    pub(crate) style: u64,
}

early_id! {
    /// Typed `id_cache` key for migrated `SURFACE_STYLE_USAGE` (replaces
    /// `viz_ssu_id_map`; consumed by `presentation_style_assignment`).
    EarlySurfaceStyleUsageId
}

/// L1 `marker_select`: a `pre_defined_marker` ref (`#N`) or an inline
/// `marker_type` enum.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EarlyMarker {
    Predefined(u64),
    Type(MarkerType),
}

/// L1 `size_select` for a marker: an inline positive-length / descriptive
/// measure, or a `measure_with_unit` ref (`#N`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EarlyMarkerSize {
    PositiveLength(f64),
    Descriptive(String),
    MeasureWithUnit(u64),
}

/// L1 `POINT_STYLE(name, marker, marker_size, marker_colour)`. `marker_colour`
/// is a raw `colour` ref; the marker / size SELECTs carry raw refs in their ref
/// variants. [`lower`](super::lower) resolves all refs via the existing
/// `id_cache`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EarlyPointStyle {
    pub(crate) name: String,
    pub(crate) marker: EarlyMarker,
    pub(crate) marker_size: EarlyMarkerSize,
    pub(crate) marker_colour: u64,
}

early_id! {
    /// Typed `id_cache` key for migrated `POINT_STYLE` (replaces
    /// `viz_point_style_id_map`; consumed by `presentation_style_assignment`).
    EarlyPointStyleId
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
