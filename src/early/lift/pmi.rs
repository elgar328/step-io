//! PMI-domain `lift` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyAngularLocation, EarlyAngularSize, EarlyAngularityTolerance,
    EarlyAnnotationCurveOccurrence, EarlyAnnotationOccurrence,
    EarlyAnnotationPlaceholderOccurrence, EarlyAnnotationPlaceholderOccurrenceWithLeaderLine,
    EarlyAnnotationPlane, EarlyAnnotationSymbolOccurrence, EarlyAnnotationTextOccurrence,
    EarlyCircularRunoutTolerance, EarlyConcentricityTolerance, EarlyCylindricityTolerance,
    EarlyDatum, EarlyDatumFeature, EarlyDimensionalLocation, EarlyDimensionalSize,
    EarlyDirectedDimensionalLocation, EarlyDraughtingAnnotationOccurrence, EarlyDraughtingCallout,
    EarlyDraughtingCalloutRelationship, EarlyDraughtingPreDefinedTextFont, EarlyFlatnessTolerance,
    EarlyGeometricToleranceRelationship, EarlyLeaderCurve, EarlyLeaderDirectedCallout,
    EarlyLeaderTerminator, EarlyLimitsAndFits, EarlyMeasureQualification,
    EarlyParallelismTolerance, EarlyPerpendicularityTolerance, EarlyPlusMinusTolerance,
    EarlyProjectedZoneDefinition, EarlyRoundnessTolerance, EarlyStraightnessTolerance,
    EarlySurfaceProfileTolerance, EarlySymmetryTolerance, EarlyTerminatorSymbol,
    EarlyTessellatedAnnotationOccurrence, EarlyToleranceValue, EarlyToleranceZoneForm,
    EarlyTotalRunoutTolerance, EarlyTypeQualifier, EarlyValueFormatTypeQualifier,
};
use crate::ir::pmi::{
    AnnotationPlaceholderOccurrenceRole, LimitsAndFits, TessellatedAnnotationOccurrence,
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
