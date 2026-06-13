//! PMI-domain `lift` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyCylindricityTolerance, EarlyFlatnessTolerance, EarlyRoundnessTolerance,
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
