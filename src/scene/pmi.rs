//! PMI — the product and manufacturing information annotating a shape.
//!
//! Model-wide slices so far: **dimensions** ([`Scene::dimensions`]),
//! **geometric tolerances** ([`Scene::tolerances`]), and **datums**
//! ([`Scene::datums`]). A dimension carries a [`kind`](Dimension::kind) and a
//! nominal [`value`](Dimension::value); a tolerance its
//! [`kind`](Tolerance::kind), [`magnitude`](Tolerance::magnitude), and
//! referenced [`datums`](Tolerance::datums). Both point at the shape
//! [`Feature`] they apply to ([`Dimension::features`] /
//! [`Tolerance::feature`]).

// Every public method here is a pure read accessor; `#[must_use]` on each would be
// pure noise, so allow the pedantic lint module-wide.
#![allow(clippy::must_use_candidate)]

use crate::generated::model as m;
use crate::scene::geometry::{Curve, Edge, Face, Point, Solid};
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
    pub fn dimensions(&self) -> impl Iterator<Item = Dimension<'_>> + '_ {
        all_dimensions(self.ctx()).into_iter()
    }
}

/// Every dimension in the model (the four base types). Shared by
/// [`Scene::dimensions`] and the per-part [`crate::scene::ProductDef`] queries.
fn all_dimensions(cx: Ctx<'_>) -> Vec<Dimension<'_>> {
    let mut out = Vec::new();
    for i in 0..cx.model.dimensional_size_arena.items.len() {
        out.push(Dimension {
            cx,
            which: DimImpl::Size(m::DimensionalSizeId(i)),
        });
    }
    for i in 0..cx.model.dimensional_location_arena.items.len() {
        out.push(Dimension {
            cx,
            which: DimImpl::Location(m::DimensionalLocationId(i)),
        });
    }
    for i in 0..cx.model.angular_size_arena.items.len() {
        out.push(Dimension {
            cx,
            which: DimImpl::AngularSize(m::AngularSizeId(i)),
        });
    }
    for i in 0..cx.model.angular_location_arena.items.len() {
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
            DimImpl::Size(i) => &self.cx.model.dimensional_size_arena.get(i.0).name,
            DimImpl::Location(i) => &self.cx.model.dimensional_location_arena.get(i.0).name,
            DimImpl::AngularSize(i) => &self.cx.model.angular_size_arena.get(i.0).name,
            DimImpl::AngularLocation(i) => &self.cx.model.angular_location_arena.get(i.0).name,
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
    /// referencing this dimension → its `SHAPE_DIMENSION_REPRESENTATION`'s first
    /// `MEASURE_REPRESENTATION_ITEM`).
    ///
    /// The number is raw: interpret its unit via [`kind`](Dimension::kind)
    /// (`Size`/`Location` are lengths, `Angular*` are angles) and
    /// [`Scene::units`], exactly as point coordinates are read. A representation
    /// the walk cannot resolve (a complex one) is reported via
    /// [`Scene::warnings`](crate::scene::Scene); a dimension with no value
    /// representation simply yields `None`.
    pub fn value(&self) -> Option<f64> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        for r in rg.referrers(self.key()) {
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
                if let m::RepresentationItemRef::MeasureRepresentationItem(mid) = item {
                    return Some(
                        cx.model
                            .measure_representation_item_arena
                            .get(mid.0)
                            .value_component
                            .value
                            .as_f64(),
                    );
                }
            }
        }
        None
    }
}

/// The GD&T characteristic a [`Tolerance`] specifies (its control-frame symbol).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// (reported via [`Scene::warnings`](crate::scene::Scene)).
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
            Complex(m::ComplexUnitId),
        }

        fn push_dedicated_tolerances<'m>(cx: Ctx<'m>, out: &mut Vec<Tolerance<'m>>) {
            $(
                for i in 0..cx.model.$arena.items.len() {
                    out.push(Tolerance { cx, which: TolImpl::$ent(m::$id(i)) });
                }
            )+
        }

        impl<'m> Tolerance<'m> {
            /// The GD&T characteristic this tolerance specifies.
            pub fn kind(&self) -> ToleranceKind {
                match self.which {
                    $( TolImpl::$ent(_) => ToleranceKind::$kind, )+
                    TolImpl::Complex(cuid) => self.complex_kind(cuid),
                }
            }

            /// This tolerance's global identity (a `Copy` key for maps / dedup).
            pub fn key(&self) -> m::EntityKey {
                match self.which {
                    $( TolImpl::$ent(i) => m::EntityKey::$ent(i), )+
                    TolImpl::Complex(cuid) => m::EntityKey::ComplexUnit(cuid),
                }
            }

            /// `(name, magnitude)` for a dedicated-arena leaf tolerance.
            fn dedicated_fields(&self) -> Option<(&'m str, Option<&'m m::LengthMeasureWithUnitRef>)> {
                match self.which {
                    $( TolImpl::$ent(i) => {
                        let e = self.cx.model.$arena.get(i.0);
                        Some((&e.name, e.magnitude.as_ref()))
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
    /// instances (any `ComplexUnit` carrying a `GEOMETRIC_TOLERANCE` facet).
    pub fn tolerances(&self) -> impl Iterator<Item = Tolerance<'_>> + '_ {
        all_tolerances(self.ctx()).into_iter()
    }
}

/// Every geometric tolerance in the model — standalone leaf entities plus complex
/// multi-supertype instances. Shared by [`Scene::tolerances`] and per-part queries.
fn all_tolerances(cx: Ctx<'_>) -> Vec<Tolerance<'_>> {
    let mut out: Vec<Tolerance<'_>> = Vec::new();
    push_dedicated_tolerances(cx, &mut out);
    for i in 0..cx.model.complex_unit_arena.items.len() {
        let cu = cx.model.complex_unit_arena.get(i);
        if cu
            .parts
            .iter()
            .any(|p| matches!(p, m::UnitPart::GeometricTolerance { .. }))
        {
            out.push(Tolerance {
                cx,
                which: TolImpl::Complex(m::ComplexUnitId(i)),
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
    /// The number is raw, in the model's length unit — interpret it via
    /// [`Scene::units`], exactly as point coordinates and dimension values are
    /// read. `None` when the tolerance carries no magnitude, or its magnitude is
    /// a complex measure instance the walk cannot resolve (reported via
    /// [`Scene::warnings`](crate::scene::Scene)).
    pub fn magnitude(&self) -> Option<f64> {
        let (_, mag) = self
            .dedicated_fields()
            .or_else(|| self.complex_geometric_tolerance())?;
        mag.and_then(|r| length_measure_f64(self.cx, r))
    }

    /// The `GEOMETRIC_TOLERANCE` supertype facet of a complex tolerance, as
    /// `(name, magnitude)`; `None` for a dedicated-arena tolerance.
    fn complex_geometric_tolerance(
        &self,
    ) -> Option<(&'m str, Option<&'m m::LengthMeasureWithUnitRef>)> {
        let TolImpl::Complex(cuid) = self.which else {
            return None;
        };
        let cu = self.cx.model.complex_unit_arena.get(cuid.0);
        cu.parts.iter().find_map(|p| match p {
            m::UnitPart::GeometricTolerance {
                name, magnitude, ..
            } => Some((name.as_str(), magnitude.as_ref())),
            _ => None,
        })
    }

    /// The characteristic of a complex tolerance, read from its leaf-subtype
    /// facet. Only the leaf markers that occur in the corpus exist as
    /// `UnitPart` variants; a complex tolerance without a recognized one is
    /// surfaced as `Other`.
    fn complex_kind(&self, cuid: m::ComplexUnitId) -> ToleranceKind {
        let cu = self.cx.model.complex_unit_arena.get(cuid.0);
        for p in &cu.parts {
            let kind = match p {
                m::UnitPart::PositionTolerance => ToleranceKind::Position,
                m::UnitPart::FlatnessTolerance => ToleranceKind::Flatness,
                m::UnitPart::StraightnessTolerance => ToleranceKind::Straightness,
                m::UnitPart::RoundnessTolerance => ToleranceKind::Roundness,
                m::UnitPart::CylindricityTolerance => ToleranceKind::Cylindricity,
                m::UnitPart::LineProfileTolerance => ToleranceKind::LineProfile,
                m::UnitPart::SurfaceProfileTolerance => ToleranceKind::SurfaceProfile,
                m::UnitPart::ParallelismTolerance => ToleranceKind::Parallelism,
                m::UnitPart::PerpendicularityTolerance => ToleranceKind::Perpendicularity,
                m::UnitPart::CircularRunoutTolerance => ToleranceKind::CircularRunout,
                _ => continue,
            };
            return kind;
        }
        self.cx
            .warn("geometric tolerance has no recognized characteristic subtype".to_owned());
        ToleranceKind::Other
    }
}

/// Resolve a `LENGTH_MEASURE_WITH_UNIT` reference to its scalar value; a complex
/// measure instance is reported and yields `None`.
fn length_measure_f64(cx: Ctx<'_>, r: &m::LengthMeasureWithUnitRef) -> Option<f64> {
    match r {
        m::LengthMeasureWithUnitRef::LengthMeasureWithUnit(id) => Some(
            cx.model
                .length_measure_with_unit_arena
                .get(id.0)
                .value_component
                .value
                .as_f64(),
        ),
        m::LengthMeasureWithUnitRef::Complex(_) => {
            cx.warn("tolerance magnitude is a complex measure instance".to_owned());
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
    /// a datum is established from is a different entity, reached later.
    pub fn datums(&self) -> impl Iterator<Item = Datum<'_>> + '_ {
        all_datums(self.ctx()).into_iter()
    }
}

/// Every datum (`DATUM` / `COMMON_DATUM`) in the model. Shared by
/// [`Scene::datums`] and per-part queries.
fn all_datums(cx: Ctx<'_>) -> Vec<Datum<'_>> {
    let mut out: Vec<Datum<'_>> = Vec::new();
    for i in 0..cx.model.datum_arena.items.len() {
        out.push(Datum {
            cx,
            which: DatumImpl::Datum(m::DatumId(i)),
        });
    }
    for i in 0..cx.model.common_datum_arena.items.len() {
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
            DatumImpl::Datum(i) => &self.cx.model.datum_arena.get(i.0).identification,
            DatumImpl::CommonDatum(i) => &self.cx.model.common_datum_arena.get(i.0).identification,
        }
    }

    /// The datum's name.
    pub fn name(&self) -> &'m str {
        match self.which {
            DatumImpl::Datum(i) => &self.cx.model.datum_arena.get(i.0).name,
            DatumImpl::CommonDatum(i) => &self.cx.model.common_datum_arena.get(i.0).name,
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
    /// linked to it by a `SHAPE_ASPECT_RELATIONSHIP`. Returned as a [`Feature`],
    /// whose faces/edges are reachable via [`Feature::geometry`]. `None` when the
    /// datum is defined only by datum targets, or carries no such link.
    pub fn datum_feature(&self) -> Option<Feature<'m>> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        for r in rg.referrers(self.key()) {
            let m::EntityKey::ShapeAspectRelationship(sid) = r else {
                continue;
            };
            let sar = cx.model.shape_aspect_relationship_arena.get(sid.0);
            // The datum is one end of the relationship; the datum feature is the
            // other (checking both ends naturally selects it).
            for aspect in [&sar.relating_shape_aspect, &sar.related_shape_aspect] {
                if let m::ShapeAspectRef::DatumFeature(fid) = aspect {
                    return Some(Feature {
                        cx,
                        which: FeatureImpl::DatumFeature(*fid),
                    });
                }
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
    /// datum. A common datum (`A-B`) is flattened to its constituent datums; a
    /// reference the walk cannot resolve (a complex facet) is reported via
    /// [`Scene::warnings`](crate::scene::Scene).
    pub fn datums(&self) -> Vec<Datum<'m>> {
        // `precedence` is authored per DATUM_REFERENCE but a DATUM_SYSTEM orders
        // its compartments positionally; collect a sort key from whichever form
        // applies, then order by it so the frame reads primary-first regardless
        // of file order.
        let mut keyed: Vec<(i64, Datum<'m>)> = Vec::new();
        for r in self.datum_system() {
            self.collect_system_ref(r, &mut keyed);
        }
        keyed.sort_by_key(|(k, _)| *k);
        keyed.into_iter().map(|(_, d)| d).collect()
    }

    /// The tolerance's `datum_system` list: on the dedicated arena for the eight
    /// leaf subtypes that carry one, on the `GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE`
    /// facet for a complex tolerance, and empty for the datum-free subtypes.
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
                .complex_unit_arena
                .get(cuid.0)
                .parts
                .iter()
                .find_map(|p| match p {
                    m::UnitPart::GeometricToleranceWithDatumReference { datum_system } => {
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
                    let m::DatumReferenceCompartmentRef::DatumReferenceCompartment(cid) = c;
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
                    let m::DatumReferenceElementRef::DatumReferenceElement(eid) = er;
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
/// or occasionally a datum / tolerance-zone construct. [`key`](Feature::key)
/// reveals the exact kind when it matters.
#[derive(Clone, Copy)]
pub struct Feature<'m> {
    cx: Ctx<'m>,
    which: FeatureImpl,
}

// The 20 SHAPE_ASPECT-family entities all share `name` / `description` / a
// dedicated arena / an `EntityKey` variant. This table generates the cursor enum,
// the accessors, and the two reference-union converters — all covering the full
// family, since a dimension / tolerance target may point at any of them.
//
// The family splits in two: `regions` are the SHAPE_ASPECT subtypes that denote a
// portion of the part's shape (a feature) and are what `Scene::features` lists;
// `aspects` are also SHAPE_ASPECTs in the schema but are datum callouts, datum
// reference structures, tolerance zones, views, or dimensions — reached via
// their own queries or the raw model. Corpus-checked: no dimension / tolerance
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
        }

        /// The concrete kind of a [`Feature`] — which `SHAPE_ASPECT`-family
        /// entity backs it, without matching on [`Feature::key`]'s `EntityKey`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum FeatureKind {
            $( $rent, )+
            $( $aent, )+
        }

        impl<'m> Feature<'m> {
            /// The concrete kind backing this feature.
            pub fn kind(&self) -> FeatureKind {
                match self.which {
                    $( FeatureImpl::$rent(_) => FeatureKind::$rent, )+
                    $( FeatureImpl::$aent(_) => FeatureKind::$aent, )+
                }
            }
            /// The feature's name.
            pub fn name(&self) -> &'m str {
                match self.which {
                    $( FeatureImpl::$rent(i) => &self.cx.model.$rarena.get(i.0).name, )+
                    $( FeatureImpl::$aent(i) => &self.cx.model.$aarena.get(i.0).name, )+
                }
            }

            /// The feature's description, if any.
            pub fn description(&self) -> Option<&'m str> {
                match self.which {
                    $( FeatureImpl::$rent(i) => {
                        self.cx.model.$rarena.get(i.0).description.as_deref()
                    } )+
                    $( FeatureImpl::$aent(i) => {
                        self.cx.model.$aarena.get(i.0).description.as_deref()
                    } )+
                }
            }

            /// This feature's global identity (a `Copy` key for maps / dedup).
            pub fn key(&self) -> m::EntityKey {
                match self.which {
                    $( FeatureImpl::$rent(i) => m::EntityKey::$rent(i), )+
                    $( FeatureImpl::$aent(i) => m::EntityKey::$aent(i), )+
                }
            }

            /// The `of_shape` product-definition-shape this feature belongs to.
            fn of_shape(&self) -> &'m m::ProductDefinitionShapeRef {
                match self.which {
                    $( FeatureImpl::$rent(i) => &self.cx.model.$rarena.get(i.0).of_shape, )+
                    $( FeatureImpl::$aent(i) => &self.cx.model.$aarena.get(i.0).of_shape, )+
                }
            }
        }

        /// A `ShapeAspectRef` (a dimension's target) → a feature cursor. Every
        /// non-`Complex` variant is a shape-aspect-family member.
        fn feature_impl_from_shape_aspect_ref(r: &m::ShapeAspectRef) -> Option<FeatureImpl> {
            match r {
                $( m::ShapeAspectRef::$rent(i) => Some(FeatureImpl::$rent(*i)), )+
                $( m::ShapeAspectRef::$aent(i) => Some(FeatureImpl::$aent(*i)), )+
                m::ShapeAspectRef::Complex(_) => None,
            }
        }

        /// A `GeometricToleranceTargetRef` (a tolerance's target) → a feature
        /// cursor. The shape-aspect-family arms resolve; the dimension / product /
        /// complex arms are not features and yield `None`.
        fn feature_impl_from_tolerance_target(
            r: &m::GeometricToleranceTargetRef,
        ) -> Option<FeatureImpl> {
            match r {
                $( m::GeometricToleranceTargetRef::$rent(i) => Some(FeatureImpl::$rent(*i)), )+
                $( m::GeometricToleranceTargetRef::$aent(i) => Some(FeatureImpl::$aent(*i)), )+
                _ => None,
            }
        }

        /// Every shape-region feature (the `regions` group) in the model — what
        /// [`Scene::features`] enumerates.
        fn push_all_features<'m>(cx: Ctx<'m>, out: &mut Vec<Feature<'m>>) {
            $(
                for i in 0..cx.model.$rarena.items.len() {
                    out.push(Feature { cx, which: FeatureImpl::$rent(m::$rid(i)) });
                }
            )+
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

impl Scene<'_> {
    /// Every geometric feature in the model — the `SHAPE_ASPECT` subtypes that
    /// denote a region of the part's shape (plain aspects, datum features,
    /// composite / derived / all-around / centre-of-symmetry / continuous
    /// aspects). Datum callouts, datum-system references, tolerance zones, and
    /// views are also `SHAPE_ASPECT`s in the schema but are not features here —
    /// datums are reached via [`Scene::datums`], the rest via the raw model.
    pub fn features(&self) -> impl Iterator<Item = Feature<'_>> + '_ {
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
    pub fn features(&self) -> Vec<Feature<'m>> {
        let model = self.cx.model;
        let refs: Vec<&m::ShapeAspectRef> = match self.which {
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
            .filter_map(|r| self.feature_from_sa_ref(r))
            .collect()
    }

    fn feature_from_sa_ref(&self, r: &m::ShapeAspectRef) -> Option<Feature<'m>> {
        if matches!(r, m::ShapeAspectRef::Complex(_)) {
            self.cx
                .warn("dimension feature is a complex instance".to_owned());
            return None;
        }
        feature_impl_from_shape_aspect_ref(r).map(|which| Feature { cx: self.cx, which })
    }
}

impl<'m> Tolerance<'m> {
    /// The shape feature this tolerance applies to (its `toleranced_shape_aspect`).
    ///
    /// `None` when the target is not a named shape aspect — a tolerance placed on
    /// a dimension or on the whole part (`PRODUCT_DEFINITION_SHAPE`) — or when the
    /// target is a complex instance the walk cannot resolve (reported via
    /// [`Scene::warnings`](crate::scene::Scene)).
    pub fn feature(&self) -> Option<Feature<'m>> {
        let target = self.tolerance_target()?;
        if matches!(target, m::GeometricToleranceTargetRef::Complex(_)) {
            self.cx
                .warn("tolerance target is a complex instance".to_owned());
            return None;
        }
        feature_impl_from_tolerance_target(target).map(|which| Feature { cx: self.cx, which })
    }

    /// The tolerance's `toleranced_shape_aspect`: from the dedicated arena for a
    /// leaf tolerance, or the `GEOMETRIC_TOLERANCE` facet for a complex one.
    fn tolerance_target(&self) -> Option<&'m m::GeometricToleranceTargetRef> {
        if let Some(t) = self.dedicated_target() {
            return Some(t);
        }
        let TolImpl::Complex(cuid) = self.which else {
            return None;
        };
        self.cx
            .model
            .complex_unit_arena
            .get(cuid.0)
            .parts
            .iter()
            .find_map(|p| match p {
                m::UnitPart::GeometricTolerance {
                    toleranced_shape_aspect,
                    ..
                } => Some(toleranced_shape_aspect),
                _ => None,
            })
    }
}

/// A concrete geometry item a [`Feature`] designates (via
/// `GEOMETRIC_ITEM_SPECIFIC_USAGE`). The common cases resolve to a typed handle;
/// anything else is surfaced as `Other` carrying its key rather than dropped.
pub enum FeatureGeometry<'m> {
    /// An `ADVANCED_FACE`.
    Face(Face<'m>),
    /// An `EDGE_CURVE`.
    Edge(Edge<'m>),
    /// A `CARTESIAN_POINT`.
    Point(Point<'m>),
    /// A `MANIFOLD_SOLID_BREP`.
    Solid(Solid<'m>),
    /// A curve (`TRIMMED_CURVE`, `COMPOSITE_CURVE`, `CIRCLE`, …); dispatch its
    /// exact kind via [`Curve::kind`](crate::scene::geometry::Curve::kind).
    Curve(Curve<'m>),
    /// Any other identified item (a tessellated face, a geometric set, …), by key.
    Other(m::EntityKey),
}

impl<'m> Feature<'m> {
    /// The concrete geometry this feature designates — the faces / edges / points
    /// / solids linked to it by `GEOMETRIC_ITEM_SPECIFIC_USAGE` (GISU).
    ///
    /// A feature may designate several items (one per GISU). Items outside the
    /// typed set (curves, tessellated faces, …) come back as
    /// [`FeatureGeometry::Other`] with their key. Only GISUs whose `definition`
    /// points directly at this feature are followed; indirect links through a
    /// shape-aspect relationship are not yet resolved.
    pub fn geometry(&self) -> Vec<FeatureGeometry<'m>> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let mut out = Vec::new();
        for r in rg.referrers(self.key()) {
            let m::EntityKey::GeometricItemSpecificUsage(gid) = r else {
                continue;
            };
            let item = &cx
                .model
                .geometric_item_specific_usage_arena
                .get(gid.0)
                .identified_item;
            out.push(match item {
                m::GeometricModelItemRef::AdvancedFace(id) => {
                    FeatureGeometry::Face(Face::from_advanced_face(cx, *id))
                }
                m::GeometricModelItemRef::EdgeCurve(id) => {
                    FeatureGeometry::Edge(Edge::from_id(cx, *id))
                }
                m::GeometricModelItemRef::CartesianPoint(id) => {
                    FeatureGeometry::Point(Point::from_id(cx, *id))
                }
                m::GeometricModelItemRef::ManifoldSolidBrep(id) => {
                    FeatureGeometry::Solid(Solid::from_id(cx, *id))
                }
                other => match Curve::from_model_item(cx, other) {
                    Some(c) => FeatureGeometry::Curve(c),
                    None => FeatureGeometry::Other(other.entity_key()),
                },
            });
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Per-part PMI: which product definition a feature / datum / tolerance belongs
// to, and the filtered per-part queries backing `ProductDef`.
// ---------------------------------------------------------------------------

/// The product definition a `PRODUCT_DEFINITION_SHAPE` defines, if its
/// `definition` is a plain `PRODUCT_DEFINITION`.
fn product_def_of_pds(
    cx: Ctx<'_>,
    id: m::ProductDefinitionShapeId,
) -> Option<m::ProductDefinitionId> {
    match cx.model.product_definition_shape_arena.get(id.0).definition {
        m::CharacterizedDefinitionRef::ProductDefinition(pd) => Some(pd),
        _ => None,
    }
}

/// The product definition an `of_shape` reference resolves to (its PDS's
/// `definition`); `None` for a complex PDS or a non-`PRODUCT_DEFINITION` owner.
fn product_def_of_shape(
    cx: Ctx<'_>,
    r: &m::ProductDefinitionShapeRef,
) -> Option<m::ProductDefinitionId> {
    match r {
        m::ProductDefinitionShapeRef::ProductDefinitionShape(id) => product_def_of_pds(cx, *id),
        m::ProductDefinitionShapeRef::Complex(_) => None,
    }
}

impl Feature<'_> {
    /// The product definition this feature belongs to (`of_shape` → PDS →
    /// definition), if it resolves to a plain `PRODUCT_DEFINITION`.
    fn part_def_id(&self) -> Option<m::ProductDefinitionId> {
        product_def_of_shape(self.cx, self.of_shape())
    }
}

impl Datum<'_> {
    /// The product definition this datum belongs to (`of_shape` → PDS → part).
    fn part_def_id(&self) -> Option<m::ProductDefinitionId> {
        let of_shape = match self.which {
            DatumImpl::Datum(i) => &self.cx.model.datum_arena.get(i.0).of_shape,
            DatumImpl::CommonDatum(i) => &self.cx.model.common_datum_arena.get(i.0).of_shape,
        };
        product_def_of_shape(self.cx, of_shape)
    }
}

impl Tolerance<'_> {
    /// The product definition this tolerance belongs to: its target may be a
    /// shape-aspect feature (→ that feature's part) or the whole part's
    /// `PRODUCT_DEFINITION_SHAPE` directly (an all-over tolerance).
    fn part_def_id(&self) -> Option<m::ProductDefinitionId> {
        let target = self.tolerance_target()?;
        if let m::GeometricToleranceTargetRef::ProductDefinitionShape(id) = target {
            return product_def_of_pds(self.cx, *id);
        }
        let which = feature_impl_from_tolerance_target(target)?;
        Feature { cx: self.cx, which }.part_def_id()
    }
}

/// The shape-region features (narrow set) belonging to `part`.
pub(crate) fn features_of(cx: Ctx<'_>, part: m::ProductDefinitionId) -> Vec<Feature<'_>> {
    let mut all = Vec::new();
    push_all_features(cx, &mut all);
    all.retain(|f| f.part_def_id() == Some(part));
    all
}

/// The datums belonging to `part`.
pub(crate) fn datums_of(cx: Ctx<'_>, part: m::ProductDefinitionId) -> Vec<Datum<'_>> {
    let mut all = all_datums(cx);
    all.retain(|d| d.part_def_id() == Some(part));
    all
}

/// The dimensions belonging to `part` — those whose targeted feature is on it.
pub(crate) fn dimensions_of(cx: Ctx<'_>, part: m::ProductDefinitionId) -> Vec<Dimension<'_>> {
    let mut all = all_dimensions(cx);
    all.retain(|d| d.features().iter().any(|f| f.part_def_id() == Some(part)));
    all
}

/// The tolerances belonging to `part` — those whose target (feature or whole
/// part) is on it.
pub(crate) fn tolerances_of(cx: Ctx<'_>, part: m::ProductDefinitionId) -> Vec<Tolerance<'_>> {
    let mut all = all_tolerances(cx);
    all.retain(|t| t.part_def_id() == Some(part));
    all
}
