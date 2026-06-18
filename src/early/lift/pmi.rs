//! PMI-domain `lift` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyAngularLocation, EarlyAngularSize, EarlyAngularityTolerance,
    EarlyAnnotationCurveOccurrence, EarlyAnnotationOccurrence,
    EarlyAnnotationOccurrenceAssociativity, EarlyAnnotationPlaceholderOccurrence,
    EarlyAnnotationPlaceholderOccurrenceWithLeaderLine, EarlyAnnotationPlane,
    EarlyAnnotationSymbolOccurrence, EarlyAnnotationTextOccurrence,
    EarlyAnnotationToModelLeaderLine, EarlyApllPoint, EarlyApllPointWithSurface, EarlyAreaUnitType,
    EarlyAuxiliaryLeaderLine, EarlyCircularRunoutTolerance, EarlyCircularRunoutToleranceComplex,
    EarlyConcentricityTolerance, EarlyCylindricityTolerance, EarlyDatum, EarlyDatumFeature,
    EarlyDimensionalLocation, EarlyDimensionalSize, EarlyDimensionalSizeWithDatumFeature,
    EarlyDirectedDimensionalLocation, EarlyDraughtingAnnotationOccurrence, EarlyDraughtingCallout,
    EarlyDraughtingCalloutRelationship, EarlyDraughtingModelItemAssociation,
    EarlyDraughtingModelItemAssociationWithPlaceholder, EarlyDraughtingPreDefinedTextFont,
    EarlyFlatnessTolerance, EarlyFlatnessToleranceComplex, EarlyFlatnessToleranceComplexModifiers,
    EarlyFlatnessToleranceComplexUnit, EarlyFlatnessToleranceComplexUnitArea,
    EarlyGeometricToleranceModifier, EarlyGeometricToleranceRelationship, EarlyLeaderCurve,
    EarlyLeaderDirectedCallout, EarlyLeaderTerminator, EarlyLimitsAndFits,
    EarlyMeasureQualification, EarlyParallelismTolerance, EarlyParallelismToleranceComplex,
    EarlyPerpendicularityTolerance, EarlyPerpendicularityToleranceComplex, EarlyPlusMinusTolerance,
    EarlyProjectedZoneDefinition, EarlyRoundnessTolerance, EarlyRoundnessToleranceComplex,
    EarlyStraightnessTolerance, EarlyStraightnessToleranceComplex, EarlySurfaceProfileTolerance,
    EarlySymmetryTolerance, EarlyTerminatorSymbol, EarlyTessellatedAnnotationOccurrence,
    EarlyToleranceValue, EarlyToleranceZoneForm, EarlyTotalRunoutTolerance, EarlyTypeQualifier,
    EarlyValueFormatTypeQualifier,
};
use crate::ir::geometry::Point3;
use crate::ir::pmi::{
    AnnotationPlaceholderOccurrenceRole, DesApllPointSymbol, LimitsAndFits,
    TessellatedAnnotationOccurrence,
};
use crate::writer::buffer::WriteBuffer;

/// Lift one `TOLERANCE_ZONE_FORM`.
pub(crate) fn lift_tolerance_zone_form(name: String) -> EarlyToleranceZoneForm {
    EarlyToleranceZoneForm { name }
}

/// Lift one `TYPE_QUALIFIER`.
pub(crate) fn lift_type_qualifier(name: String) -> EarlyTypeQualifier {
    EarlyTypeQualifier { name }
}

/// Lift one `VALUE_FORMAT_TYPE_QUALIFIER`.
pub(crate) fn lift_value_format_type_qualifier(
    format_type: String,
) -> EarlyValueFormatTypeQualifier {
    EarlyValueFormatTypeQualifier { format_type }
}

/// Lift one simple-form `FLATNESS_TOLERANCE` (refs pre-resolved; the legacy writer
/// always emitted `description`/`magnitude` — never `$`).
pub(crate) fn lift_flatness_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
) -> EarlyFlatnessTolerance {
    EarlyFlatnessTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
    }
}

/// Expand the reduced L2 [`crate::ir::GeometricToleranceModifier`] back to the
/// strict L1 enum. `Other(s)` re-parses the token (always a schema member, since
/// `lower` only emits schema tokens); a non-schema token yields `None` (dropped).
/// Inverse of [`super::super::lower::pmi::early_modifier_to_l2`].
pub(crate) fn l2_modifier_to_early(
    m: &crate::ir::GeometricToleranceModifier,
) -> Option<EarlyGeometricToleranceModifier> {
    use crate::ir::GeometricToleranceModifier as L;
    use EarlyGeometricToleranceModifier as E;
    Some(match m {
        L::MaximumMaterialRequirement => E::MaximumMaterialRequirement,
        L::LeastMaterialRequirement => E::LeastMaterialRequirement,
        L::ReciprocityRequirement => E::ReciprocityRequirement,
        L::Other(s) => match s.as_str() {
            "STANDARD_DEVIATION" => E::StandardDeviation,
            "VALLEY_DEPTH" => E::ValleyDepth,
            "PEAK_HEIGHT" => E::PeakHeight,
            "TOTAL_RANGE_DEVIATIONS" => E::TotalRangeDeviations,
            "REFERENCE_MAXIMUM_INSCRIBED_FEATURE" => E::ReferenceMaximumInscribedFeature,
            "REFERENCE_MINIMUM_CIRCUMSCRIBED_FEATURE" => E::ReferenceMinimumCircumscribedFeature,
            "REFERENCE_LEAST_SQUARE_FEATURE_WITH_INTERNAL_MATERIAL_CONSTRAINT" => {
                E::ReferenceLeastSquareFeatureWithInternalMaterialConstraint
            }
            "REFERENCE_LEAST_SQUARE_FEATURE_WITH_EXTERNAL_MATERIAL_CONSTRAINT" => {
                E::ReferenceLeastSquareFeatureWithExternalMaterialConstraint
            }
            "REFERENCE_LEAST_SQUARE_FEATURE_WITHOUT_CONSTRAINT" => {
                E::ReferenceLeastSquareFeatureWithoutConstraint
            }
            "REFERENCE_MINIMAX_FEATURE_WITH_INTERNAL_MATERIAL_CONSTRAINT" => {
                E::ReferenceMinimaxFeatureWithInternalMaterialConstraint
            }
            "REFERENCE_MINIMAX_FEATURE_WITH_EXTERNAL_MATERIAL_CONSTRAINT" => {
                E::ReferenceMinimaxFeatureWithExternalMaterialConstraint
            }
            "REFERENCE_MINIMAX_FEATURE_WITHOUT_CONSTRAINT" => {
                E::ReferenceMinimaxFeatureWithoutConstraint
            }
            "ASSOCIATED_MAXIMUM_INSCRIBED_FEATURE" => E::AssociatedMaximumInscribedFeature,
            "ASSOCIATED_TANGENT_FEATURE" => E::AssociatedTangentFeature,
            "ASSOCIATED_MINIMUM_INSCRIBED_FEATURE" => E::AssociatedMinimumInscribedFeature,
            "ASSOCIATED_LEAST_SQUARE_FEATURE" => E::AssociatedLeastSquareFeature,
            "ASSOCIATED_MINMAX_FEATURE" => E::AssociatedMinmaxFeature,
            "UNITED_FEATURE" => E::UnitedFeature,
            "SEPARATE_REQUIREMENT" => E::SeparateRequirement,
            "EACH_RADIAL_ELEMENT" => E::EachRadialElement,
            "TANGENT_PLANE" => E::TangentPlane,
            "STATISTICAL_TOLERANCE" => E::StatisticalTolerance,
            "NOT_CONVEX" => E::NotConvex,
            "LINE_ELEMENT" => E::LineElement,
            "PITCH_DIAMETER" => E::PitchDiameter,
            "MAJOR_DIAMETER" => E::MajorDiameter,
            "MINOR_DIAMETER" => E::MinorDiameter,
            "COMMON_ZONE" => E::CommonZone,
            "FREE_STATE" => E::FreeState,
            "ANY_CROSS_SECTION" => E::AnyCrossSection,
            "CIRCLE_A" => E::CircleA,
            _ => return None,
        },
    })
}

/// Expand the reduced L2 [`crate::ir::AreaUnitType`] back to the strict L1 enum.
fn l2_area_to_early(a: &crate::ir::AreaUnitType) -> EarlyAreaUnitType {
    use crate::ir::AreaUnitType as L;
    match a {
        L::Circular => EarlyAreaUnitType::Circular,
        L::Rectangular => EarlyAreaUnitType::Rectangular,
        L::Square => EarlyAreaUnitType::Square,
        L::Other(s) if s == "CYLINDRICAL" => EarlyAreaUnitType::Cylindrical,
        // SPHERICAL + any other token re-expand to the closest strict member.
        L::Other(_) => EarlyAreaUnitType::Spherical,
    }
}

/// Lift a `FLATNESS_TOLERANCE` COMPLEX form. Refs (`magnitude` / `tsa` /
/// `unit_size` / `second_unit_size`) are pre-resolved to step ids by the writer.
/// Variant selected by which optional is present (unit takes precedence; a data
/// carrying both unit and modifiers is unsupported — no dispatch case — so the
/// modifier set is dropped, flagged by the debug assert).
pub(crate) fn lift_flatness_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    modifiers: &[crate::ir::GeometricToleranceModifier],
    unit_size: Option<u64>,
    area: Option<(crate::ir::AreaUnitType, Option<u64>)>,
) -> EarlyFlatnessToleranceComplex {
    debug_assert!(
        unit_size.is_none() || modifiers.is_empty(),
        "FLATNESS complex: unit_size + modifiers has no dispatch case"
    );
    if let Some(unit_size) = unit_size {
        if let Some((area_type, second_unit_size)) = area {
            return EarlyFlatnessToleranceComplex::UnitArea(
                EarlyFlatnessToleranceComplexUnitArea {
                    name,
                    description: Some(description),
                    magnitude: Some(magnitude),
                    toleranced_shape_aspect,
                    unit_size,
                    area_type: l2_area_to_early(&area_type),
                    second_unit_size,
                },
            );
        }
        return EarlyFlatnessToleranceComplex::Unit(EarlyFlatnessToleranceComplexUnit {
            name,
            description: Some(description),
            magnitude: Some(magnitude),
            toleranced_shape_aspect,
            unit_size,
        });
    }
    EarlyFlatnessToleranceComplex::Modifiers(EarlyFlatnessToleranceComplexModifiers {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        modifiers: modifiers.iter().filter_map(l2_modifier_to_early).collect(),
    })
}

/// Lift a `ROUNDNESS_TOLERANCE` COMPLEX form (single case: modifiers).
pub(crate) fn lift_roundness_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    modifiers: &[crate::ir::GeometricToleranceModifier],
) -> EarlyRoundnessToleranceComplex {
    EarlyRoundnessToleranceComplex {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        modifiers: modifiers.iter().filter_map(l2_modifier_to_early).collect(),
    }
}

/// Lift a `STRAIGHTNESS_TOLERANCE` COMPLEX form (single case: unit).
pub(crate) fn lift_straightness_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    unit_size: u64,
) -> EarlyStraightnessToleranceComplex {
    EarlyStraightnessToleranceComplex {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        unit_size,
    }
}

/// Lift a `PARALLELISM_TOLERANCE` COMPLEX form (WDR + modifiers). Refs
/// (`magnitude` / `tsa` / `datum_system`) pre-resolved to step ids by the writer;
/// modifiers expanded from L2 via the strict bridge.
pub(crate) fn lift_parallelism_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
    modifiers: &[crate::ir::GeometricToleranceModifier],
) -> EarlyParallelismToleranceComplex {
    EarlyParallelismToleranceComplex {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
        modifiers: modifiers.iter().filter_map(l2_modifier_to_early).collect(),
    }
}

/// Lift a `PERPENDICULARITY_TOLERANCE` COMPLEX form (WDR + modifiers).
pub(crate) fn lift_perpendicularity_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
    modifiers: &[crate::ir::GeometricToleranceModifier],
) -> EarlyPerpendicularityToleranceComplex {
    EarlyPerpendicularityToleranceComplex {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
        modifiers: modifiers.iter().filter_map(l2_modifier_to_early).collect(),
    }
}

/// Lift a `CIRCULAR_RUNOUT_TOLERANCE` COMPLEX form (WDR + modifiers).
pub(crate) fn lift_circular_runout_tolerance_complex(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
    modifiers: &[crate::ir::GeometricToleranceModifier],
) -> EarlyCircularRunoutToleranceComplex {
    EarlyCircularRunoutToleranceComplex {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
        modifiers: modifiers.iter().filter_map(l2_modifier_to_early).collect(),
    }
}

/// Lift one simple-form `ANGULARITY_TOLERANCE` (with-datum; refs pre-resolved,
/// `datum_system` pre-resolved to step ids).
pub(crate) fn lift_angularity_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyAngularityTolerance {
    EarlyAngularityTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `CIRCULAR_RUNOUT_TOLERANCE`.
pub(crate) fn lift_circular_runout_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyCircularRunoutTolerance {
    EarlyCircularRunoutTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `CONCENTRICITY_TOLERANCE`.
pub(crate) fn lift_concentricity_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyConcentricityTolerance {
    EarlyConcentricityTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `PARALLELISM_TOLERANCE`.
pub(crate) fn lift_parallelism_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyParallelismTolerance {
    EarlyParallelismTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `PERPENDICULARITY_TOLERANCE`.
pub(crate) fn lift_perpendicularity_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyPerpendicularityTolerance {
    EarlyPerpendicularityTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `SYMMETRY_TOLERANCE`.
pub(crate) fn lift_symmetry_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlySymmetryTolerance {
    EarlySymmetryTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `TOTAL_RUNOUT_TOLERANCE`.
pub(crate) fn lift_total_runout_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
    datum_system: Vec<u64>,
) -> EarlyTotalRunoutTolerance {
    EarlyTotalRunoutTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
        datum_system,
    }
}

/// Lift one simple-form `SURFACE_PROFILE_TOLERANCE` (refs pre-resolved; the legacy writer
/// always emitted `description`/`magnitude` — never `$`).
pub(crate) fn lift_surface_profile_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
) -> EarlySurfaceProfileTolerance {
    EarlySurfaceProfileTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
    }
}

/// Lift one simple-form `STRAIGHTNESS_TOLERANCE` (refs pre-resolved; the legacy writer
/// always emitted `description`/`magnitude` — never `$`).
pub(crate) fn lift_straightness_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
) -> EarlyStraightnessTolerance {
    EarlyStraightnessTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
    }
}

/// Lift one simple-form `ROUNDNESS_TOLERANCE` (refs pre-resolved; the legacy writer
/// always emitted `description`/`magnitude` — never `$`).
pub(crate) fn lift_roundness_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
) -> EarlyRoundnessTolerance {
    EarlyRoundnessTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
    }
}

/// Lift one simple-form `CYLINDRICITY_TOLERANCE` (refs pre-resolved; the legacy writer
/// always emitted `description`/`magnitude` — never `$`).
pub(crate) fn lift_cylindricity_tolerance(
    name: String,
    description: String,
    magnitude: u64,
    toleranced_shape_aspect: u64,
) -> EarlyCylindricityTolerance {
    EarlyCylindricityTolerance {
        name,
        description: Some(description),
        magnitude: Some(magnitude),
        toleranced_shape_aspect,
    }
}

/// `bool` → the L1 LOGICAL slot (legacy emitted `.T.` / `.F.`).
fn bool_to_logical(b: bool) -> crate::ir::geometry::Logical {
    if b {
        crate::ir::geometry::Logical::True
    } else {
        crate::ir::geometry::Logical::False
    }
}

/// Lift one `DATUM` (`of_shape` pre-resolved; legacy always emitted
/// description as a String).
pub(crate) fn lift_datum(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
    identification: String,
) -> EarlyDatum {
    EarlyDatum {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
        identification,
    }
}

/// Lift one plain `DIMENSIONAL_SIZE` (`applies_to` pre-resolved).
pub(crate) fn lift_dimensional_size(applies_to: u64, name: String) -> EarlyDimensionalSize {
    EarlyDimensionalSize { applies_to, name }
}

/// Lift one `GEOMETRIC_TOLERANCE_RELATIONSHIP` (ends pre-resolved).
pub(crate) fn lift_geometric_tolerance_relationship(
    name: String,
    description: String,
    relating_geometric_tolerance: u64,
    related_geometric_tolerance: u64,
) -> EarlyGeometricToleranceRelationship {
    EarlyGeometricToleranceRelationship {
        name,
        description,
        relating_geometric_tolerance,
        related_geometric_tolerance,
    }
}

/// Lift one `MEASURE_QUALIFICATION` (refs pre-resolved).
pub(crate) fn lift_measure_qualification(
    name: String,
    description: String,
    qualified_measure: u64,
    qualifiers: Vec<u64>,
) -> EarlyMeasureQualification {
    EarlyMeasureQualification {
        name,
        description,
        qualified_measure,
        qualifiers,
    }
}

/// Lift one plain `DIMENSIONAL_LOCATION` (endpoints pre-resolved).
pub(crate) fn lift_dimensional_location(
    d: crate::ir::pmi::DimensionalLocationData,
    relating_shape_aspect: u64,
    related_shape_aspect: u64,
) -> EarlyDimensionalLocation {
    EarlyDimensionalLocation {
        name: d.name,
        description: Some(d.description),
        relating_shape_aspect,
        related_shape_aspect,
    }
}

/// Lift one `DIRECTED_DIMENSIONAL_LOCATION`.
pub(crate) fn lift_directed_dimensional_location(
    d: crate::ir::pmi::DimensionalLocationData,
    relating_shape_aspect: u64,
    related_shape_aspect: u64,
) -> EarlyDirectedDimensionalLocation {
    EarlyDirectedDimensionalLocation {
        name: d.name,
        description: Some(d.description),
        relating_shape_aspect,
        related_shape_aspect,
    }
}

/// Lift one `ANGULAR_LOCATION`.
pub(crate) fn lift_angular_location(
    d: crate::ir::pmi::AngularLocationData,
    relating_shape_aspect: u64,
    related_shape_aspect: u64,
) -> EarlyAngularLocation {
    EarlyAngularLocation {
        name: d.name,
        description: Some(d.description),
        relating_shape_aspect,
        related_shape_aspect,
        angle_selection: d.angle_selection,
    }
}

/// Lift one `ANGULAR_SIZE` (`applies_to` pre-resolved).
pub(crate) fn lift_angular_size(
    applies_to: u64,
    name: String,
    angle_selection: crate::ir::pmi::AngleSelection,
) -> EarlyAngularSize {
    EarlyAngularSize {
        applies_to,
        name,
        angle_selection,
    }
}

/// Lift one plain `DATUM_FEATURE` (`of_shape` pre-resolved).
pub(crate) fn lift_datum_feature(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyDatumFeature {
    EarlyDatumFeature {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift one `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` (`of_shape`/`applies_to`
/// pre-resolved by the emit site; `applies_to` is the WR1 self-reference).
pub(crate) fn lift_dimensional_size_with_datum_feature(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
    applies_to: u64,
    size_name: String,
) -> EarlyDimensionalSizeWithDatumFeature {
    EarlyDimensionalSizeWithDatumFeature {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
        applies_to,
        name_2: size_name,
    }
}

/// Lift one plain `DRAUGHTING_CALLOUT` (contents pre-resolved).
pub(crate) fn lift_draughting_callout(name: String, contents: Vec<u64>) -> EarlyDraughtingCallout {
    EarlyDraughtingCallout { name, contents }
}

/// Lift one `LEADER_DIRECTED_CALLOUT` (contents pre-resolved).
pub(crate) fn lift_leader_directed_callout(
    name: String,
    contents: Vec<u64>,
) -> EarlyLeaderDirectedCallout {
    EarlyLeaderDirectedCallout { name, contents }
}

/// Lift the styled `ANNOTATION_TEXT_OCCURRENCE` complex. `styles` = emitted
/// `PRESENTATION_STYLE_ASSIGNMENT` step ids; `item` = emitted item step id (the
/// handler pre-resolves both).
pub(crate) fn lift_annotation_text_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyAnnotationTextOccurrence {
    EarlyAnnotationTextOccurrence { name, styles, item }
}

/// Lift the styled `LEADER_CURVE` complex. `styles` = emitted PSA step ids;
/// `item` = emitted curve step id (handler pre-resolves both).
pub(crate) fn lift_leader_curve(name: String, styles: Vec<u64>, item: u64) -> EarlyLeaderCurve {
    EarlyLeaderCurve { name, styles, item }
}

/// Lift the styled `LEADER_TERMINATOR` complex. `item` = emitted item step id,
/// `annotated_curve` = emitted `LEADER_CURVE` step id (handler pre-resolves all).
pub(crate) fn lift_leader_terminator(
    name: String,
    styles: Vec<u64>,
    item: u64,
    annotated_curve: u64,
) -> EarlyLeaderTerminator {
    EarlyLeaderTerminator {
        name,
        styles,
        item,
        annotated_curve,
    }
}

/// Lift the plain `ANNOTATION_OCCURRENCE` (`styles` = emitted PSA step ids,
/// `item` = emitted item step id; the handler pre-resolves both).
pub(crate) fn lift_annotation_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyAnnotationOccurrence {
    EarlyAnnotationOccurrence { name, styles, item }
}

/// Lift one `DRAUGHTING_ANNOTATION_OCCURRENCE` (refs pre-resolved).
pub(crate) fn lift_draughting_annotation_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyDraughtingAnnotationOccurrence {
    EarlyDraughtingAnnotationOccurrence { name, styles, item }
}

/// Lift one `ANNOTATION_SYMBOL_OCCURRENCE` (refs pre-resolved).
pub(crate) fn lift_annotation_symbol_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyAnnotationSymbolOccurrence {
    EarlyAnnotationSymbolOccurrence { name, styles, item }
}

/// Lift the plain `ANNOTATION_CURVE_OCCURRENCE` (refs pre-resolved).
pub(crate) fn lift_annotation_curve_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyAnnotationCurveOccurrence {
    EarlyAnnotationCurveOccurrence { name, styles, item }
}

/// Lift one `ANNOTATION_PLANE` (refs pre-resolved). `elements` is not modelled
/// in L2, so it always lifts to `None` → serialized as `$` (matching the legacy
/// writer's unconditional unset).
pub(crate) fn lift_annotation_plane(
    name: String,
    styles: Vec<u64>,
    item: u64,
) -> EarlyAnnotationPlane {
    EarlyAnnotationPlane {
        name,
        styles,
        item,
        elements: None,
    }
}

/// Lift one `TESSELLATED_ANNOTATION_OCCURRENCE` (`item`/`styles` → cached step ids).
pub(crate) fn lift_tessellated_annotation_occurrence(
    buf: &WriteBuffer,
    tao: TessellatedAnnotationOccurrence,
) -> EarlyTessellatedAnnotationOccurrence {
    EarlyTessellatedAnnotationOccurrence {
        name: tao.name,
        styles: tao.styles.iter().map(|&psa| buf.step_id(psa)).collect(),
        item: buf.step_id(tao.item),
    }
}

/// Lift one `LIMITS_AND_FITS` (four scalar grades).
pub(crate) fn lift_limits_and_fits(lf: LimitsAndFits) -> EarlyLimitsAndFits {
    EarlyLimitsAndFits {
        form_variance: lf.form_variance,
        zone_variance: lf.zone_variance,
        grade: lf.grade,
        source: lf.source,
    }
}

/// Lift one `DRAUGHTING_PRE_DEFINED_TEXT_FONT`.
pub(crate) fn lift_draughting_pre_defined_text_font(
    name: String,
) -> EarlyDraughtingPreDefinedTextFont {
    EarlyDraughtingPreDefinedTextFont { name }
}

/// Lift one `DRAUGHTING_CALLOUT_RELATIONSHIP` (endpoints pre-resolved to step ids).
pub(crate) fn lift_draughting_callout_relationship(
    name: String,
    description: String,
    relating_draughting_callout: u64,
    related_draughting_callout: u64,
) -> EarlyDraughtingCalloutRelationship {
    EarlyDraughtingCalloutRelationship {
        name,
        description,
        relating_draughting_callout,
        related_draughting_callout,
    }
}

/// Lift one `TOLERANCE_VALUE` (both magnitudes pre-emitted to step ids).
pub(crate) fn lift_tolerance_value(lower_bound: u64, upper_bound: u64) -> EarlyToleranceValue {
    EarlyToleranceValue {
        lower_bound,
        upper_bound,
    }
}

/// Lift one `PLUS_MINUS_TOLERANCE` (both SELECT members pre-emitted to step ids).
pub(crate) fn lift_plus_minus_tolerance(
    range: u64,
    toleranced_dimension: u64,
) -> EarlyPlusMinusTolerance {
    EarlyPlusMinusTolerance {
        range,
        toleranced_dimension,
    }
}

/// Lift one `PROJECTED_ZONE_DEFINITION` (all refs pre-emitted to step ids).
pub(crate) fn lift_projected_zone_definition(
    zone: u64,
    boundaries: Vec<u64>,
    projection_end: u64,
    projected_length: u64,
) -> EarlyProjectedZoneDefinition {
    EarlyProjectedZoneDefinition {
        zone,
        boundaries,
        projection_end,
        projected_length,
    }
}

/// Lift one `TERMINATOR_SYMBOL` (`styles`/`item`/`annotated_curve` pre-emitted
/// to step ids by the handler).
pub(crate) fn lift_terminator_symbol(
    name: String,
    styles: Vec<u64>,
    item: u64,
    annotated_curve: u64,
) -> EarlyTerminatorSymbol {
    EarlyTerminatorSymbol {
        name,
        styles,
        item,
        annotated_curve,
    }
}

/// Lift one `ANNOTATION_PLACEHOLDER_OCCURRENCE` (`styles`/`item` pre-emitted to
/// step ids; `role`/`line_spacing` pass through).
pub(crate) fn lift_annotation_placeholder_occurrence(
    name: String,
    styles: Vec<u64>,
    item: u64,
    role: AnnotationPlaceholderOccurrenceRole,
    line_spacing: f64,
) -> EarlyAnnotationPlaceholderOccurrence {
    EarlyAnnotationPlaceholderOccurrence {
        name,
        styles,
        item,
        role,
        line_spacing,
    }
}

/// Lift one `ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE` (base APO +
/// `leader_line` step ids).
pub(crate) fn lift_annotation_placeholder_occurrence_with_leader_line(
    name: String,
    styles: Vec<u64>,
    item: u64,
    role: AnnotationPlaceholderOccurrenceRole,
    line_spacing: f64,
    leader_line: Vec<u64>,
) -> EarlyAnnotationPlaceholderOccurrenceWithLeaderLine {
    EarlyAnnotationPlaceholderOccurrenceWithLeaderLine {
        name,
        styles,
        item,
        role,
        line_spacing,
        leader_line,
    }
}

/// Lift one `ANNOTATION_TO_MODEL_LEADER_LINE` (`geometric_elements` pre-emitted
/// to step ids by the handler).
pub(crate) fn lift_annotation_to_model_leader_line(
    name: String,
    geometric_elements: Vec<u64>,
) -> EarlyAnnotationToModelLeaderLine {
    EarlyAnnotationToModelLeaderLine {
        name,
        geometric_elements,
    }
}

/// Lift one `AUXILIARY_LEADER_LINE` (base + `controlling_leader_line` step id).
pub(crate) fn lift_auxiliary_leader_line(
    name: String,
    geometric_elements: Vec<u64>,
    controlling_leader_line: u64,
) -> EarlyAuxiliaryLeaderLine {
    EarlyAuxiliaryLeaderLine {
        name,
        geometric_elements,
        controlling_leader_line,
    }
}

/// Lift one `APLL_POINT` (`Point3` → 3-element coordinate list, mirroring the
/// `CARTESIAN_POINT` lift; `symbol_applied` passes through).
pub(crate) fn lift_apll_point(
    name: String,
    coordinates: &Point3,
    symbol_applied: DesApllPointSymbol,
) -> EarlyApllPoint {
    EarlyApllPoint {
        name,
        coordinates: vec![coordinates.x, coordinates.y, coordinates.z],
        symbol_applied,
    }
}

/// Lift one `APLL_POINT_WITH_SURFACE` (`associated_surface` pre-emitted to a
/// step id by the handler).
pub(crate) fn lift_apll_point_with_surface(
    name: String,
    coordinates: &Point3,
    symbol_applied: DesApllPointSymbol,
    associated_surface: u64,
) -> EarlyApllPointWithSurface {
    EarlyApllPointWithSurface {
        name,
        coordinates: vec![coordinates.x, coordinates.y, coordinates.z],
        symbol_applied,
        associated_surface,
    }
}

/// Lift one `DRAUGHTING_MODEL_ITEM_ASSOCIATION` (all refs pre-resolved to step
/// ids by the handler: definition 6-way, used `step_id`, identified `emit_select`).
pub(crate) fn lift_dmia(
    name: String,
    description: Option<String>,
    definition: u64,
    used_representation: u64,
    identified_item: u64,
) -> EarlyDraughtingModelItemAssociation {
    EarlyDraughtingModelItemAssociation {
        name,
        description,
        definition,
        used_representation,
        identified_item,
    }
}

/// Lift one `DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER` (base +
/// `annotation_placeholder` step id).
pub(crate) fn lift_dmia_with_placeholder(
    name: String,
    description: Option<String>,
    definition: u64,
    used_representation: u64,
    identified_item: u64,
    annotation_placeholder: u64,
) -> EarlyDraughtingModelItemAssociationWithPlaceholder {
    EarlyDraughtingModelItemAssociationWithPlaceholder {
        name,
        description,
        definition,
        used_representation,
        identified_item,
        annotation_placeholder,
    }
}

/// Lift one `ANNOTATION_OCCURRENCE_ASSOCIATIVITY` (`relating`/`related`
/// pre-emitted to step ids via `emit_select` by the handler).
pub(crate) fn lift_annotation_occurrence_associativity(
    name: String,
    description: String,
    relating_annotation_occurrence: u64,
    related_annotation_occurrence: u64,
) -> EarlyAnnotationOccurrenceAssociativity {
    EarlyAnnotationOccurrenceAssociativity {
        name,
        description,
        relating_annotation_occurrence,
        related_annotation_occurrence,
    }
}
