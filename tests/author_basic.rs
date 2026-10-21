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
    // A trimming select pointing at an APLL point: the entity kind exists in
    // the universal model but is not AP242 — the constructor refuses it.
    let base = a
        .add_cartesian_point("", [0.0, 0.0, 0.0].into())
        .expect("point");
    let dir = a.add_direction("", [1.0, 0.0, 0.0].into()).expect("dir");
    let vec_ = a
        .add_vector("", m::DirectionRef::Direction(dir), 1.0)
        .expect("vector");
    let line = a
        .add_line(
            "",
            m::CartesianPointRef::CartesianPoint(base),
            m::VectorRef::Vector(vec_),
        )
        .expect("line");
    let err = a
        .add_trimmed_curve(
            "",
            m::CurveRef::Line(line),
            vec![m::TrimmingSelectRef::ApllPoint(m::ApllPointId(0))],
            vec![m::TrimmingSelectRef::ParameterValue(1.0)],
            true,
            m::TrimmingPreference::Parameter,
        )
        .expect_err("apll trim must be rejected");
    assert!(matches!(
        err,
        AuthorError::NotAp242 {
            entity: "APLL_POINT"
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

    // A part outside AP242 (ed3 leader-line PMI).
    let err = a
        .add_complex(vec![
            m::PartialEntity::AnnotationPlaceholderOccurrenceWithLeaderLine {
                leader_line: vec![],
            },
        ])
        .expect_err("non-AP242 part must be rejected");
    assert!(matches!(
        err,
        AuthorError::NotAp242 {
            entity: "ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE"
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
            m::CharacterizedDefinitionRef::ProductDefinition(pd),
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
