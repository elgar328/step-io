//! The strict AP242 authoring layer: constructors exist only for AP242-legal
//! entities, references are validated on insertion, and `finish()` emits
//! Part 21 text that reads back with zero drops.

use step_io::generated::model as m;
use step_io::{Ap242Author, AuthorError, read};

#[test]
fn authored_model_round_trips_with_zero_drops() {
    let mut a = Ap242Author::new();

    let origin = a
        .add_cartesian_point(String::new(), vec![0.0, 0.0, 0.0])
        .expect("point");
    let z = a
        .add_direction(String::new(), vec![0.0, 0.0, 1.0])
        .expect("dir z");
    let x = a
        .add_direction(String::new(), vec![1.0, 0.0, 0.0])
        .expect("dir x");
    let frame = a
        .add_axis2_placement3d(
            String::new(),
            m::CartesianPointRef::CartesianPoint(origin),
            Some(m::DirectionRef::Direction(z)),
            Some(m::DirectionRef::Direction(x)),
        )
        .expect("frame");
    let plane = a
        .add_plane(
            String::new(),
            m::Axis2Placement3dRef::Axis2Placement3d(frame),
        )
        .expect("plane");
    let ctx = a
        .add_representation_context("ctx".to_owned(), "3D".to_owned())
        .expect("context");
    a.add_shape_representation(
        "anchor".to_owned(),
        vec![
            m::RepresentationItemRef::Axis2Placement3d(frame),
            m::RepresentationItemRef::Plane(plane),
        ],
        m::RepresentationContextRef::RepresentationContext(ctx),
    )
    .expect("shape representation");

    let text = a.finish();
    assert!(text.contains("AP242"), "AP242 header stamped");

    // The authored file reads back whole: nothing dropped, entities present.
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.plane_arena.items.len(), 1);
    assert_eq!(model.shape_representation_arena.items.len(), 1);
    assert_eq!(model.cartesian_point_arena.items.len(), 1);
}

#[test]
fn illegal_reference_is_rejected_at_insertion() {
    let mut a = Ap242Author::new();
    // A trimming select pointing at an APLL point: the entity kind exists in
    // the universal model but is not AP242 — the constructor refuses it.
    let base = a
        .add_cartesian_point(String::new(), vec![0.0, 0.0, 0.0])
        .expect("point");
    let dir = a
        .add_direction(String::new(), vec![1.0, 0.0, 0.0])
        .expect("dir");
    let vec_ = a
        .add_vector(String::new(), m::DirectionRef::Direction(dir), 1.0)
        .expect("vector");
    let line = a
        .add_line(
            String::new(),
            m::CartesianPointRef::CartesianPoint(base),
            m::VectorRef::Vector(vec_),
        )
        .expect("line");
    let err = a
        .add_trimmed_curve(
            String::new(),
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
        .add_vector(
            String::new(),
            m::DirectionRef::Direction(m::DirectionId(7)),
            1.0,
        )
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

    // Rational B-spline curve — the most frequent corpus complex (7 parts).
    let cps: Vec<m::CartesianPointRef> = [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]]
        .iter()
        .map(|c| {
            m::CartesianPointRef::CartesianPoint(
                a.add_cartesian_point(String::new(), c.to_vec()).unwrap(),
            )
        })
        .collect();
    let curve = a
        .add_complex(vec![
            m::UnitPart::BoundedCurve,
            m::UnitPart::BSplineCurve {
                degree: 2,
                control_points_list: cps,
                curve_form: m::BSplineCurveForm::Unspecified,
                closed_curve: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::UnitPart::BSplineCurveWithKnots {
                knot_multiplicities: vec![3, 3],
                knots: vec![0.0, 1.0],
                knot_spec: m::KnotType::Unspecified,
            },
            m::UnitPart::Curve,
            m::UnitPart::GeometricRepresentationItem,
            m::UnitPart::RationalBSplineCurve {
                weights_data: vec![1.0, 2.0, 1.0],
            },
            m::UnitPart::RepresentationItem {
                name: String::new(),
            },
        ])
        .expect("rational b-spline curve complex");

    // A bounded segment of it, so the complex is reachable from the shape.
    let trim = a
        .add_trimmed_curve(
            String::new(),
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
            m::UnitPart::LengthUnit,
            m::UnitPart::NamedUnit { dimensions: None },
            m::UnitPart::SiUnit {
                prefix: Some(m::SiPrefix::Milli),
                name: m::SiUnitName::Metre,
            },
        ])
        .expect("si length unit complex");
    let ctx = a
        .add_complex(vec![
            m::UnitPart::GeometricRepresentationContext {
                coordinate_space_dimension: 3,
            },
            m::UnitPart::GlobalUnitAssignedContext {
                units: vec![m::UnitRef::Complex(mm)],
            },
            m::UnitPart::RepresentationContext {
                context_identifier: "ctx".to_owned(),
                context_type: "3D".to_owned(),
            },
        ])
        .expect("unit-assigned context complex");

    a.add_shape_representation(
        "anchor".to_owned(),
        vec![m::RepresentationItemRef::TrimmedCurve(trim)],
        m::RepresentationContextRef::Complex(ctx),
    )
    .expect("shape representation");

    let text = a.finish();
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.complex_unit_arena.items.len(), 3);
    assert_eq!(model.trimmed_curve_arena.items.len(), 1);
}

#[test]
fn invalid_complex_part_sets_are_rejected() {
    let mut a = Ap242Author::new();

    // Missing supertype part: SI_UNIT alone (NAMED_UNIT chain absent).
    let err = a
        .add_complex(vec![m::UnitPart::SiUnit {
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
            m::UnitPart::BoundedCurve,
            m::UnitPart::Curve,
            m::UnitPart::GeometricRepresentationItem,
            m::UnitPart::RepresentationItem {
                name: String::new(),
            },
            m::UnitPart::LengthUnit,
            m::UnitPart::NamedUnit { dimensions: None },
            m::UnitPart::SiUnit {
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
            m::UnitPart::BezierCurve,
            m::UnitPart::BoundedCurve,
            m::UnitPart::BSplineCurve {
                degree: 1,
                control_points_list: vec![],
                curve_form: m::BSplineCurveForm::Unspecified,
                closed_curve: m::Logical::False,
                self_intersect: m::Logical::False,
            },
            m::UnitPart::BSplineCurveWithKnots {
                knot_multiplicities: vec![],
                knots: vec![],
                knot_spec: m::KnotType::Unspecified,
            },
            m::UnitPart::Curve,
            m::UnitPart::GeometricRepresentationItem,
            m::UnitPart::RepresentationItem {
                name: String::new(),
            },
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
            m::UnitPart::AnnotationPlaceholderOccurrenceWithLeaderLine {
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
            m::UnitPart::LengthUnit,
            m::UnitPart::NamedUnit {
                dimensions: Some(m::DimensionalExponentsRef::DimensionalExponents(
                    m::DimensionalExponentsId(9),
                )),
            },
            m::UnitPart::SiUnit {
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

    // Minimal product chain up to the PRODUCT_DEFINITION_SHAPE.
    let ac = a.add_application_context("mim".to_owned()).expect("ac");
    let pc = a
        .add_product_context(
            String::new(),
            m::ApplicationContextRef::ApplicationContext(ac),
            "mechanical".to_owned(),
        )
        .expect("pc");
    let pdc = a
        .add_product_definition_context(
            "part definition".to_owned(),
            m::ApplicationContextRef::ApplicationContext(ac),
            "design".to_owned(),
        )
        .expect("pdc");
    let product = a
        .add_product(
            "P1".to_owned(),
            "part".to_owned(),
            None,
            vec![m::ProductContextRef::ProductContext(pc)],
        )
        .expect("product");
    let pdf = a
        .add_product_definition_formation("1".to_owned(), None, m::ProductRef::Product(product))
        .expect("pdf");
    let pd = a
        .add_product_definition(
            "design".to_owned(),
            None,
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(pdf),
            m::ProductDefinitionContextRef::ProductDefinitionContext(pdc),
        )
        .expect("pd");
    let pds = a
        .add_product_definition_shape(
            String::new(),
            None,
            m::CharacterizedDefinitionRef::ProductDefinition(pd),
        )
        .expect("pds");

    // The stc07 pattern: DIMENSIONAL_SIZE_WITH_DATUM_FEATURE whose
    // applies_to references the entity itself (WR1).
    let dswdf = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: "datum feature".to_owned(),
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(pds),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: "diameter".to_owned(),
        })
        .expect("self-referencing DSWDF");
    assert_eq!(dswdf.0, 0);

    let text = a.finish();
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let items = &model.dimensional_size_with_datum_feature_arena.items;
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].applies_to,
        m::ShapeAspectRef::DimensionalSizeWithDatumFeature(m::DimensionalSizeWithDatumFeatureId(0)),
        "applies_to must reference the entity itself after the round trip"
    );
}

#[test]
fn cyclic_constructor_rolls_back_on_dangling_reference() {
    let mut a = Ap242Author::new();
    // A genuinely dangling forward reference (id + 1) must be rejected and
    // the insertion rolled back — model unchanged.
    let err = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: String::new(),
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(
                m::DimensionalSizeWithDatumFeatureId(id.0 + 1),
            ),
            name_1: String::new(),
        })
        .expect_err("forward dangling reference must be rejected");
    assert!(matches!(err, AuthorError::DanglingRef { .. }));

    // Rolled back: a retry with a valid self-reference gets the same id 0.
    let pds_missing = a
        .add_dimensional_size_with_datum_feature_cyclic(|id| m::DimensionalSizeWithDatumFeature {
            name: String::new(),
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: String::new(),
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
    let seen = std::cell::Cell::new(usize::MAX);
    let _ = a.add_dimensional_size_with_datum_feature_cyclic(|id| {
        seen.set(id.0);
        m::DimensionalSizeWithDatumFeature {
            name: String::new(),
            description: None,
            of_shape: m::ProductDefinitionShapeRef::ProductDefinitionShape(
                m::ProductDefinitionShapeId(0),
            ),
            product_definitional: m::Logical::True,
            applies_to: m::ShapeAspectRef::DimensionalSizeWithDatumFeature(id),
            name_1: String::new(),
        }
    });
    assert_eq!(seen.get(), 0, "both rolled-back insertions must be popped");
}

#[test]
fn aggregate_cardinality_is_enforced() {
    let mut a = Ap242Author::new();

    // DIRECTION.direction_ratios is LIST [2:3].
    let err = a
        .add_direction(String::new(), vec![1.0])
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
    let err = a
        .add_direction(String::new(), vec![1.0, 0.0, 0.0, 0.0])
        .expect_err("four ratios exceed the maximum");
    assert!(matches!(err, AuthorError::Cardinality { got: 4, .. }));
    a.add_direction(String::new(), vec![0.0, 0.0, 1.0])
        .expect("three ratios are fine");

    // Complex parts get the same guard: GLOBAL_UNIT_ASSIGNED_CONTEXT.units
    // is SET [1:?].
    let err = a
        .add_complex(vec![
            m::UnitPart::GeometricRepresentationContext {
                coordinate_space_dimension: 3,
            },
            m::UnitPart::GlobalUnitAssignedContext { units: vec![] },
            m::UnitPart::RepresentationContext {
                context_identifier: String::new(),
                context_type: "3D".to_owned(),
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
