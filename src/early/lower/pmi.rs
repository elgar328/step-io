//! PMI-domain `lower` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyAngularLocation, EarlyAngularSize, EarlyAngularityTolerance,
    EarlyAnnotationCurveOccurrence, EarlyAnnotationOccurrence,
    EarlyAnnotationOccurrenceAssociativity, EarlyAnnotationPlaceholderOccurrence,
    EarlyAnnotationPlaceholderOccurrenceWithLeaderLine, EarlyAnnotationPlane,
    EarlyAnnotationSymbolOccurrence, EarlyAnnotationTextOccurrence,
    EarlyAnnotationToModelLeaderLine, EarlyApllPoint, EarlyApllPointWithSurface, EarlyAreaUnitType,
    EarlyAuxiliaryLeaderLine, EarlyCircularRunoutTolerance, EarlyCircularRunoutToleranceComplex,
    EarlyConcentricityTolerance, EarlyCylindricityTolerance, EarlyDatum, EarlyDatumFeature,
    EarlyDatumOrCommonDatum, EarlyDatumReferenceCompartment, EarlyDatumReferenceElement,
    EarlyDimensionalLocation, EarlyDimensionalSize, EarlyDimensionalSizeWithDatumFeature,
    EarlyDirectedDimensionalLocation, EarlyDraughtingAnnotationOccurrence, EarlyDraughtingCallout,
    EarlyDraughtingCalloutRelationship, EarlyDraughtingModelItemAssociation,
    EarlyDraughtingModelItemAssociationWithPlaceholder, EarlyDraughtingPreDefinedTextFont,
    EarlyFlatnessTolerance, EarlyFlatnessToleranceComplex, EarlyGeometricToleranceModifier,
    EarlyGeometricToleranceRelationship, EarlyLeaderCurve, EarlyLeaderDirectedCallout,
    EarlyLeaderTerminator, EarlyLimitsAndFits, EarlyLineProfileToleranceComplex,
    EarlyMeasureQualification, EarlyParallelismTolerance, EarlyParallelismToleranceComplex,
    EarlyPerpendicularityTolerance, EarlyPerpendicularityToleranceComplex, EarlyPlusMinusTolerance,
    EarlyPositionToleranceComplex, EarlyProjectedZoneDefinition, EarlyRoundnessTolerance,
    EarlyRoundnessToleranceComplex, EarlyStraightnessTolerance, EarlyStraightnessToleranceComplex,
    EarlySurfaceProfileTolerance, EarlySurfaceProfileToleranceComplex, EarlySymmetryTolerance,
    EarlyTerminatorSymbol, EarlyTessellatedAnnotationOccurrence, EarlyToleranceValue,
    EarlyToleranceZoneForm, EarlyTotalRunoutTolerance, EarlyTypeQualifier,
    EarlyValueFormatTypeQualifier,
};
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::geometry::Point3;
use crate::ir::pmi::{
    AngularLocationData, AnnotationCurveOccurrence, AnnotationOccurrence,
    AnnotationOccurrenceAssociativity, AnnotationOccurrenceRef, AnnotationPlaceholderLeaderLine,
    AnnotationPlaceholderOccurrence, AnnotationPlaceholderOccurrenceWithLeaderLine,
    AnnotationPlane, AnnotationSymbolOccurrence, AnnotationTextOccurrence,
    AnnotationToModelLeaderLine, ApllPointData, ApllPointElement, ApllPointWithSurfaceData,
    AuxiliaryLeaderLineData, Datum, DatumFeature, DimensionalLocation, DimensionalLocationData,
    DimensionalSize, DimensionalSizeKind, DimensionalSizeWithDatumFeatureData,
    DraughtingAnnotationOccurrence, DraughtingCallout, DraughtingCalloutData,
    DraughtingCalloutRelationship, DraughtingModelIdentifiedItem, DraughtingModelItemAssociation,
    DraughtingModelItemDefinition, DraughtingPreDefinedTextFont, GeneralDatumBase,
    GeneralDatumReference, GeneralDatumReferenceData, GeometricTolerance,
    GeometricToleranceRelationship, GeometricToleranceWithDatumReference, LeaderCurve,
    LeaderTerminator, LimitsAndFits, MeasureQualification, PlainAnnotationCurveOccurrence,
    PlainAnnotationOccurrence, PlusMinusTolerance, PmiPool, ProjectedZoneDefinition,
    TerminatorSymbol, TessellatedAnnotationOccurrence, ToleranceValue, ToleranceZoneForm,
    TypeQualifier, ValueFormatTypeQualifier, ValueQualifier,
};
use crate::reader::ReaderContext;

/// Lower one `TOLERANCE_ZONE_FORM`.
pub(crate) fn lower_tolerance_zone_form(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyToleranceZoneForm,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .tolerance_zone_forms
        .push(ToleranceZoneForm { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TYPE_QUALIFIER`.
pub(crate) fn lower_type_qualifier(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTypeQualifier,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .type_qualifiers
        .push(TypeQualifier { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `VALUE_FORMAT_TYPE_QUALIFIER`.
pub(crate) fn lower_value_format_type_qualifier(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyValueFormatTypeQualifier,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .value_format_type_qualifiers
        .push(ValueFormatTypeQualifier {
            format_type: early.format_type,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Shared datum-free form-tolerance lowering: resolve magnitude (2-path) +
/// toleranced target, both unresolved = silent drop (symmetric on re-read).
/// The L1 OPTIONAL magnitude also drops on `$` (the legacy required read
/// errored there; no corpus file carries one).
#[allow(clippy::type_complexity)]
fn lower_form_tolerance_common(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
) -> Option<crate::ir::pmi::GeometricToleranceData> {
    let magnitude_ref = magnitude?;
    let magnitude = crate::entities::pmi::resolve_tolerance_magnitude(ctx, magnitude_ref)?;
    let toleranced_shape_aspect =
        crate::entities::pmi::resolve_geometric_tolerance_target(ctx, toleranced_shape_aspect)?;
    Some(crate::ir::pmi::GeometricToleranceData {
        name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: description.unwrap_or_default(),
        magnitude,
        toleranced_shape_aspect,
        modifiers: Vec::new(),
        unit_size: None,
        defined_area_unit: None,
    })
}

/// Lower one `FLATNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_flatness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyFlatnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Flatness(data),
    );
}

/// Lower one `SURFACE_PROFILE_TOLERANCE` (datum-free form).
pub(crate) fn lower_surface_profile_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySurfaceProfileTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::SurfaceProfile(data),
    );
}

/// Collapse the strict L1 `GEOMETRIC_TOLERANCE_WITH_MODIFIERS` enum (full 34-member
/// schema) to the reduced L2 [`crate::ir::GeometricToleranceModifier`] (3 named +
/// `Other(token)`). Inverse: [`super::super::lift::pmi::l2_modifier_to_early`].
pub(crate) fn early_modifier_to_l2(
    m: EarlyGeometricToleranceModifier,
) -> crate::ir::GeometricToleranceModifier {
    use crate::ir::GeometricToleranceModifier as L;
    use EarlyGeometricToleranceModifier as E;
    match m {
        E::MaximumMaterialRequirement => L::MaximumMaterialRequirement,
        E::LeastMaterialRequirement => L::LeastMaterialRequirement,
        E::ReciprocityRequirement => L::ReciprocityRequirement,
        E::StandardDeviation => L::Other("STANDARD_DEVIATION".into()),
        E::ValleyDepth => L::Other("VALLEY_DEPTH".into()),
        E::PeakHeight => L::Other("PEAK_HEIGHT".into()),
        E::TotalRangeDeviations => L::Other("TOTAL_RANGE_DEVIATIONS".into()),
        E::ReferenceMaximumInscribedFeature => {
            L::Other("REFERENCE_MAXIMUM_INSCRIBED_FEATURE".into())
        }
        E::ReferenceMinimumCircumscribedFeature => {
            L::Other("REFERENCE_MINIMUM_CIRCUMSCRIBED_FEATURE".into())
        }
        E::ReferenceLeastSquareFeatureWithInternalMaterialConstraint => {
            L::Other("REFERENCE_LEAST_SQUARE_FEATURE_WITH_INTERNAL_MATERIAL_CONSTRAINT".into())
        }
        E::ReferenceLeastSquareFeatureWithExternalMaterialConstraint => {
            L::Other("REFERENCE_LEAST_SQUARE_FEATURE_WITH_EXTERNAL_MATERIAL_CONSTRAINT".into())
        }
        E::ReferenceLeastSquareFeatureWithoutConstraint => {
            L::Other("REFERENCE_LEAST_SQUARE_FEATURE_WITHOUT_CONSTRAINT".into())
        }
        E::ReferenceMinimaxFeatureWithInternalMaterialConstraint => {
            L::Other("REFERENCE_MINIMAX_FEATURE_WITH_INTERNAL_MATERIAL_CONSTRAINT".into())
        }
        E::ReferenceMinimaxFeatureWithExternalMaterialConstraint => {
            L::Other("REFERENCE_MINIMAX_FEATURE_WITH_EXTERNAL_MATERIAL_CONSTRAINT".into())
        }
        E::ReferenceMinimaxFeatureWithoutConstraint => {
            L::Other("REFERENCE_MINIMAX_FEATURE_WITHOUT_CONSTRAINT".into())
        }
        E::AssociatedMaximumInscribedFeature => {
            L::Other("ASSOCIATED_MAXIMUM_INSCRIBED_FEATURE".into())
        }
        E::AssociatedTangentFeature => L::Other("ASSOCIATED_TANGENT_FEATURE".into()),
        E::AssociatedMinimumInscribedFeature => {
            L::Other("ASSOCIATED_MINIMUM_INSCRIBED_FEATURE".into())
        }
        E::AssociatedLeastSquareFeature => L::Other("ASSOCIATED_LEAST_SQUARE_FEATURE".into()),
        E::AssociatedMinmaxFeature => L::Other("ASSOCIATED_MINMAX_FEATURE".into()),
        E::UnitedFeature => L::Other("UNITED_FEATURE".into()),
        E::SeparateRequirement => L::Other("SEPARATE_REQUIREMENT".into()),
        E::EachRadialElement => L::Other("EACH_RADIAL_ELEMENT".into()),
        E::TangentPlane => L::Other("TANGENT_PLANE".into()),
        E::StatisticalTolerance => L::Other("STATISTICAL_TOLERANCE".into()),
        E::NotConvex => L::Other("NOT_CONVEX".into()),
        E::LineElement => L::Other("LINE_ELEMENT".into()),
        E::PitchDiameter => L::Other("PITCH_DIAMETER".into()),
        E::MajorDiameter => L::Other("MAJOR_DIAMETER".into()),
        E::MinorDiameter => L::Other("MINOR_DIAMETER".into()),
        E::CommonZone => L::Other("COMMON_ZONE".into()),
        E::FreeState => L::Other("FREE_STATE".into()),
        E::AnyCrossSection => L::Other("ANY_CROSS_SECTION".into()),
        E::CircleA => L::Other("CIRCLE_A".into()),
    }
}

/// Collapse the strict L1 `area_unit_type` enum (5 schema members) to the reduced
/// L2 [`crate::ir::AreaUnitType`] (circular/rectangular/square + `Other`).
pub(crate) fn early_area_unit_to_l2(a: EarlyAreaUnitType) -> crate::ir::AreaUnitType {
    use crate::ir::AreaUnitType as L;
    match a {
        EarlyAreaUnitType::Circular => L::Circular,
        EarlyAreaUnitType::Rectangular => L::Rectangular,
        EarlyAreaUnitType::Square => L::Square,
        EarlyAreaUnitType::Spherical => L::Other("SPHERICAL".into()),
        EarlyAreaUnitType::Cylindrical => L::Other("CYLINDRICAL".into()),
    }
}

/// Shared builder for the datum-free GD&T COMPLEX forms: resolve magnitude/target
/// (drop-on-None), collapse the strict L1 modifier/area enums to L2, and apply the
/// WDAU⊂WDU cascade (area dropped if the `unit_size` ref did not resolve — mirrors
/// the legacy hand reader).
#[allow(clippy::too_many_arguments)]
fn lower_gt_complex_data(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
    modifiers: Vec<EarlyGeometricToleranceModifier>,
    unit_size: Option<u64>,
    defined_area_unit: Option<(EarlyAreaUnitType, Option<u64>)>,
) -> Option<crate::ir::pmi::GeometricToleranceData> {
    let magnitude = crate::entities::pmi::resolve_tolerance_magnitude(ctx, magnitude?)?;
    let toleranced_shape_aspect =
        crate::entities::pmi::resolve_geometric_tolerance_target(ctx, toleranced_shape_aspect)?;
    let unit_size =
        unit_size.and_then(|r| crate::entities::pmi::resolve_tolerance_magnitude(ctx, r));
    // WDAU cascades from WDU: drop area when the unit ref did not resolve.
    let defined_area_unit = if unit_size.is_some() {
        defined_area_unit.map(|(area_type, second)| crate::ir::DefinedAreaUnit {
            area_type: early_area_unit_to_l2(area_type),
            second_unit_size: second
                .and_then(|r| crate::entities::pmi::resolve_tolerance_magnitude(ctx, r)),
        })
    } else {
        None
    };
    Some(crate::ir::pmi::GeometricToleranceData {
        name,
        description: description.unwrap_or_default(),
        magnitude,
        toleranced_shape_aspect,
        modifiers: modifiers.into_iter().map(early_modifier_to_l2).collect(),
        unit_size,
        defined_area_unit,
    })
}

/// Lower one `FLATNESS_TOLERANCE` COMPLEX form (multi-case: modifiers / unit /
/// unit+area). Maps the case variant to the shared builder.
pub(crate) fn lower_flatness_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyFlatnessToleranceComplex,
) {
    let data = match early {
        EarlyFlatnessToleranceComplex::Modifiers(c) => lower_gt_complex_data(
            ctx,
            c.name,
            c.description,
            c.magnitude,
            c.toleranced_shape_aspect,
            c.modifiers,
            None,
            None,
        ),
        EarlyFlatnessToleranceComplex::Unit(c) => lower_gt_complex_data(
            ctx,
            c.name,
            c.description,
            c.magnitude,
            c.toleranced_shape_aspect,
            Vec::new(),
            Some(c.unit_size),
            None,
        ),
        EarlyFlatnessToleranceComplex::UnitArea(c) => lower_gt_complex_data(
            ctx,
            c.name,
            c.description,
            c.magnitude,
            c.toleranced_shape_aspect,
            Vec::new(),
            Some(c.unit_size),
            Some((c.area_type, c.second_unit_size)),
        ),
    };
    let Some(data) = data else { return };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Flatness(data),
    );
}

/// Lower one `ROUNDNESS_TOLERANCE` COMPLEX form (single case: modifiers).
pub(crate) fn lower_roundness_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRoundnessToleranceComplex,
) {
    let Some(data) = lower_gt_complex_data(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        early.modifiers,
        None,
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Roundness(data),
    );
}

/// Lower one `STRAIGHTNESS_TOLERANCE` COMPLEX form (single case: unit).
pub(crate) fn lower_straightness_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyStraightnessToleranceComplex,
) {
    let Some(data) = lower_gt_complex_data(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        Vec::new(),
        Some(early.unit_size),
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Straightness(data),
    );
}

/// Shared builder for the WDR simple-leaf COMPLEX forms (`PARALLELISM` /
/// `PERPENDICULARITY` / `CIRCULAR_RUNOUT`): collapse the strict L1 modifier enum
/// to L2, then resolve via the shared with-datum builder (no displacement — these
/// cases carry only `GEOMETRIC_TOLERANCE_WITH_MODIFIERS`).
#[allow(clippy::too_many_arguments)]
fn lower_gtwdr_complex_common(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
    datum_system: &[u64],
    modifiers: Vec<EarlyGeometricToleranceModifier>,
    displacement: Option<u64>,
) -> Option<crate::ir::pmi::GeometricToleranceWithDatumReferenceData> {
    let magnitude = magnitude?;
    let displacement =
        displacement.and_then(|r| crate::entities::pmi::resolve_tolerance_magnitude(ctx, r));
    crate::entities::pmi::build_gt_with_datum_reference_data(
        ctx,
        name,
        description.unwrap_or_default(),
        magnitude,
        toleranced_shape_aspect,
        datum_system,
        modifiers.into_iter().map(early_modifier_to_l2).collect(),
        displacement,
    )
}

/// Lower one `PARALLELISM_TOLERANCE` COMPLEX form (WDR + modifiers).
pub(crate) fn lower_parallelism_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyParallelismToleranceComplex,
) {
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        early.modifiers,
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::Parallelism(data),
    );
}

/// Lower one `PERPENDICULARITY_TOLERANCE` COMPLEX form (WDR + modifiers).
pub(crate) fn lower_perpendicularity_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPerpendicularityToleranceComplex,
) {
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        early.modifiers,
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::Perpendicularity(data),
    );
}

/// Lower one `CIRCULAR_RUNOUT_TOLERANCE` COMPLEX form (WDR + modifiers).
pub(crate) fn lower_circular_runout_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCircularRunoutToleranceComplex,
) {
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        early.modifiers,
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::CircularRunout(data),
    );
}

/// Lower one `POSITION_TOLERANCE` COMPLEX form (always-complex; plain or modifiers).
pub(crate) fn lower_position_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPositionToleranceComplex,
) {
    let (name, description, magnitude, tsa, datum_system, modifiers) = match early {
        EarlyPositionToleranceComplex::Plain(p) => (
            p.name,
            p.description,
            p.magnitude,
            p.toleranced_shape_aspect,
            p.datum_system,
            Vec::new(),
        ),
        EarlyPositionToleranceComplex::Modifiers(m) => (
            m.name,
            m.description,
            m.magnitude,
            m.toleranced_shape_aspect,
            m.datum_system,
            m.modifiers,
        ),
    };
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        name,
        description,
        magnitude,
        tsa,
        &datum_system,
        modifiers,
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::Position(data),
    );
}

/// Lower one `SURFACE_PROFILE_TOLERANCE` COMPLEX form (plain / modifiers /
/// displacement).
pub(crate) fn lower_surface_profile_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySurfaceProfileToleranceComplex,
) {
    let (name, description, magnitude, tsa, datum_system, modifiers, displacement) = match early {
        EarlySurfaceProfileToleranceComplex::Plain(p) => (
            p.name,
            p.description,
            p.magnitude,
            p.toleranced_shape_aspect,
            p.datum_system,
            Vec::new(),
            None,
        ),
        EarlySurfaceProfileToleranceComplex::Modifiers(m) => (
            m.name,
            m.description,
            m.magnitude,
            m.toleranced_shape_aspect,
            m.datum_system,
            m.modifiers,
            None,
        ),
        EarlySurfaceProfileToleranceComplex::Displacement(d) => (
            d.name,
            d.description,
            d.magnitude,
            d.toleranced_shape_aspect,
            d.datum_system,
            Vec::new(),
            Some(d.displacement),
        ),
    };
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        name,
        description,
        magnitude,
        tsa,
        &datum_system,
        modifiers,
        displacement,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::SurfaceProfile(data),
    );
}

/// Lower one `LINE_PROFILE_TOLERANCE` COMPLEX form (single case: plain).
pub(crate) fn lower_line_profile_tolerance_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyLineProfileToleranceComplex,
) {
    let Some(data) = lower_gtwdr_complex_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        Vec::new(),
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(
        ctx,
        entity_id,
        GeometricToleranceWithDatumReference::LineProfile(data),
    );
}

/// Shared lowering for the simple `GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE`
/// subtypes (`angularity` / `circular_runout` / `concentricity` / `parallelism`
/// / `perpendicularity` / `symmetry` / `total_runout`): resolve `magnitude` /
/// `shape_aspect` / `datum_system` via the shared builder, then push under
/// `kind`. A `$` magnitude (corpus-absent — the legacy read required it) drops
/// the entity.
#[allow(clippy::too_many_arguments)]
fn lower_gt_with_datum_reference(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
    datum_system: &[u64],
    kind: fn(
        crate::ir::pmi::GeometricToleranceWithDatumReferenceData,
    ) -> GeometricToleranceWithDatumReference,
) {
    let Some(mag) = magnitude else {
        return;
    };
    let Some(data) = crate::entities::pmi::build_gt_with_datum_reference_data(
        ctx,
        name,
        description.unwrap_or_default(),
        mag,
        toleranced_shape_aspect,
        datum_system,
        Vec::new(),
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(ctx, entity_id, kind(data));
}

/// Lower one `ANGULARITY_TOLERANCE`.
pub(crate) fn lower_angularity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAngularityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Angularity,
    );
}

/// Lower one `CIRCULAR_RUNOUT_TOLERANCE`.
pub(crate) fn lower_circular_runout_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCircularRunoutTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::CircularRunout,
    );
}

/// Lower one `CONCENTRICITY_TOLERANCE`.
pub(crate) fn lower_concentricity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyConcentricityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Concentricity,
    );
}

/// Lower one `PARALLELISM_TOLERANCE`.
pub(crate) fn lower_parallelism_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyParallelismTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Parallelism,
    );
}

/// Lower one `PERPENDICULARITY_TOLERANCE`.
pub(crate) fn lower_perpendicularity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPerpendicularityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Perpendicularity,
    );
}

/// Lower one `SYMMETRY_TOLERANCE`.
pub(crate) fn lower_symmetry_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySymmetryTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Symmetry,
    );
}

/// Lower one `TOTAL_RUNOUT_TOLERANCE`.
pub(crate) fn lower_total_runout_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTotalRunoutTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::TotalRunout,
    );
}

/// Lower one `STRAIGHTNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_straightness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyStraightnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Straightness(data),
    );
}

/// Lower one `ROUNDNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_roundness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRoundnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Roundness(data),
    );
}

/// Lower one `CYLINDRICITY_TOLERANCE` (datum-free form).
pub(crate) fn lower_cylindricity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCylindricityTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Cylindricity(data),
    );
}

/// L1 LOGICAL → the legacy `read_bool`'s `bool` (`.U.` drops).
fn logical_to_bool(l: crate::ir::geometry::Logical) -> Option<bool> {
    match l {
        crate::ir::geometry::Logical::True => Some(true),
        crate::ir::geometry::Logical::False => Some(false),
        crate::ir::geometry::Logical::Unknown => None,
    }
}

/// Lower one `DATUM` (unresolved `of_shape` product / `.U.` = silent drop).
pub(crate) fn lower_datum(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDatum) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .datums
        .push(Datum {
            name: early.name,
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: early.description.unwrap_or_default(),
            target,
            product_definitional,
            identification: early.identification,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Resolve the shared `general_datum_reference` body (the `shape_aspect` 4-attr
/// supertype body + `base` SELECT) into a [`GeneralDatumReferenceData`].
/// `None` drops the entry (symmetric on re-read) when `of_shape` is not a
/// product, the `base` datum/member does not resolve, or `product_definitional`
/// is `.U.`. The non-standard `of_shape = $` case is guarded in the handler
/// before `bind`; `modifiers` is not modelled and is ignored.
#[allow(clippy::too_many_arguments)]
fn lower_general_datum_reference_data(
    ctx: &mut ReaderContext,
    name: String,
    description: Option<String>,
    of_shape: u64,
    product_definitional: crate::ir::geometry::Logical,
    base: EarlyDatumOrCommonDatum,
    graph: &crate::parser::entity::EntityGraph,
    entity_name: &'static str,
) -> Option<GeneralDatumReferenceData> {
    let product_definitional = logical_to_bool(product_definitional)?;
    // of_shape -> PRODUCT_DEFINITION_SHAPE -> ProductId (typed one-probe).
    let target = ctx.product_of_pds(of_shape)?;
    // base — `datum_or_common_datum` SELECT: a single `DATUM`, or a
    // `COMMON_DATUM_LIST` (a list of `datum_reference_element`s). A member
    // outside these resolutions drops the owner.
    let base = match base {
        EarlyDatumOrCommonDatum::EntityRef(r) => {
            GeneralDatumBase::Datum(ctx.id_cache.get::<crate::ir::id::DatumId>(r)?)
        }
        EarlyDatumOrCommonDatum::CommonDatumList(refs) => {
            let mut ids = Vec::with_capacity(refs.len());
            for r in refs {
                let Some(gdr_id) = ctx
                    .id_cache
                    .get::<crate::ir::id::GeneralDatumReferenceId>(r)
                else {
                    // The member dropped. Record the cascade only when it is an
                    // of_shape=$ sibling (NsCase::GeneralDatumReferenceOfShapeUnset);
                    // a member dropped for a different reason stays a plain drop.
                    let member_of_shape_unset = matches!(
                        graph.get(r),
                        Some(crate::parser::entity::RawEntity::Simple { attributes, .. })
                            if matches!(
                                attributes.get(2),
                                Some(crate::parser::entity::Attribute::Unset
                                    | crate::parser::entity::Attribute::Derived)
                            )
                    );
                    if member_of_shape_unset {
                        ctx.ns_record(
                            crate::reader::NsCase::GeneralDatumReferenceOfShapeUnset,
                            entity_name.into(),
                            "dropped (common_datum_list member of_shape Unset — cascade)",
                        );
                    }
                    return None;
                };
                ids.push(gdr_id);
            }
            GeneralDatumBase::CommonDatumList(ids)
        }
    };
    Some(GeneralDatumReferenceData {
        name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: description.unwrap_or_default(),
        target,
        product_definitional,
        base,
    })
}

/// Lower one `DATUM_REFERENCE_COMPARTMENT`.
pub(crate) fn lower_datum_reference_compartment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDatumReferenceCompartment,
    graph: &crate::parser::entity::EntityGraph,
) {
    let Some(data) = lower_general_datum_reference_data(
        ctx,
        early.name,
        early.description,
        early.of_shape,
        early.product_definitional,
        early.base,
        graph,
        "DATUM_REFERENCE_COMPARTMENT",
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .general_datum_references
        .push(GeneralDatumReference::Compartment(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DATUM_REFERENCE_ELEMENT`.
pub(crate) fn lower_datum_reference_element(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDatumReferenceElement,
    graph: &crate::parser::entity::EntityGraph,
) {
    let Some(data) = lower_general_datum_reference_data(
        ctx,
        early.name,
        early.description,
        early.of_shape,
        early.product_definitional,
        early.base,
        graph,
        "DATUM_REFERENCE_ELEMENT",
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .general_datum_references
        .push(GeneralDatumReference::Element(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DIMENSIONAL_SIZE` (unresolved `applies_to` = silent drop).
pub(crate) fn lower_dimensional_size(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDimensionalSize,
) {
    let Some(applies_to) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.applies_to,
        )
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_sizes
        .push(DimensionalSize {
            applies_to,
            name: early.name,
            kind: DimensionalSizeKind::Plain,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `GEOMETRIC_TOLERANCE_RELATIONSHIP` (either end unresolved =
/// silent drop).
pub(crate) fn lower_geometric_tolerance_relationship(
    ctx: &mut ReaderContext,
    early: EarlyGeometricToleranceRelationship,
) {
    let Some(relating) = crate::entities::pmi::resolve_geometric_tolerance_ref(
        ctx,
        early.relating_geometric_tolerance,
    ) else {
        return;
    };
    let Some(related) = crate::entities::pmi::resolve_geometric_tolerance_ref(
        ctx,
        early.related_geometric_tolerance,
    ) else {
        return;
    };
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .geometric_tolerance_relationships
        .push(GeometricToleranceRelationship {
            name: early.name,
            description: early.description,
            relating,
            related,
        });
}

/// Lower one `MEASURE_QUALIFICATION` (unresolved measure = silent drop;
/// unresolved qualifier members skip individually).
pub(crate) fn lower_measure_qualification(
    ctx: &mut ReaderContext,
    early: EarlyMeasureQualification,
) {
    let Some(qualified_measure) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.qualified_measure)
    else {
        return;
    };
    let mut qualifiers = Vec::with_capacity(early.qualifiers.len());
    for r in early.qualifiers {
        if let Some(q) = ValueQualifier::resolve_select(ctx, r) {
            qualifiers.push(q);
        }
    }
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .measure_qualifications
        .push(MeasureQualification {
            name: early.name,
            description: early.description,
            qualified_measure,
            qualifiers,
        });
}

/// Shared `DIMENSIONAL_LOCATION` endpoint resolve (either unresolved =
/// silent drop) + the `$`→"" description collapse.
fn lower_dl_common(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    relating: u64,
    related: u64,
) -> Option<DimensionalLocationData> {
    let relating_shape_aspect =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx, relating,
        )?;
    let related_shape_aspect =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx, related,
        )?;
    Some(DimensionalLocationData {
        name,
        description: description.unwrap_or_default(),
        relating_shape_aspect,
        related_shape_aspect,
    })
}

/// Lower one plain `DIMENSIONAL_LOCATION`.
pub(crate) fn lower_dimensional_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDimensionalLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Plain(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DIRECTED_DIMENSIONAL_LOCATION` (same 4-attr body).
pub(crate) fn lower_directed_dimensional_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDirectedDimensionalLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Directed(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANGULAR_LOCATION` (adds the `angle_relator` enum, converted
/// by the bind's enum hint — unknown tokens error there, like the legacy
/// read).
pub(crate) fn lower_angular_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAngularLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Angular(AngularLocationData {
            name: data.name,
            description: data.description,
            relating_shape_aspect: data.relating_shape_aspect,
            related_shape_aspect: data.related_shape_aspect,
            angle_selection: early.angle_selection,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANGULAR_SIZE` (`DIMENSIONAL_SIZE` plus `angle_relator`).
pub(crate) fn lower_angular_size(ctx: &mut ReaderContext, entity_id: u64, early: EarlyAngularSize) {
    let Some(applies_to) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.applies_to,
        )
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_sizes
        .push(DimensionalSize {
            applies_to,
            name: early.name,
            kind: DimensionalSizeKind::Angular(early.angle_selection),
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DATUM_FEATURE` (`Itself` variant of the family; the
/// complex DSWDF keeps its own handler).
pub(crate) fn lower_datum_feature(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDatumFeature,
) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .datum_features
        .push(DatumFeature::Itself(crate::ir::DatumFeatureData {
            name: early.name,
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: early.description.unwrap_or_default(),
            target,
            product_definitional,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`. Multiple-inheritance
/// (`datum_feature` + `dimensional_size`): `applies_to` is the EXPRESS WR1
/// self-reference, so push a placeholder, register the id, then resolve and
/// backpatch (the id only exists after the push).
pub(crate) fn lower_dimensional_size_with_datum_feature(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDimensionalSizeWithDatumFeature,
) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let df_id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .datum_features
        .push(DatumFeature::DimensionalSizeWithDatumFeature(
            DimensionalSizeWithDatumFeatureData {
                base: crate::ir::DatumFeatureData {
                    name: early.name,
                    // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
                    description: early.description.unwrap_or_default(),
                    target,
                    product_definitional,
                },
                // placeholder, overwritten below once the id is known
                applies_to: crate::ir::ShapeAspectRef::DatumFeature(crate::ir::DatumFeatureId(0)),
                size_name: early.name_2,
            },
        ));
    ctx.id_cache.insert(entity_id, df_id);
    let applies_to =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.applies_to,
        )
        .unwrap_or(crate::ir::ShapeAspectRef::DatumFeature(df_id));
    if let DatumFeature::DimensionalSizeWithDatumFeature(d) =
        &mut ctx.pmi.get_or_insert_with(PmiPool::default).datum_features[df_id]
    {
        d.applies_to = applies_to;
    }
}

/// Lower one plain `DRAUGHTING_CALLOUT` (unresolved content members skip
/// individually via the shared contents resolver).
pub(crate) fn lower_draughting_callout(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingCallout,
) {
    let contents = crate::entities::pmi::read_draughting_callout_contents(ctx, &early.contents);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_callouts
        .push(DraughtingCallout::Plain(DraughtingCalloutData {
            name: early.name,
            contents,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `LEADER_DIRECTED_CALLOUT` (same body, `LeaderDirected` variant).
pub(crate) fn lower_leader_directed_callout(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyLeaderDirectedCallout,
) {
    let contents = crate::entities::pmi::read_draughting_callout_contents(ctx, &early.contents);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_callouts
        .push(DraughtingCallout::LeaderDirected(DraughtingCalloutData {
            name: early.name,
            contents,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `ANNOTATION_TEXT_OCCURRENCE` complex. `styles` keeps only
/// refs resolving to a `PresentationStyleAssignment` (others skipped); `item`
/// resolves through `resolve_representation_item_ref` — an unresolved item drops
/// the whole occurrence (symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_text_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationTextOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationTextOccurrence(
            AnnotationTextOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `LEADER_CURVE` complex. `styles` keeps only resolved
/// `PresentationStyleAssignment` refs; `item` must resolve to a `CurveId`
/// (unresolved drops the occurrence). Verbatim port of the legacy read.
pub(crate) fn lower_leader_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLeaderCurve,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = ctx.id_cache.get::<crate::ir::id::CurveId>(early.item) else {
        return; // item unresolved — drop the occurrence
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_curve_occurrences
        .push(AnnotationCurveOccurrence::LeaderCurve(LeaderCurve {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `LEADER_TERMINATOR` complex. Like `LEADER_CURVE` but `item`
/// resolves through `resolve_representation_item_ref` and `annotated_curve` must
/// resolve to an `AnnotationCurveOccurrenceId` (a `LEADER_CURVE`); either
/// unresolved drops the occurrence. Verbatim port of the legacy read.
pub(crate) fn lower_leader_terminator(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLeaderTerminator,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let Some(annotated_curve) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationCurveOccurrenceId>(early.annotated_curve)
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::LeaderTerminator(LeaderTerminator {
            name: early.name.clone(),
            styles,
            item,
            annotated_curve,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the plain `ANNOTATION_OCCURRENCE` supertype. `styles` keeps only
/// resolved `PresentationStyleAssignment` refs; `item` resolves through
/// `resolve_representation_item_ref` (unresolved drops the occurrence,
/// symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::Plain(PlainAnnotationOccurrence {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `DRAUGHTING_ANNOTATION_OCCURRENCE` (same shape as the supertype).
/// Verbatim port of the legacy read.
pub(crate) fn lower_draughting_annotation_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDraughtingAnnotationOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::DraughtingAnnotationOccurrence(
            DraughtingAnnotationOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `ANNOTATION_SYMBOL_OCCURRENCE` (same shape as the supertype).
/// Verbatim port of the legacy read.
pub(crate) fn lower_annotation_symbol_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationSymbolOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationSymbolOccurrence(
            AnnotationSymbolOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the plain `ANNOTATION_CURVE_OCCURRENCE` supertype (not `LEADER_CURVE`).
/// `item` resolves through `resolve_representation_item_ref` (a plain `CURVE` or
/// e.g. `GEOMETRIC_CURVE_SET`); unresolved drops the occurrence. Verbatim port
/// of the legacy read.
pub(crate) fn lower_annotation_curve_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationCurveOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_curve_occurrences
        .push(AnnotationCurveOccurrence::Plain(
            PlainAnnotationCurveOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `ANNOTATION_PLANE`. Same styled shape as the other occurrences; the
/// 4th attr `elements` (an `annotation_plane_element` list) is not modelled, so
/// `early.elements` is ignored here and re-emitted as `$` on write. `item`
/// resolves through `resolve_representation_item_ref` (unresolved drops the
/// occurrence, symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_plane(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationPlane,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_ANNOTATION_OCCURRENCE`. `styles` filter through the
/// PSA id cache; an unresolved `item` (`tessellated_item`) drops the occurrence.
pub(crate) fn lower_tessellated_annotation_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedAnnotationOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(early.item)
    else {
        return; // item unresolved — drop the occurrence
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::TessellatedAnnotationOccurrence(
            TessellatedAnnotationOccurrence {
                name: early.name,
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `LIMITS_AND_FITS` (four scalar tolerance grades; no refs).
pub(crate) fn lower_limits_and_fits(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyLimitsAndFits,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .limits_and_fits
        .push(LimitsAndFits {
            form_variance: early.form_variance,
            zone_variance: early.zone_variance,
            grade: early.grade,
            source: early.source,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DRAUGHTING_PRE_DEFINED_TEXT_FONT` (single `name` scalar).
pub(crate) fn lower_draughting_pre_defined_text_font(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingPreDefinedTextFont,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_pre_defined_text_fonts
        .push(DraughtingPreDefinedTextFont { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DRAUGHTING_CALLOUT_RELATIONSHIP` (either `draughting_callout`
/// endpoint unresolved = silent drop, symmetric on re-read).
pub(crate) fn lower_draughting_callout_relationship(
    ctx: &mut ReaderContext,
    early: EarlyDraughtingCalloutRelationship,
) {
    let Some(relating) = ctx
        .id_cache
        .get::<crate::ir::id::DraughtingCalloutId>(early.relating_draughting_callout)
    else {
        return;
    };
    let Some(related) = ctx
        .id_cache
        .get::<crate::ir::id::DraughtingCalloutId>(early.related_draughting_callout)
    else {
        return;
    };
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_callout_relationships
        .push(DraughtingCalloutRelationship {
            name: early.name,
            description: early.description,
            relating,
            related,
        });
}

/// Lower one `TOLERANCE_VALUE` (both bounds resolve through
/// `resolve_tolerance_magnitude` — units-pool or repr-item; either unresolved
/// drops the value, symmetric on re-read).
pub(crate) fn lower_tolerance_value(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyToleranceValue,
) {
    let Some(lower_bound) =
        crate::entities::pmi::resolve_tolerance_magnitude(ctx, early.lower_bound)
    else {
        return;
    };
    let Some(upper_bound) =
        crate::entities::pmi::resolve_tolerance_magnitude(ctx, early.upper_bound)
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .tolerance_values
        .push(ToleranceValue {
            lower_bound,
            upper_bound,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PLUS_MINUS_TOLERANCE` (`range` via
/// `resolve_tolerance_method_definition`, `toleranced_dimension` via
/// `resolve_dimensional_characteristic`; either unresolved drops the entity).
pub(crate) fn lower_plus_minus_tolerance(ctx: &mut ReaderContext, early: EarlyPlusMinusTolerance) {
    let Some(range) = crate::entities::pmi::resolve_tolerance_method_definition(ctx, early.range)
    else {
        return;
    };
    let Some(toleranced_dimension) =
        crate::entities::pmi::resolve_dimensional_characteristic(ctx, early.toleranced_dimension)
    else {
        return;
    };
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .plus_minus_tolerances
        .push(PlusMinusTolerance {
            range,
            toleranced_dimension,
        });
}

/// Lower one `PROJECTED_ZONE_DEFINITION` (`zone` via `tolerance_zone` cache,
/// `projection_end` via `resolve_shape_aspect_ref`, `projected_length` via
/// `resolve_tolerance_magnitude`; any required ref unresolved drops the entity.
/// Individual `boundaries` refs skip silently).
pub(crate) fn lower_projected_zone_definition(
    ctx: &mut ReaderContext,
    early: EarlyProjectedZoneDefinition,
) {
    let Some(zone) = ctx
        .id_cache
        .get::<crate::ir::id::ToleranceZoneId>(early.zone)
    else {
        return;
    };
    let Some(projection_end) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.projection_end,
        )
    else {
        return;
    };
    let Some(projected_length) =
        crate::entities::pmi::resolve_tolerance_magnitude(ctx, early.projected_length)
    else {
        return;
    };
    let mut boundaries = Vec::with_capacity(early.boundaries.len());
    for r in early.boundaries {
        if let Some(sar) =
            crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(ctx, r)
        {
            boundaries.push(sar);
        }
    }
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .tolerance_zone_definitions
        .push(ProjectedZoneDefinition {
            zone,
            boundaries,
            projection_end,
            projected_length,
        });
}

/// Lower one `TERMINATOR_SYMBOL` (`annotation_symbol_occurrence` subtype).
/// `styles` keeps only resolved `PresentationStyleAssignment` refs; `item`
/// resolves through `resolve_representation_item_ref` and `annotated_curve`
/// must resolve to an `AnnotationCurveOccurrenceId`; either unresolved drops
/// the occurrence, symmetric on re-read.
pub(crate) fn lower_terminator_symbol(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTerminatorSymbol,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let Some(annotated_curve) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationCurveOccurrenceId>(early.annotated_curve)
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::TerminatorSymbol(TerminatorSymbol {
            name: early.name,
            styles,
            item,
            annotated_curve,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Resolve a styled occurrence's `styles` list, keeping only refs resolving to a
/// `PresentationStyleAssignment` (others skipped) — shared by the placeholder
/// occurrences.
fn resolve_psa_styles(
    ctx: &ReaderContext,
    refs: &[u64],
) -> Vec<crate::ir::id::PresentationStyleAssignmentId> {
    let mut styles = Vec::with_capacity(refs.len());
    for &r in refs {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    styles
}

/// Lower one `ANNOTATION_PLACEHOLDER_OCCURRENCE`. `styles` filter through the
/// PSA cache; an unresolved `item` (`resolve_representation_item_ref`) drops the
/// occurrence (symmetric on re-read). `role` is the bind-decoded enum.
pub(crate) fn lower_annotation_placeholder_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAnnotationPlaceholderOccurrence,
) {
    let styles = resolve_psa_styles(ctx, &early.styles);
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlaceholderOccurrence(
            AnnotationPlaceholderOccurrence {
                name: early.name,
                styles,
                item,
                role: early.role,
                line_spacing: early.line_spacing,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE` (base APO +
/// `leader_line` SET; individual unresolved leader refs skip).
pub(crate) fn lower_annotation_placeholder_occurrence_with_leader_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAnnotationPlaceholderOccurrenceWithLeaderLine,
) {
    let styles = resolve_psa_styles(ctx, &early.styles);
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let mut leader_line = Vec::with_capacity(early.leader_line.len());
    for &r in &early.leader_line {
        if let Some(id) = ctx
            .id_cache
            .get::<crate::ir::id::AnnotationPlaceholderLeaderLineId>(r)
        {
            leader_line.push(id);
        }
    }
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(
            AnnotationOccurrence::AnnotationPlaceholderOccurrenceWithLeaderLine(
                AnnotationPlaceholderOccurrenceWithLeaderLine {
                    name: early.name,
                    styles,
                    item,
                    role: early.role,
                    line_spacing: early.line_spacing,
                    leader_line,
                },
            ),
        );
    ctx.id_cache.insert(entity_id, id);
}

/// Resolve a leader line's `geometric_elements` list, keeping only refs
/// resolving to an `ApllPoint` (others skipped) — shared by both leader lines.
fn resolve_apll_point_elements(
    ctx: &ReaderContext,
    refs: &[u64],
) -> Vec<crate::ir::id::ApllPointId> {
    let mut elems = Vec::with_capacity(refs.len());
    for &r in refs {
        if let Some(id) = ctx.id_cache.get::<crate::ir::id::ApllPointId>(r) {
            elems.push(id);
        }
    }
    elems
}

/// Lower one `ANNOTATION_TO_MODEL_LEADER_LINE` into the shared
/// `annotation_placeholder_leader_lines` arena.
pub(crate) fn lower_annotation_to_model_leader_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAnnotationToModelLeaderLine,
) {
    let geometric_elements = resolve_apll_point_elements(ctx, &early.geometric_elements);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_placeholder_leader_lines
        .push(
            AnnotationPlaceholderLeaderLine::AnnotationToModelLeaderLine(
                AnnotationToModelLeaderLine {
                    name: early.name,
                    geometric_elements,
                },
            ),
        );
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APLL_POINT` (a `cartesian_point` subtype waypoint). A non-3D
/// coordinate list is malformed input — warns + drops (the legacy read errored;
/// no corpus file carries one). `symbol_applied` is the bind-decoded enum.
pub(crate) fn lower_apll_point(ctx: &mut ReaderContext, entity_id: u64, early: EarlyApllPoint) {
    let Some(coordinates) = point3_or_warn(ctx, entity_id, &early.coordinates, "APLL_POINT") else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .apll_points
        .push(ApllPointElement::ApllPoint(ApllPointData {
            name: early.name,
            coordinates,
            symbol_applied: early.symbol_applied,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `APLL_POINT_WITH_SURFACE` (base + the `face_surface` it projects
/// onto). Non-3D coords warn + drop; an unresolved `associated_surface` warns +
/// drops (verbatim port of the legacy read).
pub(crate) fn lower_apll_point_with_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyApllPointWithSurface,
) {
    let Some(coordinates) = point3_or_warn(
        ctx,
        entity_id,
        &early.coordinates,
        "APLL_POINT_WITH_SURFACE",
    ) else {
        return;
    };
    let Some(associated_surface) = ctx
        .id_cache
        .get::<crate::ir::id::FaceId>(early.associated_surface)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "APLL_POINT_WITH_SURFACE.associated_surface #{} did not resolve to a known \
                 face_surface",
                    early.associated_surface
                ),
            });
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .apll_points
        .push(ApllPointElement::ApllPointWithSurface(
            ApllPointWithSurfaceData {
                name: early.name,
                coordinates,
                symbol_applied: early.symbol_applied,
                associated_surface,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// A 3-element coordinate list → [`Point3`]; a non-3D list is malformed input
/// (warns + returns `None` so the caller drops). Shared by both APLL points.
fn point3_or_warn(
    ctx: &mut ReaderContext,
    entity_id: u64,
    coords: &[f64],
    name: &str,
) -> Option<Point3> {
    if coords.len() != 3 {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("{name} must have 3 coordinates, got {}", coords.len()),
            });
        return None;
    }
    Some(Point3 {
        x: coords[0],
        y: coords[1],
        z: coords[2],
    })
}

/// Lower one `AUXILIARY_LEADER_LINE` (base + `controlling_leader_line`, another
/// arena member). An unresolved `controlling_leader_line` warns + drops
/// (symmetric on re-read), a verbatim port of the legacy read.
pub(crate) fn lower_auxiliary_leader_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAuxiliaryLeaderLine,
) {
    let geometric_elements = resolve_apll_point_elements(ctx, &early.geometric_elements);
    let Some(controlling_leader_line) =
        ctx.id_cache
            .get::<crate::ir::id::AnnotationPlaceholderLeaderLineId>(early.controlling_leader_line)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "AUXILIARY_LEADER_LINE.controlling_leader_line #{} did not resolve to a known \
                 leader line",
                    early.controlling_leader_line
                ),
            });
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_placeholder_leader_lines
        .push(AnnotationPlaceholderLeaderLine::AuxiliaryLeaderLine(
            AuxiliaryLeaderLineData {
                name: early.name,
                geometric_elements,
                controlling_leader_line,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Shared DMIA base lowering: resolve `definition`(6-way SELECT probe),
/// `used_representation`(`RepresentationId`), `identified_item`(SELECT). Verbatim
/// port of `read_dmia_base`. `annotation_placeholder` defaults `None` (the
/// `_WITH_PLACEHOLDER` lower sets it). A 6-member-miss `definition` warns +
/// drops; an unresolved `used_representation`/`identified_item` silently drops.
fn lower_dmia_common(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    description: Option<String>,
    definition_ref: u64,
    used_ref: u64,
    identified_ref: u64,
) -> Option<DraughtingModelItemAssociation> {
    let definition = if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(definition_ref)
    {
        DraughtingModelItemDefinition::Representation(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::DimensionalSizeId>(definition_ref)
    {
        DraughtingModelItemDefinition::DimensionalSize(id)
    } else if let Some(sa_ref) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            definition_ref,
        )
    {
        DraughtingModelItemDefinition::ShapeAspect(sa_ref)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(definition_ref)
    {
        DraughtingModelItemDefinition::PropertyDefinition(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::DimensionalLocationId>(definition_ref)
    {
        DraughtingModelItemDefinition::DimensionalLocation(id)
    } else if let Some(gt_ref) =
        crate::entities::pmi::resolve_geometric_tolerance_ref(ctx, definition_ref)
    {
        DraughtingModelItemDefinition::GeometricTolerance(gt_ref)
    } else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "DRAUGHTING_MODEL_ITEM_ASSOCIATION definition #{definition_ref} resolves to \
                     none of the 6 modelled SELECT members — skipping"
                ),
            });
        return None;
    };
    let used_representation = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(used_ref)?;
    let identified_item = DraughtingModelIdentifiedItem::resolve_select(ctx, identified_ref)?;
    Some(DraughtingModelItemAssociation {
        name,
        description,
        definition,
        used_representation,
        identified_item,
        annotation_placeholder: None,
    })
}

/// Lower one `DRAUGHTING_MODEL_ITEM_ASSOCIATION`.
pub(crate) fn lower_draughting_model_item_association(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingModelItemAssociation,
) {
    let Some(dmia) = lower_dmia_common(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.definition,
        early.used_representation,
        early.identified_item,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_model_item_associations
        .push(dmia);
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER` (base +
/// `annotation_placeholder`; unresolved placeholder drops the entity).
pub(crate) fn lower_draughting_model_item_association_with_placeholder(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingModelItemAssociationWithPlaceholder,
) {
    let Some(mut dmia) = lower_dmia_common(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.definition,
        early.used_representation,
        early.identified_item,
    ) else {
        return;
    };
    let Some(ph_id) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationOccurrenceId>(early.annotation_placeholder)
    else {
        return;
    };
    dmia.annotation_placeholder = Some(ph_id);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_model_item_associations
        .push(dmia);
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANNOTATION_OCCURRENCE_ASSOCIATIVITY` (pairs two
/// `annotation_occurrence` SELECT members). Either end resolving to no modelled
/// annotation-occurrence arena warns + drops (symmetric on re-read).
pub(crate) fn lower_annotation_occurrence_associativity(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAnnotationOccurrenceAssociativity,
) {
    let Some(relating) =
        AnnotationOccurrenceRef::resolve_select(ctx, early.relating_annotation_occurrence)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "ANNOTATION_OCCURRENCE_ASSOCIATIVITY relating #{} resolves to no modelled \
                     annotation occurrence — skipping",
                    early.relating_annotation_occurrence
                ),
            });
        return;
    };
    let Some(related) =
        AnnotationOccurrenceRef::resolve_select(ctx, early.related_annotation_occurrence)
    else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "ANNOTATION_OCCURRENCE_ASSOCIATIVITY related #{} resolves to no modelled \
                     annotation occurrence — skipping",
                    early.related_annotation_occurrence
                ),
            });
        return;
    };
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrence_associativities
        .push(AnnotationOccurrenceAssociativity {
            name: early.name,
            description: early.description,
            relating,
            related,
        });
}
