//! PMI-domain `lift` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyCylindricityTolerance, EarlyDatum, EarlyDimensionalSize, EarlyFlatnessTolerance,
    EarlyGeometricToleranceRelationship, EarlyMeasureQualification, EarlyRoundnessTolerance,
    EarlyStraightnessTolerance, EarlySurfaceProfileTolerance, EarlyToleranceZoneForm,
    EarlyTypeQualifier, EarlyValueFormatTypeQualifier,
};

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
