//! PMI — the product and manufacturing information annotating a shape.
//!
//! Model-wide slices: **dimensions** ([`Scene::all_dimensions`]),
//! **geometric tolerances** ([`Scene::all_tolerances`]), **datums**
//! ([`Scene::all_datums`]), and **features** ([`Scene::all_features`]). A
//! dimension carries a [`kind`](Dimension::kind) and a nominal
//! [`value`](Dimension::value); a tolerance its
//! [`kind`](Tolerance::kind), [`magnitude`](Tolerance::magnitude), and
//! referenced [`datums`](Tolerance::datums). Both point at the shape
//! [`Feature`] they apply to ([`Dimension::features`] /
//! [`Tolerance::feature`]).

// Every public method here is a pure read accessor; `#[must_use]` on each would be
// pure noise, so allow the pedantic lint module-wide.
#![allow(clippy::must_use_candidate)]

use std::collections::HashSet;

use crate::generated::model as m;
use crate::scene::geometry::{Curve, Edge, Face, Point, Solid};
use crate::scene::mesh::Mesh;
use crate::scene::sets::GeometricSet;
use crate::scene::{Ctx, Scene};

/// What a [`Dimension`] measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionKind {
    /// `DIMENSIONAL_SIZE` — the size of one feature.
    Size,
    /// `DIMENSIONAL_LOCATION` — a distance between two features.
    Location,
    /// `ANGULAR_SIZE` — the angle of one feature.
    AngularSize,
    /// `ANGULAR_LOCATION` — an angle between two features.
    AngularLocation,
}

#[derive(Clone, Copy)]
enum DimImpl {
    Size(m::DimensionalSizeId),
    Location(m::DimensionalLocationId),
    AngularSize(m::AngularSizeId),
    AngularLocation(m::AngularLocationId),
}

/// A dimension (`DIMENSIONAL_SIZE` / `_LOCATION` / `ANGULAR_SIZE` /
/// `ANGULAR_LOCATION`) — its kind, name, and nominal value.
#[derive(Clone, Copy)]
pub struct Dimension<'m> {
    cx: Ctx<'m>,
    which: DimImpl,
}

impl Scene<'_> {
    /// Every dimension in the model. Covers the four base dimension types;
    /// the `…_WITH_PATH` / `…_WITH_DATUM_FEATURE` / `DIRECTED_…` subtypes are
    /// not yet surfaced.
    pub fn all_dimensions(&self) -> impl Iterator<Item = Dimension<'_>> + '_ {
        all_dimensions(self.ctx()).into_iter()
    }
}

/// Every dimension in the model (the four base types). Shared by
/// [`Scene::all_dimensions`] and the per-part [`crate::scene::ProductDef`] queries.
fn all_dimensions(cx: Ctx<'_>) -> Vec<Dimension<'_>> {
    let mut out = Vec::new();
    for i in cx.model.dimensional_size_arena.indices() {
        out.push(Dimension {
            cx,
            which: DimImpl::Size(m::DimensionalSizeId(i)),
        });
    }
    for i in cx.model.dimensional_location_arena.indices() {
        out.push(Dimension {
            cx,
            which: DimImpl::Location(m::DimensionalLocationId(i)),
        });
    }
    for i in cx.model.angular_size_arena.indices() {
        out.push(Dimension {
            cx,
            which: DimImpl::AngularSize(m::AngularSizeId(i)),
        });
    }
    for i in cx.model.angular_location_arena.indices() {
        out.push(Dimension {
            cx,
            which: DimImpl::AngularLocation(m::AngularLocationId(i)),
        });
    }
    out
}

impl<'m> Dimension<'m> {
    /// What this dimension measures.
    pub fn kind(&self) -> DimensionKind {
        match self.which {
            DimImpl::Size(_) => DimensionKind::Size,
            DimImpl::Location(_) => DimensionKind::Location,
            DimImpl::AngularSize(_) => DimensionKind::AngularSize,
            DimImpl::AngularLocation(_) => DimensionKind::AngularLocation,
        }
    }

    /// The dimension's name.
    pub fn name(&self) -> &'m str {
        match self.which {
            DimImpl::Size(i) => self
                .cx
                .model
                .text(self.cx.model.dimensional_size_arena.get(i.0).name),
            DimImpl::Location(i) => self
                .cx
                .model
                .text(self.cx.model.dimensional_location_arena.get(i.0).name),
            DimImpl::AngularSize(i) => self
                .cx
                .model
                .text(self.cx.model.angular_size_arena.get(i.0).name),
            DimImpl::AngularLocation(i) => self
                .cx
                .model
                .text(self.cx.model.angular_location_arena.get(i.0).name),
        }
    }

    /// This dimension's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            DimImpl::Size(i) => m::EntityKey::DimensionalSize(i),
            DimImpl::Location(i) => m::EntityKey::DimensionalLocation(i),
            DimImpl::AngularSize(i) => m::EntityKey::AngularSize(i),
            DimImpl::AngularLocation(i) => m::EntityKey::AngularLocation(i),
        }
    }

    /// The nominal value (reverse: a `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION`
    /// referencing this dimension → its `SHAPE_DIMENSION_REPRESENTATION`'s
    /// `MEASURE_REPRESENTATION_ITEM`, plain or a complex instance with that part,
    /// such as `(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
    /// MEASURE_WITH_UNIT(…) …)`). A representation may list the limits of the
    /// dimension beside its nominal value, named `'upper limit'` and
    /// `'lower limit'` as the standard names a value range's limits; the value
    /// is the first measure item with neither name.
    ///
    /// The number is raw, in the unit its measure names (`unit_component`,
    /// not read here), normally the model's length or angle unit as
    /// [`kind`](Dimension::kind) says (`Size`/`Location` are lengths,
    /// `Angular*` are angles) — interpret it via [`Scene::units`], exactly as
    /// point coordinates are read. A representation the scene cannot resolve
    /// (a complex one) is reported via [`Scene::warnings`]; a dimension with no
    /// value representation, or whose representation holds no such item (only
    /// limits, or only a `VALUE_RANGE`, say), simply yields `None`.
    pub fn value(&self) -> Option<f64> {
        let this = self.about();
        let cx = this.cx;
        let rg = cx.ref_graph();
        for r in rg.referrers(this.key()) {
            let m::EntityKey::DimensionalCharacteristicRepresentation(dcr_id) = r else {
                continue;
            };
            let dcr = cx
                .model
                .dimensional_characteristic_representation_arena
                .get(dcr_id.0);
            let sdr_id = match &dcr.representation {
                m::ShapeDimensionRepresentationRef::ShapeDimensionRepresentation(id) => id,
                m::ShapeDimensionRepresentationRef::Complex(_) => {
                    cx.warn("dimension value representation is a complex instance".to_owned());
                    continue;
                }
            };
            let sdr = cx.model.shape_dimension_representation_arena.get(sdr_id.0);
            for item in &sdr.items {
                match item {
                    m::RepresentationItemRef::MeasureRepresentationItem(mid) => {
                        let mri = cx.model.measure_representation_item_arena.get(mid.0);
                        if is_limit_name(cx.model.text(mri.name)) {
                            continue;
                        }
                        return Some(mri.value_component.value.as_f64());
                    }
                    // A complex item is a value only with a
                    // `MEASURE_REPRESENTATION_ITEM` part.
                    m::RepresentationItemRef::Complex(cuid) => {
                        let parts = &cx.model.complex_entity_arena.get(cuid.0).parts;
                        if !parts
                            .iter()
                            .any(|p| matches!(p, m::PartialEntity::MeasureRepresentationItem))
                        {
                            continue;
                        }
                        let name = parts.iter().find_map(|p| match p {
                            m::PartialEntity::RepresentationItem { name } => {
                                Some(cx.model.text(*name))
                            }
                            _ => None,
                        });
                        if name.is_some_and(is_limit_name) {
                            continue;
                        }
                        if let Some((value, _)) =
                            crate::scene::units::complex_measure(cx.model, *cuid)
                        {
                            return Some(value.value.as_f64());
                        }
                        cx.warn(
                            "dimension value is a complex instance without a MEASURE_WITH_UNIT part"
                                .to_owned(),
                        );
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// Whether a measure item's name marks it as a limit rather than the nominal
/// value.
fn is_limit_name(name: &str) -> bool {
    name == "upper limit" || name == "lower limit"
}

/// The GD&T characteristic a [`Tolerance`] specifies (its control-frame symbol).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToleranceKind {
    /// `POSITION_TOLERANCE`.
    Position,
    /// `FLATNESS_TOLERANCE`.
    Flatness,
    /// `STRAIGHTNESS_TOLERANCE`.
    Straightness,
    /// `ROUNDNESS_TOLERANCE`.
    Roundness,
    /// `CYLINDRICITY_TOLERANCE`.
    Cylindricity,
    /// `LINE_PROFILE_TOLERANCE`.
    LineProfile,
    /// `SURFACE_PROFILE_TOLERANCE`.
    SurfaceProfile,
    /// `ANGULARITY_TOLERANCE`.
    Angularity,
    /// `PARALLELISM_TOLERANCE`.
    Parallelism,
    /// `PERPENDICULARITY_TOLERANCE`.
    Perpendicularity,
    /// `CONCENTRICITY_TOLERANCE`.
    Concentricity,
    /// `COAXIALITY_TOLERANCE`.
    Coaxiality,
    /// `SYMMETRY_TOLERANCE`.
    Symmetry,
    /// `CIRCULAR_RUNOUT_TOLERANCE`.
    CircularRunout,
    /// `TOTAL_RUNOUT_TOLERANCE`.
    TotalRunout,
    /// A geometric tolerance whose characteristic subtype was not recognized
    /// (reported via [`Scene::warnings`]).
    Other,
}

/// A geometric tolerance (`GEOMETRIC_TOLERANCE` and its subtypes) — its
/// characteristic [`kind`](Tolerance::kind), name, and nominal
/// [`magnitude`](Tolerance::magnitude).
#[derive(Clone, Copy)]
pub struct Tolerance<'m> {
    cx: Ctx<'m>,
    which: TolImpl,
}

// The 15 leaf tolerance subtypes each have a dedicated arena and an `EntityKey`
// variant, and all carry the `name` + `magnitude` fields inherited from
// `GEOMETRIC_TOLERANCE`. This table generates the `TolImpl` cursor enum plus the
// per-variant `kind` / `key` / field access, so only the complex-instance path
// below is hand-written.
macro_rules! leaf_tolerances {
    ($( $ent:ident, $id:ident, $arena:ident, $kind:ident );+ $(;)?) => {
        #[derive(Clone, Copy)]
        enum TolImpl {
            $( $ent(m::$id), )+
            Complex(m::ComplexEntityId),
        }

        fn push_dedicated_tolerances<'m>(cx: Ctx<'m>, out: &mut Vec<Tolerance<'m>>) {
            $(
                for i in cx.model.$arena.indices() {
                    out.push(Tolerance { cx, which: TolImpl::$ent(m::$id(i)) });
                }
            )+
        }

        impl<'m> Tolerance<'m> {
            /// The GD&T characteristic this tolerance specifies.
            pub fn kind(&self) -> ToleranceKind {
                let this = self.about();
                match this.which {
                    $( TolImpl::$ent(_) => ToleranceKind::$kind, )+
                    TolImpl::Complex(cuid) => this.complex_kind(cuid),
                }
            }

            /// This tolerance's global identity (a `Copy` key for maps / dedup).
            pub fn key(&self) -> m::EntityKey {
                match self.which {
                    $( TolImpl::$ent(i) => m::EntityKey::$ent(i), )+
                    TolImpl::Complex(cuid) => m::EntityKey::ComplexEntity(cuid),
                }
            }

            /// `(name, magnitude)` for a dedicated-arena leaf tolerance.
            fn dedicated_fields(&self) -> Option<(&'m str, Option<&'m m::MeasureWithUnitRef>)> {
                match self.which {
                    $( TolImpl::$ent(i) => {
                        let e = self.cx.model.$arena.get(i.0);
                        Some((self.cx.model.text(e.name), e.magnitude.as_ref()))
                    } )+
                    TolImpl::Complex(_) => None,
                }
            }

            /// The toleranced feature target for a dedicated-arena leaf tolerance.
            fn dedicated_target(&self) -> Option<&'m m::GeometricToleranceTargetRef> {
                match self.which {
                    $( TolImpl::$ent(i) => {
                        Some(&self.cx.model.$arena.get(i.0).toleranced_shape_aspect)
                    } )+
                    TolImpl::Complex(_) => None,
                }
            }
        }
    };
}

leaf_tolerances! {
    PositionTolerance,        PositionToleranceId,        position_tolerance_arena,         Position;
    FlatnessTolerance,        FlatnessToleranceId,        flatness_tolerance_arena,         Flatness;
    StraightnessTolerance,    StraightnessToleranceId,    straightness_tolerance_arena,     Straightness;
    RoundnessTolerance,       RoundnessToleranceId,       roundness_tolerance_arena,        Roundness;
    CylindricityTolerance,    CylindricityToleranceId,    cylindricity_tolerance_arena,     Cylindricity;
    LineProfileTolerance,     LineProfileToleranceId,     line_profile_tolerance_arena,     LineProfile;
    SurfaceProfileTolerance,  SurfaceProfileToleranceId,  surface_profile_tolerance_arena,  SurfaceProfile;
    AngularityTolerance,      AngularityToleranceId,      angularity_tolerance_arena,       Angularity;
    ParallelismTolerance,     ParallelismToleranceId,     parallelism_tolerance_arena,      Parallelism;
    PerpendicularityTolerance,PerpendicularityToleranceId,perpendicularity_tolerance_arena, Perpendicularity;
    ConcentricityTolerance,   ConcentricityToleranceId,   concentricity_tolerance_arena,    Concentricity;
    CoaxialityTolerance,      CoaxialityToleranceId,      coaxiality_tolerance_arena,       Coaxiality;
    SymmetryTolerance,        SymmetryToleranceId,        symmetry_tolerance_arena,         Symmetry;
    CircularRunoutTolerance,  CircularRunoutToleranceId,  circular_runout_tolerance_arena,  CircularRunout;
    TotalRunoutTolerance,     TotalRunoutToleranceId,     total_runout_tolerance_arena,     TotalRunout;
}

impl Scene<'_> {
    /// Every geometric tolerance in the model. Covers both standalone leaf
    /// entities (in their dedicated arenas) and complex multi-supertype
    /// instances (any `ComplexEntity` carrying a `GEOMETRIC_TOLERANCE` part).
    pub fn all_tolerances(&self) -> impl Iterator<Item = Tolerance<'_>> + '_ {
        all_tolerances(self.ctx()).into_iter()
    }
}

/// Every geometric tolerance in the model — standalone leaf entities plus complex
/// multi-supertype instances. Shared by [`Scene::all_tolerances`] and per-part queries.
fn all_tolerances(cx: Ctx<'_>) -> Vec<Tolerance<'_>> {
    let mut out: Vec<Tolerance<'_>> = Vec::new();
    push_dedicated_tolerances(cx, &mut out);
    for i in cx.model.complex_entity_arena.indices() {
        let cu = cx.model.complex_entity_arena.get(i);
        if cu
            .parts
            .iter()
            .any(|p| matches!(p, m::PartialEntity::GeometricTolerance { .. }))
        {
            out.push(Tolerance {
                cx,
                which: TolImpl::Complex(m::ComplexEntityId(i)),
            });
        }
    }
    out
}

impl<'m> Tolerance<'m> {
    /// The tolerance's name.
    pub fn name(&self) -> &'m str {
        self.dedicated_fields()
            .or_else(|| self.complex_geometric_tolerance())
            .map_or("", |(name, _)| name)
    }

    /// The nominal magnitude (the tolerance-zone size).
    ///
    /// The number is raw, in the unit its measure names (`unit_component`,
    /// not read here), normally the model's length unit — interpret it via
    /// [`Scene::units`], exactly as point coordinates and dimension values are
    /// read. A complex measure such as `(LENGTH_MEASURE_WITH_UNIT()
    /// MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(…) …)` is read from its
    /// `MEASURE_WITH_UNIT` part. AP203 and AP214 allow a plain
    /// `MEASURE_WITH_UNIT` here; its value is taken when it is a length
    /// measure. `None` when the tolerance carries no magnitude, or its
    /// magnitude is not a length or a complex instance without a
    /// `MEASURE_WITH_UNIT` part (reported via [`Scene::warnings`]).
    pub fn magnitude(&self) -> Option<f64> {
        let this = self.about();
        let (_, mag) = this
            .dedicated_fields()
            .or_else(|| this.complex_geometric_tolerance())?;
        mag.and_then(|r| tolerance_magnitude_f64(this.cx, r))
    }

    /// The `GEOMETRIC_TOLERANCE` supertype part of a complex tolerance, as
    /// `(name, magnitude)`; `None` for a dedicated-arena tolerance.
    fn complex_geometric_tolerance(&self) -> Option<(&'m str, Option<&'m m::MeasureWithUnitRef>)> {
        let TolImpl::Complex(cuid) = self.which else {
            return None;
        };
        let cu = self.cx.model.complex_entity_arena.get(cuid.0);
        cu.parts.iter().find_map(|p| match p {
            m::PartialEntity::GeometricTolerance {
                name, magnitude, ..
            } => Some((self.cx.model.text(*name), magnitude.as_ref())),
            _ => None,
        })
    }

    /// The characteristic of a complex tolerance, read from its leaf-subtype
    /// part. Only the leaf markers that occur in real files exist as
    /// `PartialEntity` variants; a complex tolerance without a recognized one is
    /// surfaced as `Other`.
    fn complex_kind(&self, cuid: m::ComplexEntityId) -> ToleranceKind {
        let cu = self.cx.model.complex_entity_arena.get(cuid.0);
        for p in &cu.parts {
            let kind = match p {
                m::PartialEntity::PositionTolerance => ToleranceKind::Position,
                m::PartialEntity::FlatnessTolerance => ToleranceKind::Flatness,
                m::PartialEntity::StraightnessTolerance => ToleranceKind::Straightness,
                m::PartialEntity::RoundnessTolerance => ToleranceKind::Roundness,
                m::PartialEntity::CylindricityTolerance => ToleranceKind::Cylindricity,
                m::PartialEntity::LineProfileTolerance => ToleranceKind::LineProfile,
                m::PartialEntity::SurfaceProfileTolerance => ToleranceKind::SurfaceProfile,
                m::PartialEntity::ParallelismTolerance => ToleranceKind::Parallelism,
                m::PartialEntity::PerpendicularityTolerance => ToleranceKind::Perpendicularity,
                m::PartialEntity::CircularRunoutTolerance => ToleranceKind::CircularRunout,
                _ => continue,
            };
            return kind;
        }
        self.cx
            .warn("geometric tolerance has no recognized characteristic subtype".to_owned());
        ToleranceKind::Other
    }
}

/// Resolve a tolerance magnitude to its scalar length. AP242 types it as a
/// `LENGTH_MEASURE_WITH_UNIT`; AP203 and AP214 allow any `MEASURE_WITH_UNIT`,
/// whose value counts only when it is a length measure. A complex measure is a
/// length when it has a length-measure part, else by its value's type. A value
/// of another kind is reported and yields `None`.
fn tolerance_magnitude_f64(cx: Ctx<'_>, r: &m::MeasureWithUnitRef) -> Option<f64> {
    let model = cx.model;
    let length = |v: &m::MeasureValue| {
        // Every edition's `measure_value` has these three length members.
        let is_length = v.type_name.is_some_and(|t| {
            let t = model.text(t);
            [
                "LENGTH_MEASURE",
                "POSITIVE_LENGTH_MEASURE",
                "NON_NEGATIVE_LENGTH_MEASURE",
            ]
            .iter()
            .any(|n| t.eq_ignore_ascii_case(n))
        });
        if is_length {
            Some(v.value.as_f64())
        } else {
            cx.warn("tolerance magnitude is not a length measure".to_owned());
            None
        }
    };
    match r {
        m::MeasureWithUnitRef::LengthMeasureWithUnit(id) => Some(
            model
                .length_measure_with_unit_arena
                .get(id.0)
                .value_component
                .value
                .as_f64(),
        ),
        m::MeasureWithUnitRef::PositiveLengthMeasureWithUnit(id) => Some(
            model
                .positive_length_measure_with_unit_arena
                .get(id.0)
                .value_component
                .value
                .as_f64(),
        ),
        m::MeasureWithUnitRef::MeasureWithUnit(id) => {
            length(&model.measure_with_unit_arena.get(id.0).value_component)
        }
        m::MeasureWithUnitRef::MeasureRepresentationItem(id) => length(
            &model
                .measure_representation_item_arena
                .get(id.0)
                .value_component,
        ),
        m::MeasureWithUnitRef::UncertaintyMeasureWithUnit(id) => length(
            &model
                .uncertainty_measure_with_unit_arena
                .get(id.0)
                .value_component,
        ),
        m::MeasureWithUnitRef::Complex(cuid) => {
            let Some((value, _)) = crate::scene::units::complex_measure(model, *cuid) else {
                cx.warn(
                    "tolerance magnitude is a complex instance without a MEASURE_WITH_UNIT part"
                        .to_owned(),
                );
                return None;
            };
            if crate::scene::units::complex_is_length_measure(model, *cuid) {
                Some(value.value.as_f64())
            } else {
                length(value)
            }
        }
        m::MeasureWithUnitRef::MassMeasureWithUnit(_)
        | m::MeasureWithUnitRef::PlaneAngleMeasureWithUnit(_)
        | m::MeasureWithUnitRef::PositivePlaneAngleMeasureWithUnit(_)
        | m::MeasureWithUnitRef::RatioMeasureWithUnit(_) => {
            cx.warn("tolerance magnitude is not a length measure".to_owned());
            None
        }
    }
}

/// A datum (`DATUM` / `COMMON_DATUM`) — the geometric reference a tolerance is
/// measured against. Its identifying letter (A, B, C, …) is [`letter`](Datum::letter).
#[derive(Clone, Copy)]
pub struct Datum<'m> {
    cx: Ctx<'m>,
    which: DatumImpl,
}

#[derive(Clone, Copy)]
enum DatumImpl {
    Datum(m::DatumId),
    CommonDatum(m::CommonDatumId),
}

impl Scene<'_> {
    /// Every datum in the model — `DATUM`s and `COMMON_DATUM`s (the reference
    /// features that carry an identifying letter). The physical `DATUM_FEATURE`
    /// a datum is established from is a different entity, reached through
    /// [`Datum::datum_feature`].
    pub fn all_datums(&self) -> impl Iterator<Item = Datum<'_>> + '_ {
        all_datums(self.ctx()).into_iter()
    }
}

/// Every datum (`DATUM` / `COMMON_DATUM`) in the model. Shared by
/// [`Scene::all_datums`] and per-part queries.
fn all_datums(cx: Ctx<'_>) -> Vec<Datum<'_>> {
    let mut out: Vec<Datum<'_>> = Vec::new();
    for i in cx.model.datum_arena.indices() {
        out.push(Datum {
            cx,
            which: DatumImpl::Datum(m::DatumId(i)),
        });
    }
    for i in cx.model.common_datum_arena.indices() {
        out.push(Datum {
            cx,
            which: DatumImpl::CommonDatum(m::CommonDatumId(i)),
        });
    }
    out
}

impl<'m> Datum<'m> {
    /// The datum's identifying label — STEP `identification`. Usually a single
    /// letter (A, B, C, …); a common datum may carry a compound label ("A-B").
    pub fn letter(&self) -> &'m str {
        match self.which {
            DatumImpl::Datum(i) => self
                .cx
                .model
                .text(self.cx.model.datum_arena.get(i.0).identification),
            DatumImpl::CommonDatum(i) => self
                .cx
                .model
                .text(self.cx.model.common_datum_arena.get(i.0).identification),
        }
    }

    /// The datum's name.
    pub fn name(&self) -> &'m str {
        match self.which {
            DatumImpl::Datum(i) => self.cx.model.text(self.cx.model.datum_arena.get(i.0).name),
            DatumImpl::CommonDatum(i) => self
                .cx
                .model
                .text(self.cx.model.common_datum_arena.get(i.0).name),
        }
    }

    /// This datum's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            DatumImpl::Datum(i) => m::EntityKey::Datum(i),
            DatumImpl::CommonDatum(i) => m::EntityKey::CommonDatum(i),
        }
    }

    /// The physical feature this datum is established from — the `DATUM_FEATURE`
    /// (or `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`, or a complex instance with a
    /// `DATUM_FEATURE` part, as a composite datum feature is written) linked to
    /// it by a `SHAPE_ASPECT_RELATIONSHIP`. Returned as a [`Feature`],
    /// whose faces/edges are reachable via [`Feature::geometry`]. `None` when the
    /// datum is defined only by datum targets, or carries no such link.
    pub fn datum_feature(&self) -> Option<Feature<'m>> {
        let this = self.about();
        let cx = this.cx;
        let rg = cx.ref_graph();
        for r in rg.referrers(this.key()) {
            let m::EntityKey::ShapeAspectRelationship(sid) = r else {
                continue;
            };
            let sar = cx.model.shape_aspect_relationship_arena.get(sid.0);
            // The datum is one end of the relationship; the datum feature is the
            // other (checking both ends naturally selects it).
            for aspect in [&sar.relating_shape_aspect, &sar.related_shape_aspect] {
                let which = match aspect {
                    m::ShapeAspectRef::DatumFeature(fid) => FeatureImpl::DatumFeature(*fid),
                    m::ShapeAspectRef::DimensionalSizeWithDatumFeature(fid) => {
                        FeatureImpl::DimensionalSizeWithDatumFeature(*fid)
                    }
                    m::ShapeAspectRef::Complex(cid) => {
                        let parts = &cx.model.complex_entity_arena.get(cid.0).parts;
                        if !parts
                            .iter()
                            .any(|p| matches!(p, m::PartialEntity::DatumFeature))
                        {
                            continue;
                        }
                        let Some(which) = complex_feature(cx.model, *cid) else {
                            cx.warn(
                                "a datum's shape aspect relationship ends in a complex datum \
                                 feature without a SHAPE_ASPECT part; not read"
                                    .to_owned(),
                            );
                            continue;
                        };
                        which
                    }
                    _ => continue,
                };
                return Some(Feature { cx: self.cx, which });
            }
        }
        None
    }
}

impl<'m> Tolerance<'m> {
    /// The datum reference frame this tolerance is controlled against, in
    /// precedence order (primary → secondary → tertiary).
    ///
    /// Empty for a form tolerance (flatness, straightness, …) that references no
    /// datum. A common-datum list in a datum system compartment is flattened
    /// to its constituent datums; a `COMMON_DATUM` comes back as one [`Datum`]
    /// (its letter may read `A-B`). A reference the scene cannot resolve (a
    /// complex instance) is reported via [`Scene::warnings`].
    pub fn datums(&self) -> impl Iterator<Item = Datum<'m>> + 'm {
        let this = self.about();
        // `precedence` is authored per DATUM_REFERENCE but a DATUM_SYSTEM orders
        // its compartments positionally; collect a sort key from whichever form
        // applies, then order by it so the frame reads primary-first regardless
        // of file order.
        let mut keyed: Vec<(i64, Datum<'m>)> = Vec::new();
        for r in this.datum_system() {
            this.collect_system_ref(r, &mut keyed);
        }
        keyed.sort_by_key(|(k, _)| *k);
        keyed.into_iter().map(|(_, d)| d)
    }

    /// The tolerance's `datum_system` list: on the dedicated arena for the eight
    /// leaf subtypes that carry one, on the `GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE`
    /// part for a complex tolerance, and empty for the datum-free subtypes.
    fn datum_system(&self) -> &'m [m::DatumSystemOrReferenceRef] {
        let model = self.cx.model;
        match self.which {
            TolImpl::AngularityTolerance(i) => {
                &model.angularity_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::CircularRunoutTolerance(i) => {
                &model.circular_runout_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::CoaxialityTolerance(i) => {
                &model.coaxiality_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::ConcentricityTolerance(i) => {
                &model.concentricity_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::ParallelismTolerance(i) => {
                &model.parallelism_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::PerpendicularityTolerance(i) => {
                &model.perpendicularity_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::SymmetryTolerance(i) => &model.symmetry_tolerance_arena.get(i.0).datum_system,
            TolImpl::TotalRunoutTolerance(i) => {
                &model.total_runout_tolerance_arena.get(i.0).datum_system
            }
            TolImpl::Complex(cuid) => model
                .complex_entity_arena
                .get(cuid.0)
                .parts
                .iter()
                .find_map(|p| match p {
                    m::PartialEntity::GeometricToleranceWithDatumReference { datum_system } => {
                        Some(datum_system.as_slice())
                    }
                    _ => None,
                })
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    /// One `datum_system` element → its datum(s). A `DATUM_REFERENCE` carries an
    /// explicit precedence; a `DATUM_SYSTEM`'s compartments are ordered, so their
    /// index is the precedence.
    fn collect_system_ref(
        &self,
        r: &m::DatumSystemOrReferenceRef,
        out: &mut Vec<(i64, Datum<'m>)>,
    ) {
        match r {
            m::DatumSystemOrReferenceRef::DatumReference(id) => {
                let dr = self.cx.model.datum_reference_arena.get(id.0);
                self.collect_datum_ref(dr.precedence, &dr.referenced_datum, out);
            }
            m::DatumSystemOrReferenceRef::DatumSystem(id) => {
                let ds = self.cx.model.datum_system_arena.get(id.0);
                for (idx, c) in ds.constituents.iter().enumerate() {
                    let m::DatumReferenceCompartmentRef::DatumReferenceCompartment(cid) = c else {
                        self.cx
                            .warn("datum system compartment is a complex instance".to_owned());
                        continue;
                    };
                    let comp = self.cx.model.datum_reference_compartment_arena.get(cid.0);
                    let key = i64::try_from(idx).unwrap_or(i64::MAX);
                    self.collect_base(key, &comp.base, out, 0);
                }
            }
            m::DatumSystemOrReferenceRef::Complex(_) => {
                self.cx
                    .warn("tolerance datum reference is a complex instance".to_owned());
            }
        }
    }

    /// A `DatumRef` (the target of a `DATUM_REFERENCE`) → a datum handle.
    fn collect_datum_ref(&self, key: i64, r: &m::DatumRef, out: &mut Vec<(i64, Datum<'m>)>) {
        match r {
            m::DatumRef::Datum(id) => out.push((key, self.datum(DatumImpl::Datum(*id)))),
            m::DatumRef::CommonDatum(id) => {
                out.push((key, self.datum(DatumImpl::CommonDatum(*id))));
            }
            m::DatumRef::Complex(_) => {
                self.cx
                    .warn("tolerance references a complex datum instance".to_owned());
            }
        }
    }

    /// A compartment `base` → its datum(s). Resolves the `DATUM_REFERENCE_ELEMENT`
    /// and common-datum-list indirections down to plain datums.
    fn collect_base(
        &self,
        key: i64,
        r: &m::DatumOrCommonDatumRef,
        out: &mut Vec<(i64, Datum<'m>)>,
        depth: u32,
    ) {
        if depth > 8 {
            self.cx
                .warn("datum reference nesting is too deep to resolve".to_owned());
            return;
        }
        match r {
            m::DatumOrCommonDatumRef::Datum(id) => {
                out.push((key, self.datum(DatumImpl::Datum(*id))));
            }
            m::DatumOrCommonDatumRef::CommonDatum(id) => {
                out.push((key, self.datum(DatumImpl::CommonDatum(*id))));
            }
            m::DatumOrCommonDatumRef::DatumReferenceElement(id) => {
                let e = self.cx.model.datum_reference_element_arena.get(id.0);
                self.collect_base(key, &e.base, out, depth + 1);
            }
            m::DatumOrCommonDatumRef::CommonDatumList(refs) => {
                for er in refs {
                    let m::DatumReferenceElementRef::DatumReferenceElement(eid) = er else {
                        self.cx
                            .warn("datum reference element is a complex instance".to_owned());
                        continue;
                    };
                    let e = self.cx.model.datum_reference_element_arena.get(eid.0);
                    self.collect_base(key, &e.base, out, depth + 1);
                }
            }
            m::DatumOrCommonDatumRef::Complex(_) => {
                self.cx
                    .warn("tolerance references a complex datum instance".to_owned());
            }
        }
    }

    fn datum(&self, which: DatumImpl) -> Datum<'m> {
        Datum { cx: self.cx, which }
    }
}

/// A shape feature (`SHAPE_ASPECT` and its subtypes) that a dimension or
/// tolerance applies to — its name, description, and identity.
///
/// Every entity here is a `SHAPE_ASPECT` subtype in the schema (that is why the
/// same set forms both `applies_to` and `toleranced_shape_aspect` targets), so a
/// `Feature` can be a plain aspect, a datum feature, a composite/derived aspect,
/// or occasionally a datum / tolerance-zone construct — or a complex instance
/// with a `SHAPE_ASPECT` part that combines several of them (a composite datum
/// feature written as `(COMPOSITE_SHAPE_ASPECT DATUM_FEATURE SHAPE_ASPECT)`).
/// [`key`](Feature::key) reveals the exact kind when it matters.
#[derive(Clone, Copy)]
pub struct Feature<'m> {
    cx: Ctx<'m>,
    which: FeatureImpl,
}

// The 20 SHAPE_ASPECT-family entities all share `name` / `description` / a
// dedicated arena / an `EntityKey` variant. This table generates the cursor enum,
// the accessors, and the two reference-union converters — all covering the full
// family, since a dimension / tolerance target may point at any of them — plus
// the arms for a complex instance with a `SHAPE_ASPECT` part, which reads its
// `name` / `description` / `of_shape` from that part.
//
// The family splits in two: `regions` are the SHAPE_ASPECT subtypes that denote a
// portion of the part's shape (a feature) and are what `Scene::all_features` lists;
// `aspects` are also SHAPE_ASPECTs in the schema but are datum callouts, datum
// reference structures, tolerance zones, views, or dimensions — reached via
// their own queries or the raw model. Checked on real files: no dimension / tolerance
// ever targets an `aspects` type, so this split drops no real feature.
macro_rules! feature_family {
    (
        regions: { $( $rent:ident, $rid:ident, $rarena:ident );+ $(;)? }
        aspects: { $( $aent:ident, $aid:ident, $aarena:ident );+ $(;)? }
    ) => {
        #[derive(Clone, Copy)]
        enum FeatureImpl {
            $( $rent(m::$rid), )+
            $( $aent(m::$aid), )+
            Complex(m::ComplexEntityId),
        }

        /// The concrete kind of a [`Feature`] — which `SHAPE_ASPECT`-family
        /// entity backs it, without matching on [`Feature::key`]'s `EntityKey`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum FeatureKind {
            $( $rent, )+
            $( $aent, )+
            /// A complex instance with a `SHAPE_ASPECT` part, combining several
            /// shape-aspect subtypes at once (a composite datum feature, for
            /// one); its parts are on the model under [`Feature::key`]'s
            /// `ComplexEntity`.
            Complex,
        }

        impl<'m> Feature<'m> {
            /// The concrete kind backing this feature.
            pub fn kind(&self) -> FeatureKind {
                match self.which {
                    $( FeatureImpl::$rent(_) => FeatureKind::$rent, )+
                    $( FeatureImpl::$aent(_) => FeatureKind::$aent, )+
                    FeatureImpl::Complex(_) => FeatureKind::Complex,
                }
            }
            /// The feature's name.
            pub fn name(&self) -> &'m str {
                match self.which {
                    $( FeatureImpl::$rent(i) => self.cx.model.text(self.cx.model.$rarena.get(i.0).name), )+
                    $( FeatureImpl::$aent(i) => self.cx.model.text(self.cx.model.$aarena.get(i.0).name), )+
                    FeatureImpl::Complex(i) => complex_shape_aspect(self.cx.model, i)
                        .map_or("", |(name, _, _)| self.cx.model.text(name)),
                }
            }

            /// The feature's description, if any.
            pub fn description(&self) -> Option<&'m str> {
                match self.which {
                    $( FeatureImpl::$rent(i) => {
                        self.cx.model.$rarena.get(i.0).description.map(|t| self.cx.model.text(t))
                    } )+
                    $( FeatureImpl::$aent(i) => {
                        self.cx.model.$aarena.get(i.0).description.map(|t| self.cx.model.text(t))
                    } )+
                    FeatureImpl::Complex(i) => complex_shape_aspect(self.cx.model, i)
                        .and_then(|(_, description, _)| description)
                        .map(|t| self.cx.model.text(t)),
                }
            }

            /// This feature's global identity (a `Copy` key for maps / dedup).
            pub fn key(&self) -> m::EntityKey {
                match self.which {
                    $( FeatureImpl::$rent(i) => m::EntityKey::$rent(i), )+
                    $( FeatureImpl::$aent(i) => m::EntityKey::$aent(i), )+
                    FeatureImpl::Complex(i) => m::EntityKey::ComplexEntity(i),
                }
            }

            /// The `of_shape` product-definition-shape this feature belongs to.
            fn of_shape(&self) -> Option<&'m m::ProductDefinitionShapeRef> {
                match self.which {
                    $( FeatureImpl::$rent(i) => Some(&self.cx.model.$rarena.get(i.0).of_shape), )+
                    $( FeatureImpl::$aent(i) => Some(&self.cx.model.$aarena.get(i.0).of_shape), )+
                    FeatureImpl::Complex(i) => {
                        complex_shape_aspect(self.cx.model, i).map(|(_, _, of_shape)| of_shape)
                    }
                }
            }
        }

        /// A `ShapeAspectRef` (a dimension's target) → a feature cursor. Every
        /// non-`Complex` variant is a shape-aspect-family member; a complex
        /// instance resolves when it has a `SHAPE_ASPECT` part.
        fn feature_impl_from_shape_aspect_ref(
            model: &m::StepModel,
            r: &m::ShapeAspectRef,
        ) -> Option<FeatureImpl> {
            match r {
                $( m::ShapeAspectRef::$rent(i) => Some(FeatureImpl::$rent(*i)), )+
                $( m::ShapeAspectRef::$aent(i) => Some(FeatureImpl::$aent(*i)), )+
                m::ShapeAspectRef::Complex(i) => complex_feature(model, *i),
            }
        }

        /// A `GeometricToleranceTargetRef` (a tolerance's target) → a feature
        /// cursor. The shape-aspect-family arms resolve, and so does a complex
        /// instance with a `SHAPE_ASPECT` part; the dimension / product arms
        /// are not features and yield `None`.
        fn feature_impl_from_tolerance_target(
            model: &m::StepModel,
            r: &m::GeometricToleranceTargetRef,
        ) -> Option<FeatureImpl> {
            match r {
                $( m::GeometricToleranceTargetRef::$rent(i) => Some(FeatureImpl::$rent(*i)), )+
                $( m::GeometricToleranceTargetRef::$aent(i) => Some(FeatureImpl::$aent(*i)), )+
                m::GeometricToleranceTargetRef::Complex(i) => complex_feature(model, *i),
                _ => None,
            }
        }

        /// Every shape-region feature (the `regions` group) in the model — what
        /// [`Scene::all_features`] enumerates — then every complex instance
        /// with a `SHAPE_ASPECT` part and no part from the `aspects` group.
        fn push_all_features<'m>(cx: Ctx<'m>, out: &mut Vec<Feature<'m>>) {
            $(
                for i in cx.model.$rarena.indices() {
                    out.push(Feature { cx, which: FeatureImpl::$rent(m::$rid(i)) });
                }
            )+
            for i in cx.model.complex_entity_arena.indices() {
                let id = m::ComplexEntityId(i);
                let parts = &cx.model.complex_entity_arena.get(i).parts;
                if complex_shape_aspect(cx.model, id).is_some()
                    && !parts
                        .iter()
                        .any(|p| matches!(p, $( m::PartialEntity::$aent { .. } )|+))
                {
                    out.push(Feature { cx, which: FeatureImpl::Complex(id) });
                }
            }
        }
    };
}

feature_family! {
    // regions — SHAPE_ASPECT subtypes that denote a portion of the shape.
    regions: {
        ShapeAspect,                     ShapeAspectId,                     shape_aspect_arena;
        DatumFeature,                    DatumFeatureId,                    datum_feature_arena;
        AllAroundShapeAspect,            AllAroundShapeAspectId,            all_around_shape_aspect_arena;
        CentreOfSymmetry,                CentreOfSymmetryId,                centre_of_symmetry_arena;
        CompositeShapeAspect,            CompositeShapeAspectId,            composite_shape_aspect_arena;
        CompositeGroupShapeAspect,       CompositeGroupShapeAspectId,       composite_group_shape_aspect_arena;
        ContinuousShapeAspect,           ContinuousShapeAspectId,           continuous_shape_aspect_arena;
        DerivedShapeAspect,              DerivedShapeAspectId,              derived_shape_aspect_arena;
    }
    // aspects — SHAPE_ASPECTs in the schema, but not features here (datum
    // callouts / reference structures, tolerance zones, views, dimensions).
    aspects: {
        Datum,                           DatumId,                           datum_arena;
        CommonDatum,                     CommonDatumId,                     common_datum_arena;
        DatumTarget,                     DatumTargetId,                     datum_target_arena;
        DatumReferenceCompartment,       DatumReferenceCompartmentId,       datum_reference_compartment_arena;
        DatumReferenceElement,           DatumReferenceElementId,           datum_reference_element_arena;
        DatumSystem,                     DatumSystemId,                     datum_system_arena;
        DefaultModelGeometricView,       DefaultModelGeometricViewId,       default_model_geometric_view_arena;
        GeneralDatumReference,           GeneralDatumReferenceId,           general_datum_reference_arena;
        PlacedDatumTargetFeature,        PlacedDatumTargetFeatureId,        placed_datum_target_feature_arena;
        ToleranceZone,                   ToleranceZoneId,                   tolerance_zone_arena;
        ToleranceZoneWithDatum,          ToleranceZoneWithDatumId,          tolerance_zone_with_datum_arena;
        DimensionalSizeWithDatumFeature, DimensionalSizeWithDatumFeatureId, dimensional_size_with_datum_feature_arena;
    }
}

/// The `SHAPE_ASPECT` part of a complex instance: its `name`, `description`
/// and `of_shape`. `None` when it has no such part.
fn complex_shape_aspect(
    model: &m::StepModel,
    id: m::ComplexEntityId,
) -> Option<(m::Text, Option<m::Text>, &m::ProductDefinitionShapeRef)> {
    model
        .complex_entity_arena
        .get(id.0)
        .parts
        .iter()
        .find_map(|p| match p {
            m::PartialEntity::ShapeAspect {
                name,
                description,
                of_shape,
                ..
            } => Some((*name, *description, of_shape)),
            _ => None,
        })
}

/// A complex instance as a feature cursor, if it has a `SHAPE_ASPECT` part.
fn complex_feature(model: &m::StepModel, id: m::ComplexEntityId) -> Option<FeatureImpl> {
    complex_shape_aspect(model, id).map(|_| FeatureImpl::Complex(id))
}

impl Scene<'_> {
    /// Every geometric feature in the model — the `SHAPE_ASPECT` subtypes that
    /// denote a region of the part's shape (plain aspects, datum features,
    /// composite / derived / all-around / centre-of-symmetry / continuous
    /// aspects), and complex instances with a `SHAPE_ASPECT` part (a composite
    /// datum feature, for one) that carry none of the parts below. Datum
    /// callouts, datum-system references, tolerance zones, and views are also
    /// `SHAPE_ASPECT`s in the schema but are not features here — datums are
    /// reached via [`Scene::all_datums`], the rest via the raw model.
    pub fn all_features(&self) -> impl Iterator<Item = Feature<'_>> + '_ {
        let cx = self.ctx();
        let mut out: Vec<Feature<'_>> = Vec::new();
        push_all_features(cx, &mut out);
        out.into_iter()
    }
}

impl<'m> Dimension<'m> {
    /// The shape feature(s) this dimension applies to. A size (`DIMENSIONAL_SIZE`
    /// / `ANGULAR_SIZE`) yields the one feature it measures; a location
    /// (`DIMENSIONAL_LOCATION` / `ANGULAR_LOCATION`) yields the two it spans, in
    /// `[relating, related]` order.
    pub fn features(&self) -> impl Iterator<Item = Feature<'m>> + 'm {
        let this = self.about();
        let model = this.cx.model;
        let refs: Vec<&m::ShapeAspectRef> = match this.which {
            DimImpl::Size(i) => vec![&model.dimensional_size_arena.get(i.0).applies_to],
            DimImpl::AngularSize(i) => vec![&model.angular_size_arena.get(i.0).applies_to],
            DimImpl::Location(i) => {
                let e = model.dimensional_location_arena.get(i.0);
                vec![&e.relating_shape_aspect, &e.related_shape_aspect]
            }
            DimImpl::AngularLocation(i) => {
                let e = model.angular_location_arena.get(i.0);
                vec![&e.relating_shape_aspect, &e.related_shape_aspect]
            }
        };
        refs.into_iter()
            .filter_map(move |r| this.feature_from_sa_ref(r))
    }

    fn feature_from_sa_ref(&self, r: &m::ShapeAspectRef) -> Option<Feature<'m>> {
        let which = feature_impl_from_shape_aspect_ref(self.cx.model, r);
        if which.is_none() {
            self.cx.warn(
                "dimension feature is a complex instance without a SHAPE_ASPECT part".to_owned(),
            );
        }
        which.map(|which| Feature { cx: self.cx, which })
    }
}

impl<'m> Tolerance<'m> {
    /// The shape feature this tolerance applies to (its `toleranced_shape_aspect`).
    ///
    /// `None` when the target is not a named shape aspect — a tolerance placed on
    /// a dimension, on the whole part (`PRODUCT_DEFINITION_SHAPE`) or on a
    /// relationship between shape aspects — or when the target is a complex
    /// instance without a `SHAPE_ASPECT` part (reported via
    /// [`Scene::warnings`]).
    pub fn feature(&self) -> Option<Feature<'m>> {
        let this = self.about();
        let target = this.tolerance_target()?;
        let which = feature_impl_from_tolerance_target(this.cx.model, target);
        if which.is_none() && matches!(target, m::GeometricToleranceTargetRef::Complex(_)) {
            this.cx.warn(
                "tolerance target is a complex instance without a SHAPE_ASPECT part".to_owned(),
            );
        }
        which.map(|which| Feature { cx: this.cx, which })
    }

    /// The tolerance's `toleranced_shape_aspect`: from the dedicated arena for a
    /// leaf tolerance, or the `GEOMETRIC_TOLERANCE` part for a complex one.
    fn tolerance_target(&self) -> Option<&'m m::GeometricToleranceTargetRef> {
        if let Some(t) = self.dedicated_target() {
            return Some(t);
        }
        let TolImpl::Complex(cuid) = self.which else {
            return None;
        };
        self.cx
            .model
            .complex_entity_arena
            .get(cuid.0)
            .parts
            .iter()
            .find_map(|p| match p {
                m::PartialEntity::GeometricTolerance {
                    toleranced_shape_aspect,
                    ..
                } => Some(toleranced_shape_aspect),
                _ => None,
            })
    }
}

/// A concrete geometry item a [`Feature`] designates (via
/// `GEOMETRIC_ITEM_SPECIFIC_USAGE` or a plain
/// `ITEM_IDENTIFIED_REPRESENTATION_USAGE`). The common cases resolve to a typed
/// handle; anything else is surfaced as `Other` carrying its key rather than
/// dropped.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum FeatureGeometry<'m> {
    /// An `ADVANCED_FACE` or `FACE_SURFACE`.
    Face(Face<'m>),
    /// An `EDGE_CURVE`.
    Edge(Edge<'m>),
    /// A point: a `CARTESIAN_POINT`, an `APLL_POINT` (with or without
    /// surface), or a complex instance with a `CARTESIAN_POINT` part.
    Point(Point<'m>),
    /// A `MANIFOLD_SOLID_BREP` or `BREP_WITH_VOIDS`.
    Solid(Solid<'m>),
    /// A curve (`TRIMMED_CURVE`, `COMPOSITE_CURVE`, `CIRCLE`, …); dispatch its
    /// exact kind via [`Curve::kind`](crate::scene::Curve::kind).
    Curve(Curve<'m>),
    /// A display mesh (`COMPLEX_TRIANGULATED_FACE`, …).
    Mesh(Mesh<'m>),
    /// A `GEOMETRIC_SET` or `GEOMETRIC_CURVE_SET`.
    GeometricSet(GeometricSet<'m>),
    /// Any other identified item (a tessellated geometric set, a placement, …),
    /// by key.
    Other(m::EntityKey),
}

impl<'m> Feature<'m> {
    /// The concrete geometry this feature designates — the faces / edges / points
    /// / solids / curves / meshes / geometric sets linked to it by a
    /// `GEOMETRIC_ITEM_SPECIFIC_USAGE` (GISU) or a plain
    /// `ITEM_IDENTIFIED_REPRESENTATION_USAGE` (IIRU), whose identified item may
    /// be a set or list of items, each of which counts. A
    /// `DRAUGHTING_MODEL_ITEM_ASSOCIATION` links the feature to its
    /// presentation, not its geometry, and is not followed.
    ///
    /// A composite feature (`COMPOSITE_SHAPE_ASPECT`,
    /// `COMPOSITE_GROUP_SHAPE_ASPECT`, `CONTINUOUS_SHAPE_ASPECT`,
    /// `ALL_AROUND_SHAPE_ASPECT`) or a datum feature (`DATUM_FEATURE`,
    /// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`), or a complex instance with one of
    /// those parts, usually carries no GISU or IIRU itself: its geometry is on its
    /// components, the shape aspects a plain `SHAPE_ASPECT_RELATIONSHIP` names
    /// with this feature as the relating end. Their geometry is included, and
    /// a component that is itself composite or a datum feature is followed in
    /// turn. A `DATUM` or `COMMON_DATUM` at the other end is the datum the
    /// feature establishes, not a component, and is not followed; nor is a
    /// relationship subtype such as a dimensional location.
    ///
    /// Each item comes once, even when several usages designate it. Items
    /// outside the typed set (a tessellated geometric set, a placement, …) come
    /// back as [`FeatureGeometry::Other`] with their key. A derived shape
    /// aspect's deriving relationship is not followed.
    pub fn geometry(&self) -> impl Iterator<Item = FeatureGeometry<'m>> + 'm {
        let cx = self.cx;
        let mut out = Vec::new();
        let mut items = HashSet::new();
        let mut visited = HashSet::from([self.key()]);
        let mut stack = vec![self.key()];
        while let Some(key) = stack.pop() {
            push_identified_geometry(cx, key, &mut items, &mut out);
            if !has_components(cx.model, key) {
                continue;
            }
            for component in components(cx, key) {
                if visited.insert(component) {
                    push_identified_geometry(cx, component, &mut items, &mut out);
                    if has_components(cx.model, component) {
                        stack.push(component);
                    }
                }
            }
        }
        out.into_iter()
    }
}

/// Whether the shape aspect `key` has components to follow: a composite
/// aspect or a datum feature, or a complex instance with such a part.
fn has_components(model: &m::StepModel, key: m::EntityKey) -> bool {
    match key {
        m::EntityKey::CompositeShapeAspect(_)
        | m::EntityKey::CompositeGroupShapeAspect(_)
        | m::EntityKey::ContinuousShapeAspect(_)
        | m::EntityKey::AllAroundShapeAspect(_)
        | m::EntityKey::DatumFeature(_)
        | m::EntityKey::DimensionalSizeWithDatumFeature(_) => true,
        m::EntityKey::ComplexEntity(id) => {
            model.complex_entity_arena.get(id.0).parts.iter().any(|p| {
                matches!(
                    p,
                    m::PartialEntity::CompositeShapeAspect
                        | m::PartialEntity::CompositeGroupShapeAspect
                        | m::PartialEntity::ContinuousShapeAspect
                        | m::PartialEntity::AllAroundShapeAspect
                        | m::PartialEntity::DatumFeature
                        | m::PartialEntity::DimensionalSizeWithDatumFeature
                )
            })
        }
        _ => false,
    }
}

/// Whether the shape aspect `key` is a datum (`DATUM`, `COMMON_DATUM`, or a
/// complex instance with such a part).
fn is_datum(model: &m::StepModel, key: m::EntityKey) -> bool {
    match key {
        m::EntityKey::Datum(_) | m::EntityKey::CommonDatum(_) => true,
        m::EntityKey::ComplexEntity(id) => {
            model.complex_entity_arena.get(id.0).parts.iter().any(|p| {
                matches!(
                    p,
                    m::PartialEntity::Datum { .. } | m::PartialEntity::CommonDatum
                )
            })
        }
        _ => false,
    }
}

/// The components of the shape aspect `key`: the related ends of the plain
/// `SHAPE_ASPECT_RELATIONSHIP`s whose relating end is `key`, less datums.
fn components(cx: Ctx<'_>, key: m::EntityKey) -> Vec<m::EntityKey> {
    let mut out = Vec::new();
    for r in cx.ref_graph().referrers(key) {
        let m::EntityKey::ShapeAspectRelationship(sid) = r else {
            continue;
        };
        let sar = cx.model.shape_aspect_relationship_arena.get(sid.0);
        if sar.relating_shape_aspect.entity_key() != key {
            continue;
        }
        let related = sar.related_shape_aspect.entity_key();
        if !is_datum(cx.model, related) {
            out.push(related);
        }
    }
    out
}

/// Push the geometry the GISUs and plain IIRUs on `key` designate, skipping
/// items in `items`.
fn push_identified_geometry<'m>(
    cx: Ctx<'m>,
    key: m::EntityKey,
    items: &mut HashSet<m::EntityKey>,
    out: &mut Vec<FeatureGeometry<'m>>,
) {
    let mut push = |item: m::EntityKey| {
        if items.insert(item) {
            out.push(feature_geometry(cx, item));
        }
    };
    for &r in cx.ref_graph().referrers(key) {
        match r {
            m::EntityKey::GeometricItemSpecificUsage(gid) => {
                let gisu = cx.model.geometric_item_specific_usage_arena.get(gid.0);
                push(gisu.identified_item.entity_key());
            }
            m::EntityKey::ItemIdentifiedRepresentationUsage(iid) => {
                let iiru = cx
                    .model
                    .item_identified_representation_usage_arena
                    .get(iid.0);
                if iiru.definition.entity_key() != key {
                    continue;
                }
                match &iiru.identified_item {
                    m::ItemIdentifiedRepresentationUsageSelectRef::SetRepresentationItem(v)
                    | m::ItemIdentifiedRepresentationUsageSelectRef::ListRepresentationItem(v) => {
                        for element in v {
                            push(element.entity_key());
                        }
                    }
                    single => {
                        if let Some(item) = single.entity_key() {
                            push(item);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// The identified item `key` as a [`FeatureGeometry`]: a typed handle for a
/// geometric model item the scene models, else `Other`.
fn feature_geometry(cx: Ctx<'_>, key: m::EntityKey) -> FeatureGeometry<'_> {
    let Ok(item) = m::GeometricModelItemRef::from_any(key) else {
        return FeatureGeometry::Other(key);
    };
    match &item {
        m::GeometricModelItemRef::AdvancedFace(id) => {
            FeatureGeometry::Face(Face::from_advanced_face(cx, *id))
        }
        m::GeometricModelItemRef::FaceSurface(id) => {
            FeatureGeometry::Face(Face::from_face_surface(cx, *id))
        }
        m::GeometricModelItemRef::EdgeCurve(id) => FeatureGeometry::Edge(Edge::from_id(cx, *id)),
        m::GeometricModelItemRef::CartesianPoint(id) => {
            FeatureGeometry::Point(Point::from_id(cx, *id))
        }
        m::GeometricModelItemRef::ManifoldSolidBrep(id) => {
            FeatureGeometry::Solid(Solid::from_id(cx, *id))
        }
        m::GeometricModelItemRef::BrepWithVoids(id) => {
            FeatureGeometry::Solid(Solid::from_void_id(cx, *id))
        }
        other => {
            if let Some(p) = Point::from_key(cx, key) {
                FeatureGeometry::Point(p)
            } else if let Some(c) = Curve::from_model_item(cx, other) {
                FeatureGeometry::Curve(c)
            } else if let Some(mesh) = Mesh::from_model_item(cx, other) {
                FeatureGeometry::Mesh(mesh)
            } else if let Some(set) = GeometricSet::from_model_item(cx, other) {
                FeatureGeometry::GeometricSet(set)
            } else {
                FeatureGeometry::Other(key)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-part PMI: which product definition a feature / datum / tolerance belongs
// to, and the filtered per-part queries backing `ProductDef`.
// ---------------------------------------------------------------------------

/// The product definition a `PRODUCT_DEFINITION_SHAPE` defines, as an
/// [`EntityKey`](m::EntityKey), if its `definition` is a `PRODUCT_DEFINITION` or
/// its `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` subtype (both are surfaced
/// as a [`ProductDef`](crate::scene::product::ProductDef)); `None` otherwise.
fn product_def_of_pds(cx: Ctx<'_>, id: m::ProductDefinitionShapeId) -> Option<m::EntityKey> {
    match cx.model.product_definition_shape_arena.get(id.0).definition {
        m::CharacterizedDefinitionRef::ProductDefinition(pd) => {
            Some(m::EntityKey::ProductDefinition(pd))
        }
        m::CharacterizedDefinitionRef::ProductDefinitionWithAssociatedDocuments(w) => {
            Some(m::EntityKey::ProductDefinitionWithAssociatedDocuments(w))
        }
        _ => None,
    }
}

/// The product definition an `of_shape` reference resolves to (its PDS's
/// `definition`); `None` for a complex PDS or a non-`PRODUCT_DEFINITION` owner.
fn product_def_of_shape(cx: Ctx<'_>, r: &m::ProductDefinitionShapeRef) -> Option<m::EntityKey> {
    match r {
        m::ProductDefinitionShapeRef::ProductDefinitionShape(id) => product_def_of_pds(cx, *id),
        m::ProductDefinitionShapeRef::Complex(_) => {
            cx.warn("of_shape is a complex instance; its part is not read".to_owned());
            None
        }
    }
}

impl Feature<'_> {
    /// The product definition this feature belongs to (`of_shape` → PDS →
    /// definition), as an [`EntityKey`](m::EntityKey) — a `PRODUCT_DEFINITION` or
    /// its `..._WITH_ASSOCIATED_DOCUMENTS` subtype.
    fn part_def_id(&self) -> Option<m::EntityKey> {
        let this = self.about();
        product_def_of_shape(this.cx, this.of_shape()?)
    }
}

impl Datum<'_> {
    /// The product definition this datum belongs to (`of_shape` → PDS → part).
    fn part_def_id(&self) -> Option<m::EntityKey> {
        let this = self.about();
        let of_shape = match this.which {
            DatumImpl::Datum(i) => &this.cx.model.datum_arena.get(i.0).of_shape,
            DatumImpl::CommonDatum(i) => &this.cx.model.common_datum_arena.get(i.0).of_shape,
        };
        product_def_of_shape(this.cx, of_shape)
    }
}

impl Tolerance<'_> {
    /// The product definitions this tolerance belongs to: its target may be a
    /// shape-aspect feature (→ that feature's part), the whole part's
    /// `PRODUCT_DEFINITION_SHAPE` (an all-over tolerance), or a dimension or
    /// a relationship between shape aspects (→ the parts of the aspects it
    /// joins, as for [`dimensions_of`]).
    fn part_def_ids(&self) -> Vec<m::EntityKey> {
        let this = self.about();
        let cx = this.cx;
        let model = cx.model;
        let Some(target) = this.tolerance_target() else {
            return Vec::new();
        };
        let aspects: Vec<&m::ShapeAspectRef> = match target {
            m::GeometricToleranceTargetRef::ProductDefinitionShape(id) => {
                return product_def_of_pds(cx, *id).into_iter().collect();
            }
            m::GeometricToleranceTargetRef::Complex(i) => {
                let Some(which) = complex_feature(model, *i) else {
                    cx.warn(
                        "tolerance target is a complex instance without a SHAPE_ASPECT part"
                            .to_owned(),
                    );
                    return Vec::new();
                };
                return Feature { cx, which }.part_def_id().into_iter().collect();
            }
            m::GeometricToleranceTargetRef::DimensionalSize(i) => {
                vec![&model.dimensional_size_arena.get(i.0).applies_to]
            }
            m::GeometricToleranceTargetRef::AngularSize(i) => {
                vec![&model.angular_size_arena.get(i.0).applies_to]
            }
            m::GeometricToleranceTargetRef::DimensionalSizeWithPath(i) => {
                vec![&model.dimensional_size_with_path_arena.get(i.0).applies_to]
            }
            m::GeometricToleranceTargetRef::DimensionalLocation(i) => {
                let d = model.dimensional_location_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::AngularLocation(i) => {
                let d = model.angular_location_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::DimensionalLocationWithPath(i) => {
                let d = model.dimensional_location_with_path_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::DirectedDimensionalLocation(i) => {
                let d = model.directed_dimensional_location_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::ShapeAspectRelationship(i) => {
                let d = model.shape_aspect_relationship_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::ShapeAspectAssociativity(i) => {
                let d = model.shape_aspect_associativity_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::ShapeAspectDerivingRelationship(i) => {
                let d = model.shape_aspect_deriving_relationship_arena.get(i.0);
                vec![&d.relating_shape_aspect, &d.related_shape_aspect]
            }
            m::GeometricToleranceTargetRef::FeatureForDatumTargetRelationship(i) => {
                vec![
                    &model
                        .feature_for_datum_target_relationship_arena
                        .get(i.0)
                        .relating_shape_aspect,
                ]
            }
            other => {
                let Some(which) = feature_impl_from_tolerance_target(model, other) else {
                    return Vec::new();
                };
                return Feature { cx, which }.part_def_id().into_iter().collect();
            }
        };
        let mut parts = Vec::new();
        for aspect in aspects {
            let Some(which) = feature_impl_from_shape_aspect_ref(model, aspect) else {
                cx.warn(
                    "tolerance target's shape aspect is a complex instance without a \
                     SHAPE_ASPECT part"
                        .to_owned(),
                );
                continue;
            };
            if let Some(part) = (Feature { cx, which }).part_def_id() {
                if !parts.contains(&part) {
                    parts.push(part);
                }
            }
        }
        parts
    }
}

/// The shape-region features (narrow set) belonging to `part`.
pub(crate) fn features_of(cx: Ctx<'_>, part: m::EntityKey) -> Vec<Feature<'_>> {
    let mut all = Vec::new();
    push_all_features(cx, &mut all);
    all.retain(|f| f.part_def_id() == Some(part));
    all
}

/// The datums belonging to `part`.
pub(crate) fn datums_of(cx: Ctx<'_>, part: m::EntityKey) -> Vec<Datum<'_>> {
    let mut all = all_datums(cx);
    all.retain(|d| d.part_def_id() == Some(part));
    all
}

/// The dimensions belonging to `part` — those whose targeted feature is on it.
pub(crate) fn dimensions_of(cx: Ctx<'_>, part: m::EntityKey) -> Vec<Dimension<'_>> {
    let mut all = all_dimensions(cx);
    all.retain(|d| d.features().any(|f| f.part_def_id() == Some(part)));
    all
}

/// The tolerances belonging to `part` — those whose target (a feature, the
/// whole part, or a dimension or relationship joining a feature on it) is on
/// it.
pub(crate) fn tolerances_of(cx: Ctx<'_>, part: m::EntityKey) -> Vec<Tolerance<'_>> {
    let mut all = all_tolerances(cx);
    all.retain(|t| t.part_def_ids().contains(&part));
    all
}

about_by_key!(Datum, Dimension, Feature, Tolerance);
