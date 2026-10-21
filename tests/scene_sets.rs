//! Geometric sets: a part's wireframe as a `GEOMETRIC_CURVE_SET` and a
//! `GEOMETRIC_SET`, listed model-wide and reached from the part.

mod common;

use step_io::scene::{CurveKind, GeometricSet, SetElement};
use step_io::{StepBuilder, read};

// Curves, points and a placement, added to a part's shape representation.
const SETS: &str = "\
#9000 = CARTESIAN_POINT('',(0.,0.,0.));\n\
#9001 = CARTESIAN_POINT('',(1.,0.,0.));\n\
#9002 = CARTESIAN_POINT('',(1.,1.,0.));\n\
#9003 = POLYLINE('',(#9000,#9001,#9002));\n\
#9004 = DIRECTION('',(1.,0.,0.));\n\
#9005 = VECTOR('',#9004,1.);\n\
#9006 = LINE('',#9000,#9005);\n\
#9007 = TRIMMED_CURVE('',#9006,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(2.)),.T.,.PARAMETER.);\n\
#9008 = AXIS2_PLACEMENT_3D('',#9000,$,$);\n\
#9010 = GEOMETRIC_CURVE_SET('wires',(#9003,#9007,#9002));\n\
#9011 = GEOMETRIC_SET('marks',(#9008,#9001));\n";

fn part_with_sets() -> String {
    let mut b = StepBuilder::new();
    b.part("frame");
    let text = common::text(&b.finish());
    let rep = text
        .lines()
        .find(|l| l.contains(" = SHAPE_REPRESENTATION('',("))
        .expect("the part's shape representation")
        .to_owned();
    let with_sets = rep.replacen("('',(", "('',(#9010,#9011,", 1);
    text.replace(&rep, &format!("{with_sets}\n{SETS}"))
}

#[test]
fn geometric_sets_list_their_elements_and_reach_their_part() {
    let text = part_with_sets();
    let (model, report) = read(text.as_bytes()).expect("read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let scene = model.scene();

    let sets: Vec<_> = scene.all_geometric_sets().collect();
    let names: Vec<_> = sets.iter().map(GeometricSet::name).collect();
    assert_eq!(names, ["marks", "wires"]);

    let wires = sets[1];
    let elements: Vec<_> = wires.elements().collect();
    assert_eq!(elements.len(), 3);
    let SetElement::Curve(polyline) = elements[0] else {
        panic!("a curve, got {:?}", elements[0])
    };
    assert!(matches!(polyline.kind(), CurveKind::Polyline(_)));
    let SetElement::Curve(trimmed) = elements[1] else {
        panic!("a curve, got {:?}", elements[1])
    };
    let segment = trimmed.to_nurbs().expect("a trimmed line");
    assert_eq!(segment.control_points, vec![[0., 0., 0.], [2., 0., 0.]]);
    let SetElement::Point(point) = elements[2] else {
        panic!("a point, got {:?}", elements[2])
    };
    assert_eq!(point.xyz(), [1., 1., 0.]);

    // A placement is kept by key.
    let marks: Vec<_> = sets[0].elements().collect();
    assert!(matches!(
        marks[0],
        SetElement::Other(step_io::EntityKey::Axis2Placement3d(_))
    ));
    assert!(matches!(marks[1], SetElement::Point(_)));

    let def = scene.all_product_definitions().next().expect("the part");
    let reached: Vec<_> = def.geometric_sets().map(|s| s.name()).collect();
    assert_eq!(reached, ["wires", "marks"]);
    assert!(wires.color().is_none() && wires.is_visible());
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}
