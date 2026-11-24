//! The strict AP242 authoring layer: constructors exist only for AP242-legal
//! entities, references are validated on insertion, and `finish()` emits
//! Part 21 text that reads back with zero drops.

mod common;

use step_io::model as m;
use step_io::{Ap242Author, AuthorError, read};

#[test]
fn authored_model_round_trips_with_zero_drops() {
    let mut a = Ap242Author::new();

    let origin = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let z = a.add_direction("", [0.0, 0.0, 1.0].into()).expect("dir z");
    let x = a.add_direction("", [1.0, 0.0, 0.0].into()).expect("dir x");
    let frame = a
        .add_axis2_placement3d(
            "",
            m::CartesianPointRef::CartesianPoint(origin),
            Some(m::DirectionRef::Direction(z)),
            Some(m::DirectionRef::Direction(x)),
        )
        .expect("frame");
    let plane = a
        .add_plane("", m::Axis2Placement3dRef::Axis2Placement3d(frame))
        .expect("plane");
    let ctx = a.add_representation_context("ctx", "3D").expect("context");
    a.add_shape_representation(
        "anchor",
        vec![
            m::RepresentationItemRef::Axis2Placement3d(frame),
            m::RepresentationItemRef::Plane(plane),
        ],
        m::RepresentationContextRef::RepresentationContext(ctx),
    )
    .expect("shape representation");

    let text = common::text(&a.finish());
    assert!(text.contains("AP242"), "AP242 header stamped");

    // The authored file reads back whole: nothing dropped, entities present.
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.all::<step_io::model::Plane>().len(), 1);
    assert_eq!(model.all::<step_io::model::ShapeRepresentation>().len(), 1);
    assert_eq!(model.all::<step_io::model::CartesianPoint>().len(), 1);
}

#[test]
fn illegal_reference_is_rejected_at_insertion() {
    let mut a = Ap242Author::new();
    // A presentation style select pointing at a pre-defined presentation
    // style: the entity kind exists in the universal model but is not
    // AP242 — the constructor refuses it.
    let err = a
        .add_presentation_style_assignment(vec![
            m::PresentationStyleSelectRef::PreDefinedPresentationStyle(
                m::PreDefinedPresentationStyleId(0),
            ),
        ])
        .expect_err("a pre-defined presentation style must be rejected");
    assert!(matches!(
        err,
        AuthorError::NotAp242 {
            entity: "PRE_DEFINED_PRESENTATION_STYLE"
        }
    ));
}

#[test]
fn dangling_reference_is_rejected_at_insertion() {
    let mut a = Ap242Author::new();
    // A vector whose direction id points past the (empty) direction arena.
    let err = a
        .add_vector("", m::DirectionRef::Direction(m::DirectionId(7)), 1.0)
        .expect_err("out-of-range id must be rejected");
    assert!(matches!(
        err,
        AuthorError::DanglingRef {
            entity: "DIRECTION"
        }
    ));
}

#[test]
fn complex_instances_round_trip_with_zero_drops() {
    let mut a = Ap242Author::new();
    let three_d = a.text("3D");
    let empty = a.text("");
    let ctx = a.text("ctx");

    // Rational B-spline curve — the most frequent complex in real files (7 parts).
    let cps: Vec<m::CartesianPointRef> = [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]]
        .iter()
        .map(|c| {
            m::CartesianPointRef::CartesianPoint(a.add_cartesian_point("", (*c).into()).unwrap())
        })
        .collect();
    let curve = a
        .add_complex(vec![
            m::PartialEntity::BoundedCurve,
            m::PartialEntity::BSplineCurve {
                degree: 2,
                control_points_list: cps,
                curve_form: m::BSplineCurveForm::Unspecified,
                closed_curve: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::PartialEntity::BSplineCurveWithKnots {
                knot_multiplicities: vec![3, 3],
                knots: vec![0.0, 1.0],
                knot_spec: m::KnotType::Unspecified,
            },
            m::PartialEntity::Curve,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RationalBSplineCurve {
                weights_data: vec![1.0, 2.0, 1.0],
            },
            m::PartialEntity::RepresentationItem { name: empty },
        ])
        .expect("rational b-spline curve complex");

    // A bounded segment of it, so the complex is reachable from the shape.
    let trim = a
        .add_trimmed_curve(
            "",
            m::CurveRef::Complex(curve),
            vec![m::TrimmingSelectRef::ParameterValue(0.0)],
            vec![m::TrimmingSelectRef::ParameterValue(1.0)],
            true,
            m::TrimmingPreference::Parameter,
        )
        .expect("trimmed curve on complex basis");

    // Unit complex (millimetre) referenced by a context complex.
    let mm = a
        .add_complex(vec![
            m::PartialEntity::LengthUnit,
            m::PartialEntity::NamedUnit { dimensions: None },
            m::PartialEntity::SiUnit {
                prefix: Some(m::SiPrefix::Milli),
                name: m::SiUnitName::Metre,
            },
        ])
        .expect("si length unit complex");
    let ctx = a
        .add_complex(vec![
            m::PartialEntity::GeometricRepresentationContext {
                coordinate_space_dimension: 3,
            },
            m::PartialEntity::GlobalUnitAssignedContext {
                units: vec![m::UnitRef::Complex(mm)],
            },
            m::PartialEntity::RepresentationContext {
                context_identifier: ctx,
                context_type: three_d,
            },
        ])
        .expect("unit-assigned context complex");

    a.add_shape_representation(
        "anchor",
        vec![m::RepresentationItemRef::TrimmedCurve(trim)],
        m::RepresentationContextRef::Complex(ctx),
    )
    .expect("shape representation");

    let text = common::text(&a.finish());
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.all::<step_io::model::ComplexEntity>().len(), 3);
    assert_eq!(model.all::<step_io::model::TrimmedCurve>().len(), 1);
}

#[test]
fn invalid_complex_part_sets_are_rejected() {
    let mut a = Ap242Author::new();
    let empty = a.text("");

    // Missing supertype part: SI_UNIT alone (NAMED_UNIT chain absent).
    let err = a
        .add_complex(vec![m::PartialEntity::SiUnit {
            prefix: None,
            name: m::SiUnitName::Metre,
        }])
        .expect_err("incomplete chain must be rejected");
    assert!(matches!(
        err,
        AuthorError::InvalidComplex {
            reason: "missing supertype part"
        }
    ));

    // Disjoint hierarchies: a unit part bag mixed with curve parts.
    let err = a
        .add_complex(vec![
            m::PartialEntity::BoundedCurve,
            m::PartialEntity::Curve,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RepresentationItem { name: empty },
            m::PartialEntity::LengthUnit,
            m::PartialEntity::NamedUnit { dimensions: None },
            m::PartialEntity::SiUnit {
                prefix: None,
                name: m::SiUnitName::Metre,
            },
        ])
        .expect_err("disjoint hierarchies must be rejected");
    assert!(matches!(
        err,
        AuthorError::InvalidComplex {
            reason: "disjoint parts"
        }
    ));

    // ONEOF violation: BEZIER_CURVE and B_SPLINE_CURVE_WITH_KNOTS are
    // exclusive branches of b_spline_curve's ONEOF.
    let err = a
        .add_complex(vec![
            m::PartialEntity::BezierCurve,
            m::PartialEntity::BoundedCurve,
            m::PartialEntity::BSplineCurve {
                degree: 1,
                control_points_list: vec![],
                curve_form: m::BSplineCurveForm::Unspecified,
                closed_curve: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::PartialEntity::BSplineCurveWithKnots {
                knot_multiplicities: vec![],
                knots: vec![],
                knot_spec: m::KnotType::Unspecified,
            },
            m::PartialEntity::Curve,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RepresentationItem { name: empty },
        ])
        .expect_err("oneof conflict must be rejected");
    assert!(matches!(
        err,
        AuthorError::InvalidComplex {
            reason: "oneof conflict"
        }
    ));

    // A part outside AP242 (an approximation tolerance, which AP214 has).
    let err = a
        .add_complex(vec![m::PartialEntity::ApproximationTolerance {
            tolerance: m::ToleranceSelectRef::ApproximationToleranceDeviation(
                m::ApproximationToleranceDeviationId(0),
            ),
        }])
        .expect_err("non-AP242 part must be rejected");
    assert!(matches!(
        err,
        AuthorError::NotAp242 {
            entity: "APPROXIMATION_TOLERANCE"
        }
    ));

    // A dangling reference inside an otherwise valid part bag.
    let err = a
        .add_complex(vec![
            m::PartialEntity::LengthUnit,
            m::PartialEntity::NamedUnit {
                dimensions: Some(m::DimensionalExponentsRef::DimensionalExponents(
                    m::DimensionalExponentsId(9),
                )),
            },
            m::PartialEntity::SiUnit {
                prefix: None,
                name: m::SiUnitName::Metre,
            },
        ])
        .expect_err("dangling part reference must be rejected");
    assert!(matches!(
        err,
        AuthorError::DanglingRef {
            entity: "DIMENSIONAL_EXPONENTS"
        }
    ));
}

#[test]
fn self_referencing_entity_round_trips_with_zero_drops() {
    let mut a = Ap242Author::new();
    let diameter = a.text("diameter");
    let datum_feature = a.text("datum feature");

    // Minimal product chain up to the PRODUCT_DEFINITION_SHAPE.
    let ac = a.add_application_context("mim").expect("ac");
    let pc = a
        .add_product_context(
            "",
            m::ApplicationContextRef::ApplicationContext(ac),
            "mechanical",
        )
        .expect("pc");
    let pdc = a
        .add_product_definition_context(
            "part definition",
            m::ApplicationContextRef::ApplicationContext(ac),
            "design",
        )
        .expect("pdc");
    let product = a
        .add_product(
            "P1",
            "part",
            None,
            vec![m::ProductContextRef::ProductContext(pc)],
        )
        .expect("product");
    let pdf = a
        .add_product_definition_formation("1", None, m::ProductRef::Product(product))
        .expect("pdf");
    let pd = a
        .add_product_definition(
            "design",
            None,
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(pdf),
            m::ProductDefinitionContextRef::ProductDefinitionContext(pdc),
        )
        .expect("pd");
    let pds = a
        .add_product_definition_shape(
            "",
            None,
            m::Ap242ProductDefinitionShapeDefinitionRef::ProductDefinition(pd),
        )
        .expect("pds");

    // The stc07 pattern: DIMENSIONAL_SIZE_WITH_DATUM_FEATURE whose
    // applies_to references the entity itself (WR1).
    let dswdf = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: datum_feature,
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(pds),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: diameter,
        })
        .expect("self-referencing DSWDF");
    assert_eq!(dswdf.0, 0);

    let text = common::text(&a.finish());
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let items: Vec<_> = model.all::<m::DimensionalSizeWithDatumFeature>().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].1.applies_to,
        m::ShapeAspectRef::DimensionalSizeWithDatumFeature(m::DimensionalSizeWithDatumFeatureId(0)),
        "applies_to must reference the entity itself after the round trip"
    );
}

#[test]
fn cyclic_constructor_rolls_back_on_dangling_reference() {
    let mut a = Ap242Author::new();
    let empty = a.text("");
    // A genuinely dangling forward reference (id + 1) must be rejected and
    // the insertion rolled back — model unchanged.
    let err = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: empty,
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(
                m::DimensionalSizeWithDatumFeatureId(id.0 + 1),
            ),
            name_1: empty,
        })
        .expect_err("forward dangling reference must be rejected");
    assert!(matches!(err, AuthorError::DanglingRef { .. }));

    // Rolled back: a retry with a valid self-reference gets the same id 0.
    let pds_missing = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: empty,
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: empty,
        })
        .expect_err("of_shape still dangling (empty pds arena)");
    assert!(matches!(
        pds_missing,
        AuthorError::DanglingRef {
            entity: "PRODUCT_DEFINITION_SHAPE"
        }
    ));

    // Direct rollback proof: a shape-representation anchor makes a valid
    // NEXT insertion possible in another arena, and the closure of a third
    // cyclic attempt still sees id 0 — both failed insertions were popped.
    let seen = std::cell::Cell::new(u32::MAX);
    let _ = a.add_dimensional_size_with_datum_feature_cyclic(|id| {
        seen.set(id.0);
        m::DimensionalSizeWithDatumFeature {
            name: empty,
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: empty,
        }
    });
    assert_eq!(seen.get(), 0, "both rolled-back insertions must be popped");
}

#[test]
fn aggregate_cardinality_is_enforced() {
    let mut a = Ap242Author::new();
    let three_d = a.text("3D");
    let empty = a.text("");

    // DIRECTION.direction_ratios is LIST [2:3].
    let err = a
        .add_direction("", [1.0].into())
        .expect_err("one ratio is below the minimum");
    assert!(matches!(
        err,
        AuthorError::Cardinality {
            entity: "DIRECTION",
            attribute: "direction_ratios",
            got: 1,
            min: 2,
            max: Some(3),
        }
    ));
    assert!(
        m::Reals3::from_slice(&[1.0, 0.0, 0.0, 0.0]).is_none(),
        "four ratios do not fit the type"
    );
    a.add_direction("", [0.0, 0.0, 1.0].into())
        .expect("three ratios are fine");

    // Inner lists are bounded too: a mesh vertex is LIST [3:3] OF REAL.
    let err = a
        .add_coordinates_list("", 1, vec![[1.0, 2.0].into()])
        .expect_err("a two-coordinate vertex is below the minimum");
    assert!(matches!(
        err,
        AuthorError::Cardinality {
            entity: "COORDINATES_LIST",
            attribute: "position_coords",
            got: 2,
            min: 3,
            max: Some(3),
        }
    ));

    // Complex parts get the same guard: GLOBAL_UNIT_ASSIGNED_CONTEXT.units
    // is SET [1:?].
    let err = a
        .add_complex(vec![
            m::PartialEntity::GeometricRepresentationContext {
                coordinate_space_dimension: 3,
            },
            m::PartialEntity::GlobalUnitAssignedContext { units: vec![] },
            m::PartialEntity::RepresentationContext {
                context_identifier: empty,
                context_type: three_d,
            },
        ])
        .expect_err("empty units set is below the minimum");
    assert!(matches!(
        err,
        AuthorError::Cardinality {
            entity: "GLOBAL_UNIT_ASSIGNED_CONTEXT",
            attribute: "units",
            ..
        }
    ));
}

#[test]
fn complex_instance_of_the_wrong_type_is_rejected() {
    let mut a = Ap242Author::new();
    let mm = a
        .add_complex(vec![
            m::PartialEntity::LengthUnit,
            m::PartialEntity::NamedUnit { dimensions: None },
            m::PartialEntity::SiUnit {
                prefix: Some(m::SiPrefix::Milli),
                name: m::SiUnitName::Metre,
            },
        ])
        .expect("si length unit complex");
    // A unit is no curve.
    let err = a
        .add_trimmed_curve(
            "",
            m::CurveRef::Complex(mm),
            vec![m::TrimmingSelectRef::ParameterValue(0.0)],
            vec![m::TrimmingSelectRef::ParameterValue(1.0)],
            true,
            m::TrimmingPreference::Parameter,
        )
        .expect_err("a unit complex as a curve");
    assert_eq!(
        err,
        AuthorError::InvalidComplex {
            reason: "not a CURVE"
        }
    );
}

#[test]
fn cyclic_constructor_enforces_cardinality() {
    let mut a = Ap242Author::new();
    let status = a.add_approval_status("approved").expect("status");
    let approval = a
        .add_approval(m::ApprovalStatusRef::ApprovalStatus(status), "")
        .expect("approval");
    // APPLIED_APPROVAL_ASSIGNMENT.items is SET [1:?].
    let err = a
        .add_applied_approval_assignment_cyclic(|_| m::AppliedApprovalAssignment {
            assigned_approval: m::ApprovalRef::Approval(approval),
            items: Vec::new(),
        })
        .expect_err("empty items set is below the minimum");
    assert!(matches!(
        err,
        AuthorError::Cardinality {
            entity: "APPLIED_APPROVAL_ASSIGNMENT",
            attribute: "items",
            ..
        }
    ));
}

#[test]
fn non_finite_reals_are_rejected() {
    let mut a = Ap242Author::new();
    let err = a
        .add_cartesian_point("", [f64::NAN, 0.0, 0.0].into())
        .expect_err("a NaN coordinate");
    assert_eq!(
        err,
        AuthorError::NonFiniteReal {
            entity: "CARTESIAN_POINT",
            attribute: "coordinates"
        }
    );
    let at = placement(&mut a);
    let err = a
        .add_circle("", at.clone(), f64::INFINITY)
        .expect_err("an infinite radius");
    assert!(matches!(
        err,
        AuthorError::NonFiniteReal {
            entity: "CIRCLE",
            ..
        }
    ));

    // A parameter-value trim, inside a reference select.
    let circle = a.add_circle("", at, 1.0).expect("circle");
    let err = a
        .add_trimmed_curve(
            "",
            m::CurveRef::Circle(circle),
            vec![m::TrimmingSelectRef::ParameterValue(f64::NAN)],
            vec![m::TrimmingSelectRef::ParameterValue(1.0)],
            true,
            m::TrimmingPreference::Parameter,
        )
        .expect_err("a NaN trim");
    assert_eq!(
        err,
        AuthorError::NonFiniteReal {
            entity: "PARAMETER_VALUE",
            attribute: "value"
        }
    );
}

// An axis placement at the origin, for curves.
fn placement(a: &mut Ap242Author) -> m::Axis2PlacementRef {
    let origin = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let frame = a
        .add_axis2_placement3d("", m::CartesianPointRef::CartesianPoint(origin), None, None)
        .expect("frame");
    m::Axis2PlacementRef::Axis2Placement3d(frame)
}

/// A list nested in an aggregate is held to its bounds as the aggregate is: a
/// row of one control point or one weight is refused, in a plain B-spline
/// surface and in the parts of a rational one.
#[test]
fn lists_nested_in_aggregates_keep_their_bounds() {
    let mut a = Ap242Author::new();
    let point = |a: &mut Ap242Author, c: [f64; 3]| {
        m::CartesianPointRef::CartesianPoint(a.add_cartesian_point("", c.into()).unwrap())
    };
    let row = |a: &mut Ap242Author, y: f64| vec![point(a, [0.0, y, 0.0]), point(a, [1.0, y, 0.0])];
    let short_row = vec![point(&mut a, [0.0, 2.0, 0.0])];
    let rows = vec![row(&mut a, 0.0), row(&mut a, 1.0), short_row];
    let nested = |entity: &'static str, attribute: &'static str, got: usize, min: usize| {
        AuthorError::Cardinality {
            entity,
            attribute,
            got,
            min,
            max: None,
        }
    };

    let err = a
        .add_b_spline_surface_with_knots(
            "",
            1,
            1,
            rows.clone(),
            m::BSplineSurfaceForm::Unspecified,
            m::Logical::False,
            m::Logical::False,
            m::Logical::False,
            vec![2, 1, 2],
            vec![2, 2],
            vec![0.0, 0.5, 1.0],
            vec![0.0, 1.0],
            m::KnotType::Unspecified,
        )
        .expect_err("a row of one point");
    assert_eq!(
        err,
        nested("B_SPLINE_SURFACE_WITH_KNOTS", "control_points_list", 1, 2)
    );

    let empty = a.text("");
    let rational = |rows: Vec<Vec<m::CartesianPointRef>>, weights: Vec<Vec<f64>>| {
        vec![
            m::PartialEntity::BoundedSurface,
            m::PartialEntity::BSplineSurface {
                u_degree: 1,
                v_degree: 1,
                control_points_list: rows,
                surface_form: m::BSplineSurfaceForm::Unspecified,
                u_closed: m::Logical::False,
                v_closed: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::PartialEntity::BSplineSurfaceWithKnots {
                u_multiplicities: vec![2, 1, 2],
                v_multiplicities: vec![2, 2],
                u_knots: vec![0.0, 0.5, 1.0],
                v_knots: vec![0.0, 1.0],
                knot_spec: m::KnotType::Unspecified,
            },
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RationalBSplineSurface {
                weights_data: weights,
            },
            m::PartialEntity::RepresentationItem { name: empty },
            m::PartialEntity::Surface,
        ]
    };
    let err = a
        .add_complex(rational(rows.clone(), vec![vec![1.0; 2]; 3]))
        .expect_err("a row of one point in a complex instance");
    assert_eq!(err, nested("B_SPLINE_SURFACE", "control_points_list", 1, 2));
    let full = vec![rows[0].clone(), rows[1].clone(), row(&mut a, 2.0)];
    let err = a
        .add_complex(rational(
            full.clone(),
            vec![vec![1.0; 2], vec![1.0; 2], vec![1.0]],
        ))
        .expect_err("a row of one weight");
    assert_eq!(
        err,
        nested("RATIONAL_B_SPLINE_SURFACE", "weights_data", 1, 2)
    );
    a.add_complex(rational(full, vec![vec![1.0; 2]; 3]))
        .expect("rows of two points and two weights");
}

/// A triangle strip of two points is refused, as its list nests in the
/// aggregate of strips.
#[test]
fn triangle_strips_keep_their_bounds() {
    let mut a = Ap242Author::new();
    let coordinates = a
        .add_coordinates_list(
            "",
            3,
            vec![
                [0.0; 3].into(),
                [1.0, 0.0, 0.0].into(),
                [0.0, 1.0, 0.0].into(),
            ],
        )
        .unwrap();
    let face = |a: &mut Ap242Author, strip: Vec<i64>| {
        a.add_complex_triangulated_face(
            "",
            m::CoordinatesListRef::CoordinatesList(coordinates),
            3,
            vec![],
            None,
            vec![],
            vec![strip],
            vec![],
        )
    };
    let err = face(&mut a, vec![1, 2]).expect_err("a strip of two points");
    assert_eq!(
        err,
        AuthorError::Cardinality {
            entity: "COMPLEX_TRIANGULATED_FACE",
            attribute: "triangle_strips",
            got: 2,
            min: 3,
            max: None,
        }
    );
    face(&mut a, vec![1, 2, 3]).expect("a strip of three points");
}

/// AP242 narrows some attributes more than older editions do, and the model,
/// read from every edition, keeps the wider type. Where an argument's type
/// cannot rule a wider value out — a complex instance's parts, the struct a
/// self-referencing constructor takes — the check runs on insertion.
#[test]
fn narrowed_attributes_are_checked_where_the_type_cannot() {
    let mut a = Ap242Author::new();
    let plain = a.add_representation_context("ctx", "3D").expect("context");
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let name = a.text("");
    // PRESENTATION_REPRESENTATION narrows REPRESENTATION.context_of_items to a
    // geometric context.
    let err = a
        .add_complex(vec![
            m::PartialEntity::PresentationRepresentation,
            m::PartialEntity::Representation {
                name,
                items: vec![m::RepresentationItemRef::CartesianPoint(point)],
                context_of_items: m::RepresentationContextRef::RepresentationContext(plain),
            },
        ])
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "PRESENTATION_REPRESENTATION",
                attribute: "context_of_items"
            }
        ),
        "{err:?}"
    );

    // AP242 edition 4's groupable_item takes ACTION_METHOD again, as older
    // editions do.
    let group = a.add_group("g", None).expect("group");
    let method = a.add_action_method("m", None, "", "").expect("method");
    a.add_applied_group_assignment_cyclic(|_| m::AppliedGroupAssignment {
        assigned_group: m::GroupRef::Group(group),
        items: vec![m::GroupableItemRef::ActionMethod(method)],
    })
    .expect("an action method may be grouped");
}

/// Attributes the model takes wider, as older editions declare them, are
/// checked against AP242's type where the argument's type cannot.
#[test]
fn widened_attributes_are_checked_where_the_type_cannot() {
    let mut a = Ap242Author::new();
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let name = a.text("");

    // AP242 types a tolerance magnitude as a length measure; the model, read
    // from AP203/AP214 as well, takes any measure with unit.
    let aspect = shape_aspect(&mut a);
    let mm = a
        .add_complex(vec![
            m::PartialEntity::LengthUnit,
            m::PartialEntity::NamedUnit { dimensions: None },
            m::PartialEntity::SiUnit {
                prefix: Some(m::SiPrefix::Milli),
                name: m::SiUnitName::Metre,
            },
        ])
        .expect("si length unit complex");
    let length = a.text("LENGTH_MEASURE");
    let plain = a
        .add_measure_with_unit(
            m::MeasureValue {
                type_name: Some(length),
                value: m::MeasureScalar::Real(0.1),
            },
            m::UnitRef::Complex(mm),
        )
        .expect("measure");
    let tol = a.text("pos");
    let err = a
        .add_complex(vec![
            m::PartialEntity::GeometricTolerance {
                name: tol,
                description: None,
                magnitude: Some(m::MeasureWithUnitRef::MeasureWithUnit(plain)),
                toleranced_shape_aspect: m::GeometricToleranceTargetRef::ShapeAspect(aspect),
            },
            m::PartialEntity::GeometricToleranceWithModifiers {
                modifiers: vec![m::GeometricToleranceModifier::MaximumMaterialRequirement],
            },
            m::PartialEntity::PositionTolerance,
        ])
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "GEOMETRIC_TOLERANCE",
                attribute: "magnitude"
            }
        ),
        "{err:?}"
    );

    // AP242's styled_item_target leaves out a descriptive item, which AP214
    // lets a styled item style.
    let note = a
        .add_descriptive_representation_item("note", "text")
        .expect("item");
    let style = a
        .add_presentation_style_assignment(vec![m::PresentationStyleSelectRef::NullStyle(
            m::NullStyle::Null,
        )])
        .expect("style");
    let styled = a
        .add_styled_item(
            "",
            vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
            m::Ap242StyledItemItemRef::CartesianPoint(point),
        )
        .expect("styled item");
    let err = a
        .add_over_riding_styled_item_cyclic(|_| m::OverRidingStyledItem {
            name,
            styles: vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
            item: m::StyledItemTargetOrRepresentationItemRef::DescriptiveRepresentationItem(note),
            over_ridden_style: m::StyledItemRef::StyledItem(styled),
        })
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "OVER_RIDING_STYLED_ITEM",
                attribute: "item"
            }
        ),
        "{err:?}"
    );
}

/// A shape aspect of a minimal product.
fn shape_aspect(a: &mut Ap242Author) -> m::ShapeAspectId {
    let ac = a.add_application_context("mim").expect("ac");
    let pc = a
        .add_product_context(
            "",
            m::ApplicationContextRef::ApplicationContext(ac),
            "mechanical",
        )
        .expect("pc");
    let pdc = a
        .add_product_definition_context(
            "part definition",
            m::ApplicationContextRef::ApplicationContext(ac),
            "design",
        )
        .expect("pdc");
    let product = a
        .add_product(
            "P1",
            "part",
            None,
            vec![m::ProductContextRef::ProductContext(pc)],
        )
        .expect("product");
    let pdf = a
        .add_product_definition_formation("1", None, m::ProductRef::Product(product))
        .expect("pdf");
    let pd = a
        .add_product_definition(
            "design",
            None,
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(pdf),
            m::ProductDefinitionContextRef::ProductDefinitionContext(pdc),
        )
        .expect("pd");
    let pds = a
        .add_product_definition_shape(
            "",
            None,
            m::Ap242ProductDefinitionShapeDefinitionRef::ProductDefinition(pd),
        )
        .expect("pds");
    a.add_shape_aspect(
        "feature",
        None,
        m::ProductDefinitionShapeRef::ProductDefinitionShape(pds),
        Some(m::Logical::True),
    )
    .expect("shape aspect")
}

/// AP242 redeclares a person-and-organization address to name exactly one
/// organization and one person (`SET [1:1]`).
#[test]
fn ap242_aggregate_bounds_apply() {
    let mut a = Ap242Author::new();
    let o1 = a.add_organization(None, "a", None).expect("organization");
    let o2 = a.add_organization(None, "b", None).expect("organization");
    let p = a
        .add_person("p", None, None, None, None, None)
        .expect("person");
    let err = a
        .add_person_and_organization_address(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            vec![
                m::OrganizationRef::Organization(o1),
                m::OrganizationRef::Organization(o2),
            ],
            None,
            vec![m::PersonRef::Person(p)],
            None,
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::Cardinality {
                attribute: "organizations",
                got: 2,
                min: 1,
                max: Some(1),
                ..
            }
        ),
        "{err:?}"
    );
}

/// A narrower argument converts into the model's wider reference, a complex
/// entity included.
#[test]
fn narrower_arguments_convert_into_the_model_type() {
    let r: m::RepresentationRef =
        m::ShapeRepresentationRef::ShapeRepresentation(m::ShapeRepresentationId(3)).into();
    assert_eq!(
        r,
        m::RepresentationRef::ShapeRepresentation(m::ShapeRepresentationId(3))
    );
    let r: m::RepresentationRef = m::ShapeRepresentationRef::Complex(m::ComplexEntityId(1)).into();
    assert_eq!(r, m::RepresentationRef::Complex(m::ComplexEntityId(1)));
}

/// The plain constructor of a self-referencing entity takes AP242's narrower
/// type like any other, and stores it as the model's.
#[test]
fn a_self_referencing_entity_plain_constructor_takes_the_ap242_type() {
    let mut a = Ap242Author::new();
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let styled = a
        .add_styled_item("", vec![], m::Ap242StyledItemItemRef::CartesianPoint(point))
        .expect("styled item");
    let id = a
        .add_over_riding_styled_item(
            "",
            vec![],
            m::Ap242StyledItemItemRef::CartesianPoint(point),
            m::StyledItemRef::StyledItem(styled),
        )
        .expect("over-riding styled item");
    let model = a.finish();
    assert_eq!(
        model[id].item,
        m::StyledItemTargetOrRepresentationItemRef::CartesianPoint(point)
    );
}

/// AP242's WHERE rules that restrict which types an attribute references
/// narrow the constructor's type, and are checked at run time where a
/// complex value or a complex part carries them.
#[test]
fn where_rule_type_restrictions_are_checked() {
    let mut a = Ap242Author::new();
    let si = |a: &mut Ap242Author, kind, prefix, name| {
        a.add_complex(vec![
            kind,
            m::PartialEntity::NamedUnit { dimensions: None },
            m::PartialEntity::SiUnit { prefix, name },
        ])
        .expect("si unit complex")
    };
    let mm = si(
        &mut a,
        m::PartialEntity::LengthUnit,
        Some(m::SiPrefix::Milli),
        m::SiUnitName::Metre,
    );
    let gram = si(
        &mut a,
        m::PartialEntity::MassUnit,
        None,
        m::SiUnitName::Gram,
    );
    let length = a.text("LENGTH_MEASURE");
    let value = m::MeasureValue {
        type_name: Some(length),
        value: m::MeasureScalar::Real(2.5),
    };

    // LENGTH_MEASURE_WITH_UNIT.WR1: the unit is a length unit.
    a.add_length_measure_with_unit(value.clone(), m::Ap242LengthUnitRef::Complex(mm))
        .expect("length in millimetres");
    let err = a
        .add_length_measure_with_unit(value.clone(), m::Ap242LengthUnitRef::Complex(gram))
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::InvalidComplex {
                reason: "not a LENGTH_UNIT"
            }
        ),
        "{err:?}"
    );

    // The same rule holds where LENGTH_MEASURE_WITH_UNIT is a complex part.
    let name = a.text("");
    let part = |unit| {
        vec![
            m::PartialEntity::LengthMeasureWithUnit,
            m::PartialEntity::MeasureRepresentationItem,
            m::PartialEntity::MeasureWithUnit {
                value_component: value.clone(),
                unit_component: m::UnitRef::Complex(unit),
            },
            m::PartialEntity::RepresentationItem { name },
        ]
    };
    let item = a.add_complex(part(mm)).expect("length measure item");
    let err = a.add_complex(part(gram)).unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "LENGTH_MEASURE_WITH_UNIT",
                attribute: "unit_component"
            }
        ),
        "{err:?}"
    );

    // MEASURE_QUALIFICATION.WR2 excludes a representation item: a complex
    // value with a REPRESENTATION_ITEM part breaks it, though its
    // MEASURE_WITH_UNIT part is allowed.
    let precision = a.add_precision_qualifier(3).expect("qualifier");
    let err = a
        .add_measure_qualification(
            "q",
            "",
            m::Ap242MeasureQualificationQualifiedMeasureRef::Complex(item),
            vec![m::ValueQualifierRef::PrecisionQualifier(precision)],
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "MEASURE_QUALIFICATION",
                attribute: "qualified_measure"
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_constructive_item_may_be_a_listed_subtype() {
    let mut a = Ap242Author::new();
    let name = a.text("");
    let ctx = a
        .add_geometric_representation_context("", "3D", 3)
        .expect("context");
    let ctx = m::GeometricRepresentationContextRef::GeometricRepresentationContext(ctx);
    let cps: Vec<m::CartesianPointRef> = [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]]
        .iter()
        .map(|c| {
            m::CartesianPointRef::CartesianPoint(a.add_cartesian_point("", (*c).into()).unwrap())
        })
        .collect();
    let p = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");

    // A rational B-spline curve, written as seven parts, is a curve.
    let curve = a
        .add_complex(vec![
            m::PartialEntity::BoundedCurve,
            m::PartialEntity::BSplineCurve {
                degree: 2,
                control_points_list: cps,
                curve_form: m::BSplineCurveForm::Unspecified,
                closed_curve: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::PartialEntity::BSplineCurveWithKnots {
                knot_multiplicities: vec![3, 3],
                knot_spec: m::KnotType::Unspecified,
                knots: vec![0.0, 1.0],
            },
            m::PartialEntity::Curve,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RationalBSplineCurve {
                weights_data: vec![1.0, 2.0, 1.0],
            },
            m::PartialEntity::RepresentationItem { name },
        ])
        .expect("rational b-spline curve complex");
    a.add_constructive_geometry_representation(
        "",
        vec![m::Ap242ConstructiveGeometryRepresentationItemsRef::Complex(
            curve,
        )],
        ctx.clone(),
    )
    .expect("one curve");

    // CONSTRUCTIVE_GEOMETRY_REPRESENTATION.WR2 lists `circle` with `conic`
    // and `curve`; read as asking for exactly one, it would leave a circle
    // out, so it is read as asking for at least one.
    let axis = a
        .add_axis2_placement3d("", m::CartesianPointRef::CartesianPoint(p), None, None)
        .expect("axis");
    let circle = a
        .add_complex(vec![
            m::PartialEntity::Circle { radius: 1.0 },
            m::PartialEntity::Conic {
                position: m::Axis2PlacementRef::Axis2Placement3d(axis),
            },
            m::PartialEntity::Curve,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RepresentationItem { name },
        ])
        .expect("circle complex");
    a.add_constructive_geometry_representation(
        "",
        vec![
            m::Ap242ConstructiveGeometryRepresentationItemsRef::Complex(circle),
            m::Ap242ConstructiveGeometryRepresentationItemsRef::CartesianPoint(p),
        ],
        ctx,
    )
    .expect("a circle and a cartesian point");
}

/// `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION.WR1`: each item is
/// exactly one of a geometric set, a mapped item or an axis placement. A
/// complex instance's type is its parts, counted once per listed type.
#[test]
fn exactly_one_of_several_types_counts_every_part() {
    let mut a = Ap242Author::new();
    let ctx = a
        .add_geometric_representation_context("", "3D", 3)
        .expect("context");
    let ctx = m::RepresentationContextRef::GeometricRepresentationContext(ctx);
    let p = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let axis_parts = |a: &mut Ap242Author| {
        let name = a.text("");
        vec![
            m::PartialEntity::Axis2Placement3d {
                axis: None,
                ref_direction: None,
            },
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::Placement {
                location: m::CartesianPointRef::CartesianPoint(p),
            },
            m::PartialEntity::RepresentationItem { name },
        ]
    };

    // An axis placement is one of them, though it has four parts.
    let parts = axis_parts(&mut a);
    let axis = a.add_complex(parts).expect("axis placement complex");
    a.add_geometrically_bounded_surface_shape_representation(
        "",
        vec![m::Ap242GeometricallyBoundedSurfaceShapeRepresentationItemsRef::Complex(axis)],
        ctx.clone(),
    )
    .expect("one axis placement");

    // An axis placement that is also a mapped item is two of them.
    let origin = a
        .add_axis2_placement3d("", m::CartesianPointRef::CartesianPoint(p), None, None)
        .expect("origin");
    let origin = m::RepresentationItemRef::Axis2Placement3d(origin);
    let shape = a
        .add_shape_representation("", vec![origin.clone()], ctx.clone())
        .expect("shape");
    let map = a
        .add_representation_map(
            origin.clone(),
            m::RepresentationRef::ShapeRepresentation(shape),
        )
        .expect("map");
    let mut parts = axis_parts(&mut a);
    parts.push(m::PartialEntity::MappedItem {
        mapping_source: m::RepresentationMapRef::RepresentationMap(map),
        mapping_target: origin,
    });
    let both = a
        .add_complex(parts)
        .expect("axis placement and mapped item");
    let err = a
        .add_geometrically_bounded_surface_shape_representation(
            "",
            vec![m::Ap242GeometricallyBoundedSurfaceShapeRepresentationItemsRef::Complex(both)],
            ctx,
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
                attribute: "items"
            }
        ),
        "{err:?}"
    );
}

/// `STYLED_ITEM.WR3` (AP242 edition 4): a styled item styles a mapped item,
/// a geometric item, or a set of them — a context-dependent over-riding
/// styled item included. A complex instance's type is its parts.
#[test]
fn a_styled_item_styles_mapped_and_geometric_items_or_a_set_of_them() {
    let mut a = Ap242Author::new();
    let name = a.text("");
    let style = a
        .add_presentation_style_assignment(vec![m::PresentationStyleSelectRef::NullStyle(
            m::NullStyle::Null,
        )])
        .expect("style");
    let styles = vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)];
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let other = a
        .add_cartesian_point("", [1.0, 0.0, 0.0].into())
        .expect("point");
    let set = a
        .add_styled_item(
            "",
            styles.clone(),
            m::Ap242StyledItemItemRef::SetRepresentationItem(vec![
                m::Ap242StyledItemItemSetRepresentationItemRef::CartesianPoint(point),
                m::Ap242StyledItemItemSetRepresentationItemRef::CartesianPoint(other),
            ]),
        )
        .expect("a styled set of geometric items");
    let vertex = a.add_vertex("").expect("vertex");
    let context = vec![m::StyleContextSelectRef::CartesianPoint(point)];

    let over_riding = |context: Option<Vec<m::StyleContextSelectRef>>,
                       item: m::StyledItemTargetOrRepresentationItemRef| {
        let mut parts: Vec<m::PartialEntity> = context
            .into_iter()
            .map(
                |style_context| m::PartialEntity::ContextDependentOverRidingStyledItem {
                    style_context,
                },
            )
            .collect();
        parts.extend([
            m::PartialEntity::OverRidingStyledItem {
                over_ridden_style: m::StyledItemRef::StyledItem(set),
            },
            m::PartialEntity::RepresentationItem { name },
            m::PartialEntity::StyledItem {
                styles: styles.clone(),
                item,
            },
        ]);
        parts
    };
    let narrowed = |err: AuthorError| {
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "STYLED_ITEM",
                attribute: "item"
            }
        )
    };
    for context in [None, Some(context.clone())] {
        let vertex_alone = m::StyledItemTargetOrRepresentationItemRef::Vertex(vertex);
        let err = a
            .add_complex(over_riding(context.clone(), vertex_alone))
            .unwrap_err();
        assert!(narrowed(err.clone()), "{err:?}");
        let vertex_in_a_set =
            m::StyledItemTargetOrRepresentationItemRef::SetRepresentationItem(vec![
                m::RepresentationItemRef::CartesianPoint(point),
                m::RepresentationItemRef::Vertex(vertex),
            ]);
        let err = a
            .add_complex(over_riding(context.clone(), vertex_in_a_set))
            .unwrap_err();
        assert!(narrowed(err.clone()), "{err:?}");
    }
    let points = m::StyledItemTargetOrRepresentationItemRef::SetRepresentationItem(vec![
        m::RepresentationItemRef::CartesianPoint(point),
        m::RepresentationItemRef::CartesianPoint(other),
    ]);
    a.add_complex(over_riding(Some(context), points))
        .expect("a set of geometric items may be styled");

    let model = a.finish();
    assert_eq!(
        model[set].item,
        m::StyledItemTargetOrRepresentationItemRef::SetRepresentationItem(vec![
            m::RepresentationItemRef::CartesianPoint(point),
            m::RepresentationItemRef::CartesianPoint(other),
        ])
    );
}

/// A self-referencing constructor checks a set's elements at run time.
#[test]
fn a_self_referencing_styled_item_checks_its_set_elements() {
    let mut a = Ap242Author::new();
    let name = a.text("");
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let styled = a
        .add_styled_item("", vec![], m::Ap242StyledItemItemRef::CartesianPoint(point))
        .expect("styled item");
    let vertex = a.add_vertex("").expect("vertex");
    let over_riding = |element| m::OverRidingStyledItem {
        name,
        styles: vec![],
        item: m::StyledItemTargetOrRepresentationItemRef::SetRepresentationItem(vec![element]),
        over_ridden_style: m::StyledItemRef::StyledItem(styled),
    };
    let err = a
        .add_over_riding_styled_item_cyclic(|_| {
            over_riding(m::RepresentationItemRef::Vertex(vertex))
        })
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "OVER_RIDING_STYLED_ITEM",
                attribute: "item"
            }
        ),
        "{err:?}"
    );
    a.add_over_riding_styled_item_cyclic(|_| {
        over_riding(m::RepresentationItemRef::CartesianPoint(point))
    })
    .expect("a set of a geometric item");
}

/// `DRAUGHTING_ANNOTATION_OCCURRENCE.WR7` holds one that is also an annotation
/// text occurrence, which only a complex instance is, to a composite text or
/// text literal.
#[test]
fn a_where_rule_for_one_instance_type_is_checked_in_a_complex() {
    let mut a = Ap242Author::new();
    let name = a.text("");
    let style = a
        .add_presentation_style_assignment(vec![m::PresentationStyleSelectRef::NullStyle(
            m::NullStyle::Null,
        )])
        .expect("style");
    let styles = vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)];
    let font = a
        .add_draughting_pre_defined_text_font("ISO 3098")
        .expect("font");
    let at = placement(&mut a);
    let text = a
        .add_text_literal(
            "",
            "A",
            at.clone(),
            "baseline left",
            m::TextPath::Right,
            m::FontSelectRef::DraughtingPreDefinedTextFont(font),
        )
        .expect("text");
    let glyph = a.add_pre_defined_character_glyph("a").expect("glyph");
    let glyph = a
        .add_defined_character_glyph(
            "",
            m::DefinedGlyphSelectRef::PreDefinedCharacterGlyph(glyph),
            at,
        )
        .expect("glyph");
    let text_occurrence = |item| {
        vec![
            m::PartialEntity::AnnotationOccurrence,
            m::PartialEntity::AnnotationTextOccurrence,
            m::PartialEntity::DraughtingAnnotationOccurrence,
            m::PartialEntity::GeometricRepresentationItem,
            m::PartialEntity::RepresentationItem { name },
            m::PartialEntity::StyledItem {
                styles: styles.clone(),
                item,
            },
        ]
    };
    let err = a
        .add_complex(text_occurrence(
            m::StyledItemTargetOrRepresentationItemRef::DefinedCharacterGlyph(glyph),
        ))
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "DRAUGHTING_ANNOTATION_OCCURRENCE",
                attribute: "item"
            }
        ),
        "{err:?}"
    );
    // A set of text literals is none of them: no rule on a text occurrence
    // admits a set.
    let err = a
        .add_complex(text_occurrence(
            m::StyledItemTargetOrRepresentationItemRef::SetRepresentationItem(vec![
                m::RepresentationItemRef::TextLiteral(text),
            ]),
        ))
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                attribute: "item",
                ..
            }
        ),
        "{err:?}"
    );
    a.add_complex(text_occurrence(
        m::StyledItemTargetOrRepresentationItemRef::TextLiteral(text),
    ))
    .expect("a draughting text occurrence of a text literal");
}

/// `datum_or_common_datum` takes a datum reference element only in a list:
/// a self-referencing constructor, which takes the model's type, rejects one
/// alone on insertion and keeps a list of them.
#[test]
fn a_datum_reference_element_alone_is_rejected_in_a_cyclic_constructor() {
    let mut a = Ap242Author::new();
    let ac = a.add_application_context("mim").expect("ac");
    let ac = m::ApplicationContextRef::ApplicationContext(ac);
    let pc = a
        .add_product_context("", ac.clone(), "mechanical")
        .expect("pc");
    let pdc = a
        .add_product_definition_context("part definition", ac, "design")
        .expect("pdc");
    let product = a
        .add_product(
            "P1",
            "part",
            None,
            vec![m::ProductContextRef::ProductContext(pc)],
        )
        .expect("product");
    let pdf = a
        .add_product_definition_formation("1", None, m::ProductRef::Product(product))
        .expect("pdf");
    let pd = a
        .add_product_definition(
            "design",
            None,
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(pdf),
            m::ProductDefinitionContextRef::ProductDefinitionContext(pdc),
        )
        .expect("pd");
    let pds = a
        .add_product_definition_shape(
            "",
            None,
            m::Ap242ProductDefinitionShapeDefinitionRef::ProductDefinition(pd),
        )
        .expect("pds");
    let of_shape = m::ProductDefinitionShapeRef::ProductDefinitionShape(pds);
    let name = a.text("");
    let element = |base| m::DatumReferenceElement {
        name,
        description: None,
        of_shape: of_shape.clone(),
        product_definitional: m::Logical::False,
        base,
        modifiers: None,
    };

    let err = a
        .add_datum_reference_element_cyclic(|id| {
            element(m::DatumOrCommonDatumRef::DatumReferenceElement(id))
        })
        .unwrap_err();
    assert!(
        matches!(
            err,
            AuthorError::NarrowedReference {
                entity: "DATUM_REFERENCE_ELEMENT",
                attribute: "base"
            }
        ),
        "{err:?}"
    );

    let first = a
        .add_datum_reference_element_cyclic(|id| {
            element(m::DatumOrCommonDatumRef::CommonDatumList(vec![
                m::DatumReferenceElementRef::DatumReferenceElement(id),
                m::DatumReferenceElementRef::DatumReferenceElement(id),
            ]))
        })
        .expect("a list of datum reference elements");
    assert_eq!(first.0, 0, "the rejected element was rolled back");

    // A common datum list holds at least two elements.
    let short = AuthorError::Cardinality {
        entity: "COMMON_DATUM_LIST",
        attribute: "value",
        got: 1,
        min: 2,
        max: None,
    };
    let err = a
        .add_datum_reference_element_cyclic(|_| {
            element(m::DatumOrCommonDatumRef::CommonDatumList(vec![
                m::DatumReferenceElementRef::DatumReferenceElement(first),
            ]))
        })
        .unwrap_err();
    assert_eq!(err, short);
    let err = a
        .add_datum_reference_compartment(
            "",
            None,
            of_shape.clone(),
            m::Logical::False,
            m::Ap242DatumOrCommonDatumRef::CommonDatumList(vec![
                m::DatumReferenceElementRef::DatumReferenceElement(first),
            ]),
            None,
        )
        .unwrap_err();
    assert_eq!(err, short);
}

#[test]
fn a_list_held_in_a_select_is_checked_for_its_size() {
    let mut a = Ap242Author::new();
    let style = a
        .add_presentation_style_assignment(vec![m::PresentationStyleSelectRef::NullStyle(
            m::NullStyle::Null,
        )])
        .expect("style");
    let err = a
        .add_styled_item(
            "",
            vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
            m::Ap242StyledItemItemRef::SetRepresentationItem(vec![]),
        )
        .unwrap_err();
    assert_eq!(
        err,
        AuthorError::Cardinality {
            entity: "SET_REPRESENTATION_ITEM",
            attribute: "value",
            got: 0,
            min: 1,
            max: None,
        }
    );

    // The same check runs on a part of a complex instance.
    let ctx = a.add_representation_context("ctx", "3D").expect("context");
    let point = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let mut rep = |name| {
        a.add_shape_representation(
            name,
            vec![m::RepresentationItemRef::CartesianPoint(point)],
            m::RepresentationContextRef::RepresentationContext(ctx),
        )
        .map(m::RepresentationOrRepresentationReferenceRef::ShapeRepresentation)
        .expect("shape representation")
    };
    let (child, parent) = (rep("child"), rep("parent"));
    let name = a.text("");
    let err = a
        .add_complex(vec![
            m::PartialEntity::RepresentationRelationship {
                name,
                description: None,
                rep_1: child,
                rep_2: parent,
            },
            m::PartialEntity::RepresentationRelationshipWithTransformation {
                transformation_operator: m::TransformationRef::SetItemDefinedTransformation(vec![]),
            },
            m::PartialEntity::ShapeRepresentationRelationship,
        ])
        .unwrap_err();
    assert_eq!(
        err,
        AuthorError::Cardinality {
            entity: "SET_ITEM_DEFINED_TRANSFORMATION",
            attribute: "value",
            got: 0,
            min: 1,
            max: None,
        }
    );
}
