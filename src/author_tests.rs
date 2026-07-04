//! Crate-internal tests for the complex-instance name-set rule
//! (`generated::author::complex_rule`).
//!
//! `CORPUS_COMBOS` is the full list of real complex-instance part
//! combinations observed in the reference corpus (core / fusion360 /
//! grabcad / abc), normalized to the model's part world (bare measure-type
//! carriers like `LENGTH_MEASURE` are value carriers, not parts, and are
//! excluded). Every one is schema-legal, so the rule must accept them all —
//! this pins the rule (and the profile's ONEOF export) against regressions.

use crate::generated::author::{AuthorError, complex_rule};

const CORPUS_COMBOS: &[&[&str]] = &[
    &[
        "BOUNDED_CURVE",
        "B_SPLINE_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "CURVE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "RATIONAL_B_SPLINE_CURVE",
        "REPRESENTATION_ITEM",
    ],
    &[
        "BOUNDED_SURFACE",
        "B_SPLINE_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "RATIONAL_B_SPLINE_SURFACE",
        "REPRESENTATION_ITEM",
        "SURFACE",
    ],
    &[
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT",
        "REPRESENTATION_CONTEXT",
    ],
    &["LENGTH_UNIT", "NAMED_UNIT", "SI_UNIT"],
    &["NAMED_UNIT", "PLANE_ANGLE_UNIT", "SI_UNIT"],
    &["NAMED_UNIT", "SI_UNIT", "SOLID_ANGLE_UNIT"],
    &[
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "PARAMETRIC_REPRESENTATION_CONTEXT",
        "REPRESENTATION_CONTEXT",
    ],
    &[
        "REPRESENTATION_RELATIONSHIP",
        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
        "SHAPE_REPRESENTATION_RELATIONSHIP",
    ],
    &["CONVERSION_BASED_UNIT", "LENGTH_UNIT", "NAMED_UNIT"],
    &["CONVERSION_BASED_UNIT", "NAMED_UNIT", "PLANE_ANGLE_UNIT"],
    &[
        "BOUNDED_CURVE",
        "B_SPLINE_CURVE",
        "CURVE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "QUASI_UNIFORM_CURVE",
        "RATIONAL_B_SPLINE_CURVE",
        "REPRESENTATION_ITEM",
    ],
    &["MASS_UNIT", "NAMED_UNIT", "SI_UNIT"],
    &[
        "GEOMETRIC_REPRESENTATION_ITEM",
        "REPOSITIONED_TESSELLATED_ITEM",
        "REPRESENTATION_ITEM",
        "TESSELLATED_GEOMETRIC_SET",
        "TESSELLATED_ITEM",
    ],
    &[
        "BOUNDED_SURFACE",
        "B_SPLINE_SURFACE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "QUASI_UNIFORM_SURFACE",
        "RATIONAL_B_SPLINE_SURFACE",
        "REPRESENTATION_ITEM",
        "SURFACE",
    ],
    &[
        "LENGTH_MEASURE_WITH_UNIT",
        "MEASURE_REPRESENTATION_ITEM",
        "MEASURE_WITH_UNIT",
        "QUALIFIED_REPRESENTATION_ITEM",
        "REPRESENTATION_ITEM",
    ],
    &[
        "ANNOTATION_CURVE_OCCURRENCE",
        "ANNOTATION_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "LEADER_CURVE",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
    ],
    &[
        "ANNOTATION_OCCURRENCE",
        "ANNOTATION_TEXT_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
    ],
    &[
        "ANNOTATION_OCCURRENCE",
        "ANNOTATION_SYMBOL_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "LEADER_TERMINATOR",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
        "TERMINATOR_SYMBOL",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "SURFACE_PROFILE_TOLERANCE",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "POSITION_TOLERANCE",
    ],
    &["CONVERSION_BASED_UNIT", "MASS_UNIT", "NAMED_UNIT"],
    &[
        "LENGTH_MEASURE_WITH_UNIT",
        "MEASURE_REPRESENTATION_ITEM",
        "MEASURE_WITH_UNIT",
        "REPRESENTATION_ITEM",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
        "POSITION_TOLERANCE",
    ],
    &[
        "DRAUGHTING_MODEL",
        "REPRESENTATION",
        "SHAPE_REPRESENTATION",
        "TESSELLATED_SHAPE_REPRESENTATION",
    ],
    &[
        "CHARACTERIZED_OBJECT",
        "CHARACTERIZED_REPRESENTATION",
        "DRAUGHTING_MODEL",
        "REPRESENTATION",
        "SHAPE_REPRESENTATION",
        "TESSELLATED_SHAPE_REPRESENTATION",
    ],
    &[
        "CAMERA_IMAGE",
        "CAMERA_IMAGE_3D_WITH_SCALE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "MAPPED_ITEM",
        "REPRESENTATION_ITEM",
    ],
    &[
        "CHARACTERIZED_OBJECT",
        "CHARACTERIZED_REPRESENTATION",
        "DRAUGHTING_MODEL",
        "REPRESENTATION",
    ],
    &["COMPOSITE_SHAPE_ASPECT", "DATUM_FEATURE", "SHAPE_ASPECT"],
    &[
        "COMPOSITE_GROUP_SHAPE_ASPECT",
        "COMPOSITE_SHAPE_ASPECT",
        "DATUM_FEATURE",
        "SHAPE_ASPECT",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
        "PARALLELISM_TOLERANCE",
    ],
    &[
        "MEASURE_REPRESENTATION_ITEM",
        "MEASURE_WITH_UNIT",
        "PLANE_ANGLE_MEASURE_WITH_UNIT",
        "REPRESENTATION_ITEM",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "SURFACE_PROFILE_TOLERANCE",
        "UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
        "PERPENDICULARITY_TOLERANCE",
    ],
    &[
        "FLATNESS_TOLERANCE",
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
    ],
    &[
        "FLATNESS_TOLERANCE",
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT",
        "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT",
        "STRAIGHTNESS_TOLERANCE",
    ],
    &[
        "MEASURE_REPRESENTATION_ITEM",
        "MEASURE_WITH_UNIT",
        "RATIO_MEASURE_WITH_UNIT",
        "REPRESENTATION_ITEM",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "LINE_PROFILE_TOLERANCE",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
        "SURFACE_PROFILE_TOLERANCE",
    ],
    &[
        "CIRCULAR_RUNOUT_TOLERANCE",
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
    ],
    &[
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_MODIFIERS",
        "ROUNDNESS_TOLERANCE",
    ],
];

#[test]
fn corpus_combos_all_pass_complex_rule() {
    for combo in CORPUS_COMBOS {
        assert_eq!(
            complex_rule(combo),
            Ok(()),
            "corpus combination rejected: {combo:?}"
        );
    }
}

#[test]
fn complex_rule_rejects_invalid_name_sets() {
    // Missing supertype part: SI_UNIT requires NAMED_UNIT.
    assert_eq!(
        complex_rule(&["LENGTH_UNIT", "SI_UNIT"]),
        Err(AuthorError::InvalidComplex {
            reason: "missing supertype part"
        })
    );
    // Disjoint hierarchies: a unit complex mixed with a curve part.
    assert_eq!(
        complex_rule(&[
            "BOUNDED_CURVE",
            "CURVE",
            "GEOMETRIC_REPRESENTATION_ITEM",
            "LENGTH_UNIT",
            "NAMED_UNIT",
            "REPRESENTATION_ITEM",
            "SI_UNIT",
        ]),
        Err(AuthorError::InvalidComplex {
            reason: "disjoint parts"
        })
    );
    // ONEOF violation: BEZIER_CURVE and B_SPLINE_CURVE_WITH_KNOTS are
    // exclusive branches of b_spline_curve's ONEOF.
    assert_eq!(
        complex_rule(&[
            "BEZIER_CURVE",
            "BOUNDED_CURVE",
            "B_SPLINE_CURVE",
            "B_SPLINE_CURVE_WITH_KNOTS",
            "CURVE",
            "GEOMETRIC_REPRESENTATION_ITEM",
            "REPRESENTATION_ITEM",
        ]),
        Err(AuthorError::InvalidComplex {
            reason: "oneof conflict"
        })
    );
    // Duplicates and the empty bag.
    assert_eq!(
        complex_rule(&["NAMED_UNIT", "NAMED_UNIT", "SI_UNIT"]),
        Err(AuthorError::InvalidComplex {
            reason: "duplicate part"
        })
    );
    assert_eq!(
        complex_rule(&[]),
        Err(AuthorError::InvalidComplex {
            reason: "empty part list"
        })
    );
}
