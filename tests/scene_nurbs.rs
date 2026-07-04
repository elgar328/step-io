//! Opt-in NURBS conversion (`curve.to_nurbs()`): analytic curves (circle,
//! ellipse) become rational B-splines; a B-spline with knots passes through;
//! unsupported kinds return `None`. Curves are reached through anchored
//! `EDGE_CURVE`s (there is no standalone curve enumerator).

use step_io::read;
use step_io::scene::geometry::{Curve, CurveKind};
use step_io::scene::{NurbsCurve, Scene};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

// Four curves — circle, ellipse, B-spline-with-knots, line — each anchored as an
// EDGE_CURVE in one solid so `all_edges()` reaches them (topology is not
// enforced on read, so the edges may share vertices).
const CURVES: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(5.,0.,0.));\n\
#3=DIRECTION('',(0.,0.,1.));\n\
#4=DIRECTION('',(1.,0.,0.));\n\
#5=AXIS2_PLACEMENT_3D('',#1,#3,#4);\n\
#6=CARTESIAN_POINT('',(2.,0.,0.));\n\
#7=AXIS2_PLACEMENT_3D('',#6,#3,#4);\n\
#10=CIRCLE('',#5,1.0);\n\
#11=ELLIPSE('',#5,2.0,1.0);\n\
#12=CARTESIAN_POINT('',(0.,0.,0.));\n\
#13=CARTESIAN_POINT('',(1.,0.,0.));\n\
#14=CARTESIAN_POINT('',(1.,1.,0.));\n\
#15=B_SPLINE_CURVE_WITH_KNOTS('',1,(#12,#13,#14),.UNSPECIFIED.,.F.,.F.,(2,1,2),(0.,1.,2.),.UNSPECIFIED.);\n\
#16=VECTOR('',#4,1.0);\n\
#17=LINE('',#1,#16);\n\
#18=CIRCLE('',#7,3.0);\n\
#20=VERTEX_POINT('',#1);\n\
#21=VERTEX_POINT('',#2);\n\
#30=EDGE_CURVE('',#20,#21,#10,.T.);\n\
#31=EDGE_CURVE('',#20,#21,#11,.T.);\n\
#32=EDGE_CURVE('',#20,#21,#15,.T.);\n\
#33=EDGE_CURVE('',#20,#21,#17,.T.);\n\
#34=EDGE_CURVE('',#20,#21,#18,.T.);\n\
#60=CARTESIAN_POINT('',(0.,0.,0.));\n\
#61=CARTESIAN_POINT('',(1.,1.,0.));\n\
#62=CARTESIAN_POINT('',(2.,0.,0.));\n\
#63=(BOUNDED_CURVE()B_SPLINE_CURVE(2,(#60,#61,#62),.UNSPECIFIED.,.F.,.F.)B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE((1.,0.5,1.))REPRESENTATION_ITEM(''));\n\
#64=EDGE_CURVE('',#20,#21,#63,.T.);\n\
#70=TRIMMED_CURVE('',#17,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(5.)),.T.,.PARAMETER.);\n\
#71=CARTESIAN_POINT('',(1.,0.,0.));\n\
#72=CARTESIAN_POINT('',(0.,1.,0.));\n\
#73=CARTESIAN_POINT('',(0.,-1.,0.));\n\
#74=TRIMMED_CURVE('',#10,(#71),(#72),.T.,.CARTESIAN.);\n\
#75=TRIMMED_CURVE('',#10,(#71),(#73),.T.,.CARTESIAN.);\n\
#76=EDGE_CURVE('',#20,#21,#70,.T.);\n\
#77=EDGE_CURVE('',#20,#21,#74,.T.);\n\
#78=EDGE_CURVE('',#20,#21,#75,.T.);\n\
#83=TRIMMED_CURVE('',#10,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(3.141592653589793)),.T.,.PARAMETER.);\n\
#84=EDGE_CURVE('',#20,#21,#83,.T.);\n\
#40=ORIENTED_EDGE('',*,*,#30,.T.);\n\
#41=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#42=ORIENTED_EDGE('',*,*,#32,.T.);\n\
#43=ORIENTED_EDGE('',*,*,#33,.T.);\n\
#44=ORIENTED_EDGE('',*,*,#34,.T.);\n\
#65=ORIENTED_EDGE('',*,*,#64,.T.);\n\
#80=ORIENTED_EDGE('',*,*,#76,.T.);\n\
#81=ORIENTED_EDGE('',*,*,#77,.T.);\n\
#82=ORIENTED_EDGE('',*,*,#78,.T.);\n\
#85=ORIENTED_EDGE('',*,*,#84,.T.);\n\
#45=EDGE_LOOP('',(#40,#41,#42,#43,#44,#65,#80,#81,#82,#85));\n\
#46=FACE_OUTER_BOUND('',#45,.T.);\n\
#47=PLANE('',#5);\n\
#48=ADVANCED_FACE('',(#46),#47,.T.);\n\
#49=CLOSED_SHELL('',(#48));\n\
#50=MANIFOLD_SOLID_BREP('',#49);\n";

fn approx(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}

fn cp_flat(n: &NurbsCurve) -> Vec<f64> {
    n.control_points.iter().flatten().copied().collect()
}

fn find<'m>(scene: &'m Scene<'m>, pred: impl Fn(&Curve<'m>) -> bool) -> Curve<'m> {
    scene
        .all_edges()
        .map(|e| e.curve())
        .find(pred)
        .expect("a matching curve")
}

#[test]
fn circle_becomes_rational_quadratic() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The unit circle at the origin (radius 1).
    let circle = find(
        &scene,
        |c| matches!(c.kind(), CurveKind::Circle(cc) if (cc.radius - 1.0).abs() < 1e-9),
    );
    let n = circle.to_nurbs().expect("circle → nurbs");

    assert_eq!(n.degree, 2);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!(approx(&n.weights, &[1., w, 1., w, 1., w, 1., w, 1.]));
    assert!(approx(
        &n.knots,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    ));
    // On-circle points at 0°, 90°, 180°, 270°, plus a 45° corner control point.
    assert!(approx(
        &cp_flat(&n),
        &[
            1., 0., 0., // 0°
            1., 1., 0., // corner
            0., 1., 0., // 90°
            -1., 1., 0., // corner
            -1., 0., 0., // 180°
            -1., -1., 0., //
            0., -1., 0., // 270°
            1., -1., 0., //
            1., 0., 0., // back to 0°
        ]
    ));
}

#[test]
fn circle_scales_and_translates() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Radius 3, centred at (2,0,0).
    let circle = find(
        &scene,
        |c| matches!(c.kind(), CurveKind::Circle(cc) if (cc.radius - 3.0).abs() < 1e-9),
    );
    let n = circle.to_nurbs().expect("circle → nurbs");
    assert!(approx(&n.control_points[0], &[5., 0., 0.])); // 2 + 3
    assert!(approx(&n.control_points[2], &[2., 3., 0.])); // centre + 3·y
    assert!(approx(&n.control_points[4], &[-1., 0., 0.])); // 2 − 3
}

#[test]
fn ellipse_scales_axes() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let ellipse = find(&scene, |c| matches!(c.kind(), CurveKind::Ellipse(_)));
    let n = ellipse.to_nurbs().expect("ellipse → nurbs");
    // semi_axis_1 = 2 (x), semi_axis_2 = 1 (y).
    assert!(approx(&n.control_points[0], &[2., 0., 0.]));
    assert!(approx(&n.control_points[1], &[2., 1., 0.])); // corner: 2·x + 1·y
    assert!(approx(&n.control_points[2], &[0., 1., 0.]));
}

#[test]
fn bspline_with_knots_passes_through() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let bs = find(&scene, |c| {
        matches!(c.kind(), CurveKind::BSplineWithKnots(_))
    });
    let n = bs.to_nurbs().expect("bspline → nurbs");
    assert_eq!(n.degree, 1);
    assert!(approx(&n.weights, &[1., 1., 1.]));
    assert!(approx(&n.knots, &[0., 0., 1., 2., 2.])); // expanded from mult (2,1,2)
    assert!(approx(&cp_flat(&n), &[0., 0., 0., 1., 0., 0., 1., 1., 0.]));
}

#[test]
fn complex_rational_bspline_passes_through() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The rational B-spline arrives as a complex instance (kind() = Other).
    let bs = find(
        &scene,
        |c| matches!(c.kind(), CurveKind::Other(k) if k == "COMPLEX"),
    );
    let n = bs.to_nurbs().expect("complex rational bspline → nurbs");
    assert_eq!(n.degree, 2);
    // Weights come from the RATIONAL_B_SPLINE_CURVE part.
    assert!(approx(&n.weights, &[1., 0.5, 1.]));
    assert!(approx(&n.knots, &[0., 0., 0., 1., 1., 1.])); // mult (3,3)
    assert!(approx(&cp_flat(&n), &[0., 0., 0., 1., 1., 0., 2., 0., 0.]));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn reads_trimmed_curves() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let trimmed: Vec<NurbsCurve> = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::Trimmed(_)))
        .filter_map(|c| c.to_nurbs())
        .collect();

    // Trimmed line → degree-1 segment from param 0 to 5 along +x.
    let seg = trimmed.iter().find(|n| n.degree == 1).expect("segment");
    assert!(approx(&cp_flat(seg), &[0., 0., 0., 5., 0., 0.]));
    assert!(approx(&seg.knots, &[0., 0., 1., 1.]));

    // 90° circle arc → one degree-2 segment (3 control points).
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let a90 = trimmed
        .iter()
        .find(|n| n.degree == 2 && n.control_points.len() == 3)
        .expect("90 arc");
    assert!(approx(&cp_flat(a90), &[1., 0., 0., 1., 1., 0., 0., 1., 0.]));
    assert!(approx(&a90.weights, &[1., w, 1.]));
    assert!(approx(&a90.knots, &[0., 0., 0., 1., 1., 1.]));

    // 270° arc → three segments (7 control points), from (1,0,0) to (0,-1,0).
    let a270 = trimmed
        .iter()
        .find(|n| n.degree == 2 && n.control_points.len() == 7)
        .expect("270 arc");
    assert!(approx(
        &a270.knots,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 3.]
    ));
    assert!(approx(&a270.control_points[0], &[1., 0., 0.]));
    assert!(approx(&a270.control_points[6], &[0., -1., 0.]));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn parameter_value_arc_in_radians() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The 180° arc trimmed by PARAMETER_VALUE (0, π); CURVES has no unit context,
    // so the parameter is read as radians (factor 1.0). Two segments → 5 CP.
    let arc = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::Trimmed(_)))
        .filter_map(|c| c.to_nurbs())
        .find(|n| n.degree == 2 && n.control_points.len() == 5)
        .expect("180 arc from param trims");
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!(approx(&arc.weights, &[1., w, 1., w, 1.]));
    assert!(approx(&arc.knots, &[0., 0., 0., 1., 1., 2., 2., 2.]));
    assert!(approx(&arc.control_points[0], &[1., 0., 0.]));
    assert!(approx(&arc.control_points[2], &[0., 1., 0.]));
    assert!(approx(&arc.control_points[4], &[-1., 0., 0.]));
}

// A degree plane-angle unit context + a circle arc trimmed by PARAMETER_VALUE
// (0, 90) — the parameter is degrees, converted to radians via the unit's factor.
const DEGREE_ARC: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=CIRCLE('',#4,1.0);\n\
#6=TRIMMED_CURVE('',#5,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(90.)),.T.,.PARAMETER.);\n\
#7=VERTEX_POINT('',#1);\n\
#8=EDGE_CURVE('',#7,#7,#6,.T.);\n\
#9=ORIENTED_EDGE('',*,*,#8,.T.);\n\
#10=EDGE_LOOP('',(#9));\n\
#11=FACE_OUTER_BOUND('',#10,.T.);\n\
#12=PLANE('',#4);\n\
#13=ADVANCED_FACE('',(#11),#12,.T.);\n\
#14=CLOSED_SHELL('',(#13));\n\
#15=MANIFOLD_SOLID_BREP('',#14);\n\
#20=( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
#21=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#20);\n\
#22=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
#23=( CONVERSION_BASED_UNIT('DEGREE',#21) NAMED_UNIT(#22) PLANE_ANGLE_UNIT() );\n\
#24=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#25=( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNIT_ASSIGNED_CONTEXT((#24,#23)) REPRESENTATION_CONTEXT('','') );\n\
#26=SHAPE_REPRESENTATION('',(),#25);\n";

#[test]
fn parameter_value_arc_in_degrees() {
    let src = format!("{HEADER}{DEGREE_ARC}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Param (0, 90) in degrees → a 90° arc once converted to radians.
    let arc = scene
        .all_edges()
        .map(|e| e.curve())
        .filter_map(|c| c.to_nurbs())
        .find(|n| n.degree == 2)
        .expect("arc from degree param trims");
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert_eq!(arc.control_points.len(), 3);
    assert!(approx(&arc.weights, &[1., w, 1.]));
    assert!(approx(
        &cp_flat(&arc),
        &[1., 0., 0., 1., 1., 0., 0., 1., 0.]
    ));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn unsupported_curve_returns_none() {
    let src = format!("{HEADER}{CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let line = find(&scene, |c| matches!(c.kind(), CurveKind::Line(_)));
    assert!(line.to_nurbs().is_none(), "an unbounded line has no NURBS");
    assert!(
        scene.warnings().is_empty(),
        "unsupported kind is silent: {:?}",
        scene.warnings()
    );
}

// The uniform-knot family — subtypes that store no knots (a standard rule
// derives them) — plus a polyline, each anchored as an EDGE_CURVE. #90 is the
// corpus form of a rational quasi-uniform complex instance (no WITH_KNOTS
// part; the QUASI_UNIFORM_CURVE marker implies the knots).
const UNIFORM_FAMILY: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,2.,0.));\n\
#3=CARTESIAN_POINT('',(2.,2.,0.));\n\
#4=CARTESIAN_POINT('',(3.,0.,0.));\n\
#5=CARTESIAN_POINT('',(4.,1.,0.));\n\
#10=QUASI_UNIFORM_CURVE('',2,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.F.);\n\
#11=UNIFORM_CURVE('',2,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.F.);\n\
#12=BEZIER_CURVE('',2,(#1,#2,#3,#4,#5),.UNSPECIFIED.,.F.,.F.);\n\
#13=BEZIER_CURVE('',2,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.F.);\n\
#14=POLYLINE('',(#1,#2,#3));\n\
#90=(BOUNDED_CURVE()B_SPLINE_CURVE(3,(#1,#2,#3,#4),.UNSPECIFIED.,.F.,.U.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()QUASI_UNIFORM_CURVE()RATIONAL_B_SPLINE_CURVE((1.,0.9,1.1,1.))REPRESENTATION_ITEM(''));\n\
#20=VERTEX_POINT('',#1);\n\
#21=VERTEX_POINT('',#4);\n\
#30=EDGE_CURVE('',#20,#21,#10,.T.);\n\
#31=EDGE_CURVE('',#20,#21,#11,.T.);\n\
#32=EDGE_CURVE('',#20,#21,#12,.T.);\n\
#33=EDGE_CURVE('',#20,#21,#13,.T.);\n\
#34=EDGE_CURVE('',#20,#21,#14,.T.);\n\
#35=EDGE_CURVE('',#20,#21,#90,.T.);\n\
#40=ORIENTED_EDGE('',*,*,#30,.T.);\n\
#41=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#42=ORIENTED_EDGE('',*,*,#32,.T.);\n\
#43=ORIENTED_EDGE('',*,*,#33,.T.);\n\
#44=ORIENTED_EDGE('',*,*,#34,.T.);\n\
#45=ORIENTED_EDGE('',*,*,#35,.T.);\n\
#46=EDGE_LOOP('',(#40,#41,#42,#43,#44,#45));\n\
#47=FACE_OUTER_BOUND('',#46,.T.);\n\
#50=DIRECTION('',(0.,0.,1.));\n\
#51=DIRECTION('',(1.,0.,0.));\n\
#52=AXIS2_PLACEMENT_3D('',#1,#50,#51);\n\
#53=PLANE('',#52);\n\
#54=ADVANCED_FACE('',(#47),#53,.T.);\n\
#55=CLOSED_SHELL('',(#54));\n\
#56=MANIFOLD_SOLID_BREP('',#55);\n";

#[test]
fn uniform_family_curves_derive_their_knots() {
    let src = format!("{HEADER}{UNIFORM_FAMILY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let quasi = find(&scene, |c| matches!(c.kind(), CurveKind::QuasiUniform(_)));
    let n = quasi.to_nurbs().expect("quasi-uniform → nurbs");
    assert_eq!(n.degree, 2);
    assert!(approx(&n.knots, &[0., 0., 0., 1., 2., 2., 2.])); // clamped
    assert!(approx(&n.control_points[0], &[0., 0., 0.]));
    assert!(approx(&n.weights, &[1.; 4]));

    let uniform = find(&scene, |c| matches!(c.kind(), CurveKind::Uniform(_)));
    let n = uniform.to_nurbs().expect("uniform → nurbs");
    assert!(approx(&n.knots, &[0., 1., 2., 3., 4., 5., 6.])); // unclamped

    // Two Bézier curves: 5 CPs (two exact segments) converts, 4 CPs violates
    // n = k·degree + 1 and is refused with a warning.
    let beziers: Vec<Option<NurbsCurve>> = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::Bezier(_)))
        .map(|c| c.to_nurbs())
        .collect();
    assert_eq!(beziers.len(), 2);
    let converted: Vec<&NurbsCurve> = beziers.iter().flatten().collect();
    assert_eq!(converted.len(), 1);
    assert!(approx(
        &converted[0].knots,
        &[0., 0., 0., 1., 1., 2., 2., 2.]
    ));
    assert_eq!(converted[0].control_points.len(), 5);
    assert_eq!(scene.warnings().len(), 1, "one bezier violation warning");
    assert!(scene.warnings()[0].contains("knot family"));
}

#[test]
fn polyline_becomes_degree_one_bspline() {
    let src = format!("{HEADER}{UNIFORM_FAMILY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let poly = find(
        &scene,
        |c| matches!(c.kind(), CurveKind::Polyline(p) if p.points.len() == 3),
    );
    let n = poly.to_nurbs().expect("polyline → nurbs");
    assert_eq!(n.degree, 1);
    assert!(approx(&n.knots, &[0., 0., 1., 2., 2.]));
    assert!(approx(&cp_flat(&n), &[0., 0., 0., 1., 2., 0., 2., 2., 0.]));
    assert!(approx(&n.weights, &[1.; 3]));
    assert!(scene.warnings().is_empty());
}

#[test]
fn complex_quasi_uniform_marker_implies_knots() {
    let src = format!("{HEADER}{UNIFORM_FAMILY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The corpus form: rational + QUASI_UNIFORM_CURVE marker, no WITH_KNOTS.
    let complex = find(&scene, |c| matches!(c.kind(), CurveKind::Other("COMPLEX")));
    let n = complex.to_nurbs().expect("complex quasi-uniform → nurbs");
    assert_eq!(n.degree, 3);
    // degree 3 with 4 CPs → a single clamped span.
    assert!(approx(&n.knots, &[0., 0., 0., 0., 1., 1., 1., 1.]));
    assert!(approx(&n.weights, &[1., 0.9, 1.1, 1.]));
    assert!(scene.warnings().is_empty());
}

// Trimmed curves over B-spline bases. #110 is the full unit circle written as
// a complex rational B-spline (the 9-control-point form); trimming it between
// two CARTESIAN_POINTs must reproduce the analytic 90° arc — one oracle for
// the whole path (complex basis + rational point inversion + knot insertion).
const TRIMMED_BSPLINE: &str = "\
#1=CARTESIAN_POINT('',(1.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,1.,0.));\n\
#3=CARTESIAN_POINT('',(0.,1.,0.));\n\
#4=CARTESIAN_POINT('',(-1.,1.,0.));\n\
#5=CARTESIAN_POINT('',(-1.,0.,0.));\n\
#6=CARTESIAN_POINT('',(-1.,-1.,0.));\n\
#7=CARTESIAN_POINT('',(0.,-1.,0.));\n\
#8=CARTESIAN_POINT('',(1.,-1.,0.));\n\
#110=(BOUNDED_CURVE()B_SPLINE_CURVE(2,(#1,#2,#3,#4,#5,#6,#7,#8,#1),.UNSPECIFIED.,.F.,.F.)B_SPLINE_CURVE_WITH_KNOTS((3,2,2,2,3),(0.,1.,2.,3.,4.),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE((1.,0.70710678118654757,1.,0.70710678118654757,1.,0.70710678118654757,1.,0.70710678118654757,1.))REPRESENTATION_ITEM(''));\n\
#111=TRIMMED_CURVE('',#110,(#1),(#3),.T.,.CARTESIAN.);\n\
#120=CARTESIAN_POINT('',(0.,0.,0.));\n\
#121=CARTESIAN_POINT('',(1.,1.,0.));\n\
#122=CARTESIAN_POINT('',(2.,0.,0.));\n\
#123=B_SPLINE_CURVE_WITH_KNOTS('',1,(#120,#121,#122),.UNSPECIFIED.,.F.,.F.,(2,1,2),(0.,1.,2.),.UNSPECIFIED.);\n\
#124=CARTESIAN_POINT('',(0.5,0.5,0.));\n\
#125=CARTESIAN_POINT('',(1.5,0.5,0.));\n\
#126=TRIMMED_CURVE('',#123,(#124),(#125),.T.,.CARTESIAN.);\n\
#127=TRIMMED_CURVE('',#123,(#125),(#124),.F.,.CARTESIAN.);\n\
#128=TRIMMED_CURVE('',#123,(PARAMETER_VALUE(0.5)),(PARAMETER_VALUE(1.5)),.T.,.PARAMETER.);\n\
#20=VERTEX_POINT('',#1);\n\
#21=VERTEX_POINT('',#3);\n\
#30=EDGE_CURVE('',#20,#21,#111,.T.);\n\
#31=EDGE_CURVE('',#20,#21,#126,.T.);\n\
#32=EDGE_CURVE('',#20,#21,#127,.T.);\n\
#33=EDGE_CURVE('',#20,#21,#128,.T.);\n\
#40=ORIENTED_EDGE('',*,*,#30,.T.);\n\
#41=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#42=ORIENTED_EDGE('',*,*,#32,.T.);\n\
#43=ORIENTED_EDGE('',*,*,#33,.T.);\n\
#44=EDGE_LOOP('',(#40,#41,#42,#43));\n\
#45=FACE_OUTER_BOUND('',#44,.T.);\n\
#50=DIRECTION('',(0.,0.,1.));\n\
#51=DIRECTION('',(1.,0.,0.));\n\
#52=AXIS2_PLACEMENT_3D('',#120,#50,#51);\n\
#53=PLANE('',#52);\n\
#54=ADVANCED_FACE('',(#45),#53,.T.);\n\
#55=CLOSED_SHELL('',(#54));\n\
#56=MANIFOLD_SOLID_BREP('',#55);\n";

#[test]
fn trimmed_bspline_bases_extract_exact_segments() {
    let src = format!("{HEADER}{TRIMMED_BSPLINE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let trimmed: Vec<NurbsCurve> = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::Trimmed(_)))
        .filter_map(|c| c.to_nurbs())
        .collect();
    assert_eq!(trimmed.len(), 4);

    // Oracle: the complex rational circle trimmed (1,0,0) → (0,1,0) must
    // reproduce the analytic 90° arc (rational inversion + knot insertion).
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let arc = trimmed.iter().find(|n| n.degree == 2).expect("the arc");
    assert_eq!(arc.control_points.len(), 3);
    assert!(approx(&cp_flat(arc), &[1., 0., 0., 1., 1., 0., 0., 1., 0.]));
    assert!(approx(&arc.weights, &[1., w, 1.]));
    assert!(approx(&arc.knots, &[0., 0., 0., 1., 1., 1.]));

    // The degree-1 basis cut mid-span on both sides: CARTESIAN trims and
    // PARAMETER trims give the same segment; the .F. one is reversed.
    let segs: Vec<&NurbsCurve> = trimmed.iter().filter(|n| n.degree == 1).collect();
    assert_eq!(segs.len(), 3);
    let forward: Vec<_> = segs
        .iter()
        .filter(|n| approx(&n.control_points[0], &[0.5, 0.5, 0.]))
        .collect();
    assert_eq!(forward.len(), 2, "cartesian + parameter trims agree");
    assert!(approx(
        &cp_flat(forward[0]),
        &[0.5, 0.5, 0., 1., 1., 0., 1.5, 0.5, 0.]
    ));
    assert!(approx(&forward[0].knots, &[0.5, 0.5, 1., 1.5, 1.5]));
    let reversed = segs
        .iter()
        .find(|n| approx(&n.control_points[0], &[1.5, 0.5, 0.]))
        .expect("the .F. trim runs from trim_1 backwards");
    assert!(approx(
        &cp_flat(reversed),
        &[1.5, 0.5, 0., 1., 1., 0., 0.5, 0.5, 0.]
    ));
    assert!(approx(&reversed.knots, &[0.5, 0.5, 1., 1.5, 1.5]));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// A trimmed B-spline with empty trim lists (a corpus-observed nonstandard
// form) — no usable trim, so the conversion refuses with a warning.
const TRIMMED_EMPTY: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,1.,0.));\n\
#3=CARTESIAN_POINT('',(2.,0.,0.));\n\
#4=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.,(2,1,2),(0.,1.,2.),.UNSPECIFIED.);\n\
#5=TRIMMED_CURVE('',#4,(),(),.T.,.UNSPECIFIED.);\n\
#20=VERTEX_POINT('',#1);\n\
#21=VERTEX_POINT('',#3);\n\
#30=EDGE_CURVE('',#20,#21,#5,.T.);\n\
#40=ORIENTED_EDGE('',*,*,#30,.T.);\n\
#44=EDGE_LOOP('',(#40));\n\
#45=FACE_OUTER_BOUND('',#44,.T.);\n\
#50=DIRECTION('',(0.,0.,1.));\n\
#51=DIRECTION('',(1.,0.,0.));\n\
#52=AXIS2_PLACEMENT_3D('',#1,#50,#51);\n\
#53=PLANE('',#52);\n\
#54=ADVANCED_FACE('',(#45),#53,.T.);\n\
#55=CLOSED_SHELL('',(#54));\n\
#56=MANIFOLD_SOLID_BREP('',#55);\n";

#[test]
fn trimmed_bspline_with_empty_trims_warns() {
    let src = format!("{HEADER}{TRIMMED_EMPTY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let trimmed = find(&scene, |c| matches!(c.kind(), CurveKind::Trimmed(_)));
    assert!(trimmed.to_nurbs().is_none());
    assert_eq!(scene.warnings().len(), 1);
    assert!(scene.warnings()[0].contains("no usable trim"));
}

// Composite curves: a line chain, a mixed arc+line chain (degree elevation),
// a reversed segment, and a disjoint (non-joining) chain. The last segment's
// .DISCONTINUOUS. transition just marks the curve open.
const COMPOSITES: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(1.,0.,0.));\n\
#3=DIRECTION('',(0.,1.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=VECTOR('',#2,1.0);\n\
#6=VECTOR('',#3,1.0);\n\
#7=VECTOR('',#4,1.0);\n\
#10=LINE('',#1,#5);\n\
#11=CARTESIAN_POINT('',(1.,0.,0.));\n\
#12=LINE('',#11,#6);\n\
#13=TRIMMED_CURVE('',#10,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(1.)),.T.,.PARAMETER.);\n\
#14=TRIMMED_CURVE('',#12,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(1.)),.T.,.PARAMETER.);\n\
#15=COMPOSITE_CURVE_SEGMENT(.CONTINUOUS.,.T.,#13);\n\
#16=COMPOSITE_CURVE_SEGMENT(.DISCONTINUOUS.,.T.,#14);\n\
#17=COMPOSITE_CURVE('',(#15,#16),.F.);\n\
#20=AXIS2_PLACEMENT_3D('',#1,#4,#2);\n\
#21=CIRCLE('',#20,1.0);\n\
#22=CARTESIAN_POINT('',(0.,1.,0.));\n\
#23=TRIMMED_CURVE('',#21,(#11),(#22),.T.,.CARTESIAN.);\n\
#24=LINE('',#22,#7);\n\
#25=TRIMMED_CURVE('',#24,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(2.)),.T.,.PARAMETER.);\n\
#26=COMPOSITE_CURVE_SEGMENT(.CONTINUOUS.,.T.,#23);\n\
#27=COMPOSITE_CURVE_SEGMENT(.DISCONTINUOUS.,.T.,#25);\n\
#28=COMPOSITE_CURVE('',(#26,#27),.F.);\n\
#30=CARTESIAN_POINT('',(1.,1.,0.));\n\
#31=LINE('',#30,#6);\n\
#32=TRIMMED_CURVE('',#31,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(-1.)),.F.,.PARAMETER.);\n\
#33=COMPOSITE_CURVE_SEGMENT(.CONTINUOUS.,.T.,#13);\n\
#34=COMPOSITE_CURVE_SEGMENT(.DISCONTINUOUS.,.F.,#32);\n\
#35=COMPOSITE_CURVE('',(#33,#34),.F.);\n\
#40=CARTESIAN_POINT('',(5.,5.,5.));\n\
#41=LINE('',#40,#5);\n\
#42=TRIMMED_CURVE('',#41,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(1.)),.T.,.PARAMETER.);\n\
#43=COMPOSITE_CURVE_SEGMENT(.CONTINUOUS.,.T.,#13);\n\
#44=COMPOSITE_CURVE_SEGMENT(.DISCONTINUOUS.,.T.,#42);\n\
#45=COMPOSITE_CURVE('',(#43,#44),.F.);\n\
#50=VERTEX_POINT('',#1);\n\
#51=VERTEX_POINT('',#30);\n\
#60=EDGE_CURVE('',#50,#51,#17,.T.);\n\
#61=EDGE_CURVE('',#50,#51,#28,.T.);\n\
#62=EDGE_CURVE('',#50,#51,#35,.T.);\n\
#63=EDGE_CURVE('',#50,#51,#45,.T.);\n\
#70=ORIENTED_EDGE('',*,*,#60,.T.);\n\
#71=ORIENTED_EDGE('',*,*,#61,.T.);\n\
#72=ORIENTED_EDGE('',*,*,#62,.T.);\n\
#73=ORIENTED_EDGE('',*,*,#63,.T.);\n\
#74=EDGE_LOOP('',(#70,#71,#72,#73));\n\
#75=FACE_OUTER_BOUND('',#74,.T.);\n\
#76=PLANE('',#20);\n\
#77=ADVANCED_FACE('',(#75),#76,.T.);\n\
#78=CLOSED_SHELL('',(#77));\n\
#79=MANIFOLD_SOLID_BREP('',#78);\n";

#[test]
fn composite_curves_join_into_one_nurbs() {
    let src = format!("{HEADER}{COMPOSITES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let composites: Vec<Option<NurbsCurve>> = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::Composite(_)))
        .map(|c| c.to_nurbs())
        .collect();
    assert_eq!(composites.len(), 4);
    let joined: Vec<&NurbsCurve> = composites.iter().flatten().collect();
    assert_eq!(joined.len(), 3, "the disjoint chain refuses");
    assert_eq!(scene.warnings().len(), 1);
    assert!(scene.warnings()[0].contains("do not join"));

    // Two line chains (one with a reversed .F. segment) meet at (1,0,0) and
    // end at (1,1,0): degree 1, knots over two unit intervals.
    let chains: Vec<&&NurbsCurve> = joined.iter().filter(|n| n.degree == 1).collect();
    assert_eq!(chains.len(), 2);
    for chain in chains {
        assert!(approx(
            &cp_flat(chain),
            &[0., 0., 0., 1., 0., 0., 1., 1., 0.]
        ));
        assert!(approx(&chain.knots, &[0., 0., 1., 2., 2.]));
        assert!(approx(&chain.weights, &[1., 1., 1.]));
    }

    // The mixed chain: a 90° arc then a line, elevated to degree 2. The line
    // gains its midpoint as a control point; the arc weights survive.
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let mixed = joined.iter().find(|n| n.degree == 2).expect("mixed chain");
    assert_eq!(mixed.control_points.len(), 5);
    assert!(approx(
        &cp_flat(mixed),
        &[
            1., 0., 0., // arc start
            1., 1., 0., // arc corner
            0., 1., 0., // junction
            0., 1., 1., // elevated line midpoint
            0., 1., 2., // line end
        ]
    ));
    assert!(approx(&mixed.weights, &[1., w, 1., 1., 1.]));
    assert!(approx(&mixed.knots, &[0., 0., 0., 1., 1., 2., 2., 2.]));
}

// Surface-curve containers (a 3D curve paired with its on-surface pcurve
// representations): to_nurbs() delegates to the 3D curve. A bare-LINE
// curve_3d stays unbounded → None, without a warning.
const SURFACE_CURVES: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=PLANE('',#4);\n\
#6=CIRCLE('',#4,1.0);\n\
#7=VECTOR('',#3,1.0);\n\
#8=LINE('',#1,#7);\n\
#9=TRIMMED_CURVE('',#8,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(5.)),.T.,.PARAMETER.);\n\
#10=CARTESIAN_POINT('',(0.,1.,0.));\n\
#11=CARTESIAN_POINT('',(1.,1.,0.));\n\
#12=B_SPLINE_CURVE_WITH_KNOTS('',1,(#10,#11),.UNSPECIFIED.,.F.,.F.,(2,2),(0.,1.),.UNSPECIFIED.);\n\
#20=SURFACE_CURVE('',#6,(#5),.CURVE_3D.);\n\
#21=SEAM_CURVE('',#9,(#5),.CURVE_3D.);\n\
#22=INTERSECTION_CURVE('',#12,(#5),.CURVE_3D.);\n\
#23=SURFACE_CURVE('',#8,(#5),.CURVE_3D.);\n\
#30=VERTEX_POINT('',#1);\n\
#31=VERTEX_POINT('',#10);\n\
#40=EDGE_CURVE('',#30,#31,#20,.T.);\n\
#41=EDGE_CURVE('',#30,#31,#21,.T.);\n\
#42=EDGE_CURVE('',#30,#31,#22,.T.);\n\
#43=EDGE_CURVE('',#30,#31,#23,.T.);\n\
#50=ORIENTED_EDGE('',*,*,#40,.T.);\n\
#51=ORIENTED_EDGE('',*,*,#41,.T.);\n\
#52=ORIENTED_EDGE('',*,*,#42,.T.);\n\
#53=ORIENTED_EDGE('',*,*,#43,.T.);\n\
#54=EDGE_LOOP('',(#50,#51,#52,#53));\n\
#55=FACE_OUTER_BOUND('',#54,.T.);\n\
#56=ADVANCED_FACE('',(#55),#5,.T.);\n\
#57=CLOSED_SHELL('',(#56));\n\
#58=MANIFOLD_SOLID_BREP('',#57);\n";

#[test]
fn surface_curves_delegate_to_their_3d_curve() {
    let src = format!("{HEADER}{SURFACE_CURVES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // SURFACE_CURVE over a full circle → the whole rational circle.
    let sc = find(&scene, |c| matches!(c.kind(), CurveKind::SurfaceCurve(_)));
    let n = sc.to_nurbs().expect("surface curve → nurbs");
    assert_eq!(n.degree, 2);
    assert_eq!(n.control_points.len(), 9);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!((n.weights[1] - w).abs() < 1e-9);

    // SEAM_CURVE over a trimmed line → a degree-1 segment.
    let seam = find(&scene, |c| matches!(c.kind(), CurveKind::SeamCurve(_)));
    let n = seam.to_nurbs().expect("seam curve → nurbs");
    assert_eq!(n.degree, 1);
    assert!(approx(&cp_flat(&n), &[0., 0., 0., 5., 0., 0.]));

    // INTERSECTION_CURVE over a B-spline → the passthrough.
    let ic = find(&scene, |c| {
        matches!(c.kind(), CurveKind::IntersectionCurve(_))
    });
    let n = ic.to_nurbs().expect("intersection curve → nurbs");
    assert!(approx(&cp_flat(&n), &[0., 1., 0., 1., 1., 0.]));

    // A bare-LINE curve_3d is unbounded: None, silently.
    let line_wrapped: Vec<Option<NurbsCurve>> = scene
        .all_edges()
        .map(|e| e.curve())
        .filter(|c| matches!(c.kind(), CurveKind::SurfaceCurve(_)))
        .map(|c| c.to_nurbs())
        .collect();
    assert_eq!(line_wrapped.len(), 2);
    assert_eq!(line_wrapped.iter().flatten().count(), 1);

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// Edge-level conversion: the geometry bounded between the edge's two vertex
// points, in the edge's direction. #40 line edge, #41/#42 the two arcs of one
// circle (same_sense picks which), #43 a closed edge (whole circle), #44 a
// SURFACE_CURVE-wrapped line, #45 a partial edge over a B-spline.
const EDGES: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=PLANE('',#4);\n\
#6=VECTOR('',#3,1.0);\n\
#7=LINE('',#1,#6);\n\
#8=CIRCLE('',#4,1.0);\n\
#9=CARTESIAN_POINT('',(3.,4.,0.));\n\
#10=CARTESIAN_POINT('',(1.,0.,0.));\n\
#11=CARTESIAN_POINT('',(0.,1.,0.));\n\
#12=SURFACE_CURVE('',#7,(#5),.CURVE_3D.);\n\
#13=CARTESIAN_POINT('',(0.,0.,0.));\n\
#14=CARTESIAN_POINT('',(1.,1.,0.));\n\
#15=CARTESIAN_POINT('',(2.,0.,0.));\n\
#16=B_SPLINE_CURVE_WITH_KNOTS('',1,(#13,#14,#15),.UNSPECIFIED.,.F.,.F.,(2,1,2),(0.,1.,2.),.UNSPECIFIED.);\n\
#17=CARTESIAN_POINT('',(0.5,0.5,0.));\n\
#18=CARTESIAN_POINT('',(1.5,0.5,0.));\n\
#20=VERTEX_POINT('',#1);\n\
#21=VERTEX_POINT('',#9);\n\
#22=VERTEX_POINT('',#10);\n\
#23=VERTEX_POINT('',#11);\n\
#24=VERTEX_POINT('',#17);\n\
#25=VERTEX_POINT('',#18);\n\
#40=EDGE_CURVE('',#20,#21,#7,.T.);\n\
#41=EDGE_CURVE('',#22,#23,#8,.T.);\n\
#42=EDGE_CURVE('',#22,#23,#8,.F.);\n\
#43=EDGE_CURVE('',#22,#22,#8,.T.);\n\
#44=EDGE_CURVE('',#20,#21,#12,.T.);\n\
#45=EDGE_CURVE('',#24,#25,#16,.T.);\n\
#50=ORIENTED_EDGE('',*,*,#40,.T.);\n\
#51=ORIENTED_EDGE('',*,*,#41,.T.);\n\
#52=ORIENTED_EDGE('',*,*,#42,.T.);\n\
#53=ORIENTED_EDGE('',*,*,#43,.T.);\n\
#54=ORIENTED_EDGE('',*,*,#44,.T.);\n\
#55=ORIENTED_EDGE('',*,*,#45,.T.);\n\
#56=EDGE_LOOP('',(#50,#51,#52,#53,#54,#55));\n\
#57=FACE_OUTER_BOUND('',#56,.T.);\n\
#58=ADVANCED_FACE('',(#57),#5,.T.);\n\
#59=CLOSED_SHELL('',(#58));\n\
#60=MANIFOLD_SOLID_BREP('',#59);\n";

#[test]
fn edges_convert_to_bounded_nurbs() {
    let src = format!("{HEADER}{EDGES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let results: Vec<NurbsCurve> = scene.all_edges().filter_map(|e| e.to_nurbs()).collect();
    assert_eq!(results.len(), 6, "all six edges convert");

    // The bare-LINE edge (whole-curve semantics would refuse it) becomes the
    // segment between its vertices; the wrapped one likewise.
    let segments: Vec<&NurbsCurve> = results
        .iter()
        .filter(|n| n.degree == 1 && n.control_points.len() == 2)
        .collect();
    assert_eq!(segments.len(), 2, "bare line + surface-curve line");
    for seg in segments {
        assert!(approx(&cp_flat(seg), &[0., 0., 0., 3., 4., 0.]));
    }

    let w = std::f64::consts::FRAC_1_SQRT_2;
    // same_sense=.T.: the 90° arc (1,0,0) → (0,1,0).
    let a90 = results
        .iter()
        .find(|n| n.degree == 2 && n.control_points.len() == 3)
        .expect("90 arc");
    assert!(approx(&cp_flat(a90), &[1., 0., 0., 1., 1., 0., 0., 1., 0.]));
    assert!((a90.weights[1] - w).abs() < 1e-9);
    // same_sense=.F.: the other way round — 270°, still start → end.
    let a270 = results
        .iter()
        .find(|n| n.degree == 2 && n.control_points.len() == 7)
        .expect("270 arc");
    assert!(approx(&a270.control_points[0], &[1., 0., 0.]));
    assert!(approx(&a270.control_points[6], &[0., 1., 0.]));
    assert!(approx(&a270.control_points[2], &[0., -1., 0.]));
    // The closed edge: the whole circle.
    let full = results
        .iter()
        .find(|n| n.degree == 2 && n.control_points.len() == 9)
        .expect("full circle");
    assert!(approx(&full.control_points[0], &[1., 0., 0.]));
    // The partial B-spline edge: the interior span between the two vertices.
    let partial = results
        .iter()
        .find(|n| n.degree == 1 && n.control_points.len() == 3)
        .expect("partial bspline");
    assert!(approx(
        &cp_flat(partial),
        &[0.5, 0.5, 0., 1., 1., 0., 1.5, 0.5, 0.]
    ));
    assert!(approx(&partial.knots, &[0.5, 0.5, 1., 1.5, 1.5]));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}
