//! Opt-in NURBS surface conversion (`surface.to_nurbs()`): a sphere and a torus
//! become rational surfaces of revolution; an (unbounded) plane returns `None`.
//! Surfaces are reached through anchored faces (`all_faces().surface()`).

use step_io::read;
use step_io::scene::{NurbsSurface, Scene};
use step_io::scene::{Surface, SurfaceKind};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

// Three faces — sphere, torus, plane — anchored in one solid so `all_faces()`
// reaches them (topology is not enforced on read; they share a dummy bound).
const SURFACES: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,0.,0.));\n\
#3=DIRECTION('',(0.,0.,1.));\n\
#4=DIRECTION('',(1.,0.,0.));\n\
#5=AXIS2_PLACEMENT_3D('',#1,#3,#4);\n\
#10=SPHERICAL_SURFACE('',#5,2.0);\n\
#11=TOROIDAL_SURFACE('',#5,5.0,1.0);\n\
#12=PLANE('',#5);\n\
#13=CARTESIAN_POINT('',(0.,0.,0.));\n\
#14=CARTESIAN_POINT('',(0.,1.,0.));\n\
#15=CARTESIAN_POINT('',(1.,0.,0.));\n\
#16=CARTESIAN_POINT('',(1.,1.,1.));\n\
#17=B_SPLINE_SURFACE_WITH_KNOTS('',1,1,((#13,#14),(#15,#16)),.UNSPECIFIED.,.F.,.F.,.F.,(2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.);\n\
#18=(BOUNDED_SURFACE()B_SPLINE_SURFACE(1,1,((#13,#14),(#15,#16)),.UNSPECIFIED.,.F.,.F.,.F.)B_SPLINE_SURFACE_WITH_KNOTS((2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(((1.,0.5),(0.5,1.)))REPRESENTATION_ITEM('')SURFACE());\n\
#20=VECTOR('',#4,1.0);\n\
#21=LINE('',#1,#20);\n\
#22=VERTEX_POINT('',#1);\n\
#23=VERTEX_POINT('',#2);\n\
#24=EDGE_CURVE('',#22,#23,#21,.T.);\n\
#25=ORIENTED_EDGE('',*,*,#24,.T.);\n\
#26=EDGE_LOOP('',(#25));\n\
#27=FACE_OUTER_BOUND('',#26,.T.);\n\
#30=ADVANCED_FACE('',(#27),#10,.T.);\n\
#31=ADVANCED_FACE('',(#27),#11,.T.);\n\
#32=ADVANCED_FACE('',(#27),#12,.T.);\n\
#33=ADVANCED_FACE('',(#27),#17,.T.);\n\
#34=ADVANCED_FACE('',(#27),#18,.T.);\n\
#40=CLOSED_SHELL('',(#30,#31,#32,#33,#34));\n\
#41=MANIFOLD_SOLID_BREP('',#40);\n";

fn approx(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}

fn find<'m>(scene: &'m Scene<'m>, pred: impl Fn(&Surface<'m>) -> bool) -> Surface<'m> {
    scene
        .all_faces()
        .map(|f| f.surface())
        .find(pred)
        .expect("a matching surface")
}

#[test]
fn sphere_becomes_rational_surface_of_revolution() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let sphere = find(&scene, |s| matches!(s.kind(), SurfaceKind::Spherical(_)));
    let n: NurbsSurface = sphere.to_nurbs().expect("sphere → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 2));
    assert_eq!(n.knots_u.len(), 12);
    assert_eq!(n.knots_v.len(), 8);
    assert_eq!(n.control_points.len(), 9); // u
    assert_eq!(n.control_points[0].len(), 5); // v
    assert!(approx(&n.knots_v, &[0., 0., 0., 1., 1., 2., 2., 2.]));

    // Equator on +x (u=0, v=middle) at radius 2.
    assert!(approx(&n.control_points[0][2], &[2., 0., 0.]));
    // Equator on +y (u=quarter).
    assert!(approx(&n.control_points[2][2], &[0., 2., 0.]));
    // The poles: every u control point collapses to a single point.
    for i in 0..9 {
        assert!(approx(&n.control_points[i][0], &[0., 0., -2.]), "south {i}");
        assert!(approx(&n.control_points[i][4], &[0., 0., 2.]), "north {i}");
    }
    // Weight = u-weight × v-weight.
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!((n.weights[0][2] - 1.0).abs() < 1e-9);
    assert!((n.weights[1][0] - w).abs() < 1e-9);
}

#[test]
fn torus_becomes_rational_surface_of_revolution() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let torus = find(&scene, |s| matches!(s.kind(), SurfaceKind::Toroidal(_)));
    let n = torus.to_nurbs().expect("torus → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 2));
    assert_eq!(n.control_points.len(), 9); // u
    assert_eq!(n.control_points[0].len(), 9); // v
    assert!(approx(
        &n.knots_v,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    ));

    // On the u=0 (+x) meridian: outer / top / inner points (major 5, minor 1).
    assert!(approx(&n.control_points[0][0], &[6., 0., 0.])); // major + minor
    assert!(approx(&n.control_points[0][2], &[5., 0., 1.])); // top of tube
    assert!(approx(&n.control_points[0][4], &[4., 0., 0.])); // major − minor
}

#[test]
fn bspline_surface_passes_through() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let bs = find(&scene, |s| {
        matches!(s.kind(), SurfaceKind::BSplineWithKnots(_))
    });
    let n = bs.to_nurbs().expect("bspline surface → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (1, 1));
    assert_eq!(n.control_points.len(), 2); // u
    assert_eq!(n.control_points[0].len(), 2); // v
    assert!(approx(&n.knots_u, &[0., 0., 1., 1.]));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    assert_eq!(n.weights, vec![vec![1.0; 2]; 2]);
    // Grid is control_points[u][v].
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[0., 1., 0.]));
    assert!(approx(&n.control_points[1][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[1][1], &[1., 1., 1.]));
}

#[test]
fn complex_rational_bspline_surface_passes_through() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The rational B-spline surface arrives as a complex instance.
    let bs = find(
        &scene,
        |s| matches!(s.kind(), SurfaceKind::Other(k) if k == "COMPLEX"),
    );
    let n = bs
        .to_nurbs()
        .expect("complex rational bspline surface → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (1, 1));
    assert!(approx(&n.knots_u, &[0., 0., 1., 1.]));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    // Weights from the RATIONAL_B_SPLINE_SURFACE part.
    assert_eq!(n.weights, vec![vec![1.0, 0.5], vec![0.5, 1.0]]);
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[0., 1., 0.]));
    assert!(approx(&n.control_points[1][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[1][1], &[1., 1., 1.]));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn open_surface_returns_none() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let plane = find(&scene, |s| matches!(s.kind(), SurfaceKind::Plane(_)));
    assert!(
        plane.to_nurbs().is_none(),
        "an unbounded plane has no NURBS on its own"
    );
    assert!(
        scene.warnings().is_empty(),
        "unsupported kind is silent: {:?}",
        scene.warnings()
    );
}

fn find_face<'m>(
    scene: &'m Scene<'m>,
    pred: impl Fn(&Surface<'m>) -> bool,
) -> step_io::scene::Face<'m> {
    scene
        .all_faces()
        .find(|f| pred(&f.surface()))
        .expect("a matching face")
}

// A unit-square PLANE face: the plane is unbounded, so `face.to_nurbs()` sizes a
// patch to the square's parameter-space bounding box.
const PLANE_FACE: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,0.,0.));\n\
#3=CARTESIAN_POINT('',(1.,1.,0.));\n\
#4=CARTESIAN_POINT('',(0.,1.,0.));\n\
#5=DIRECTION('',(0.,0.,1.));\n\
#6=DIRECTION('',(1.,0.,0.));\n\
#7=AXIS2_PLACEMENT_3D('',#1,#5,#6);\n\
#8=PLANE('',#7);\n\
#10=VERTEX_POINT('',#1);\n\
#11=VERTEX_POINT('',#2);\n\
#12=VERTEX_POINT('',#3);\n\
#13=VERTEX_POINT('',#4);\n\
#14=DIRECTION('',(1.,0.,0.));\n\
#15=VECTOR('',#14,1.0);\n\
#16=LINE('',#1,#15);\n\
#20=EDGE_CURVE('',#10,#11,#16,.T.);\n\
#21=EDGE_CURVE('',#11,#12,#16,.T.);\n\
#22=EDGE_CURVE('',#12,#13,#16,.T.);\n\
#23=EDGE_CURVE('',#13,#10,#16,.T.);\n\
#24=ORIENTED_EDGE('',*,*,#20,.T.);\n\
#25=ORIENTED_EDGE('',*,*,#21,.T.);\n\
#26=ORIENTED_EDGE('',*,*,#22,.T.);\n\
#27=ORIENTED_EDGE('',*,*,#23,.T.);\n\
#28=EDGE_LOOP('',(#24,#25,#26,#27));\n\
#29=FACE_OUTER_BOUND('',#28,.T.);\n\
#30=ADVANCED_FACE('',(#29),#8,.T.);\n\
#31=CLOSED_SHELL('',(#30));\n\
#32=MANIFOLD_SOLID_BREP('',#31);\n";

#[test]
fn plane_face_becomes_bounded_patch() {
    let src = format!("{HEADER}{PLANE_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Plane(_)));
    let n = face.to_nurbs().expect("plane face → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (1, 1));
    assert!(approx(&n.knots_u, &[0., 0., 1., 1.]));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    assert_eq!(n.weights, vec![vec![1.0; 2]; 2]);
    // Corners of the unit-square bounding box: [u][v].
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.])); // umin,vmin
    assert!(approx(&n.control_points[0][1], &[0., 1., 0.])); // umin,vmax
    assert!(approx(&n.control_points[1][0], &[1., 0., 0.])); // umax,vmin
    assert!(approx(&n.control_points[1][1], &[1., 1., 0.])); // umax,vmax

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn closed_surface_face_delegates_to_surface() {
    let src = format!("{HEADER}{SURFACES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // A sphere face returns the whole sphere (same as surface.to_nurbs).
    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Spherical(_)));
    assert_eq!(face.to_nurbs(), face.surface().to_nurbs());
    assert_eq!(face.to_nurbs().expect("sphere").control_points.len(), 9);
}

// A cylinder face whose edge vertices span axial height 0..3 (radius 2).
const CYL_FACE: &str = "\
#1=CARTESIAN_POINT('',(2.,0.,0.));\n\
#2=CARTESIAN_POINT('',(2.,0.,3.));\n\
#3=CARTESIAN_POINT('',(0.,0.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=DIRECTION('',(1.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#3,#4,#5);\n\
#7=CYLINDRICAL_SURFACE('',#6,2.0);\n\
#8=VERTEX_POINT('',#1);\n\
#9=VERTEX_POINT('',#2);\n\
#10=DIRECTION('',(0.,0.,1.));\n\
#11=VECTOR('',#10,1.0);\n\
#12=LINE('',#1,#11);\n\
#13=EDGE_CURVE('',#8,#9,#12,.T.);\n\
#14=ORIENTED_EDGE('',*,*,#13,.T.);\n\
#15=EDGE_LOOP('',(#14));\n\
#16=FACE_OUTER_BOUND('',#15,.T.);\n\
#17=ADVANCED_FACE('',(#16),#7,.T.);\n\
#18=CLOSED_SHELL('',(#17));\n\
#19=MANIFOLD_SOLID_BREP('',#18);\n";

#[test]
fn cylinder_face_becomes_ruled_patch() {
    let src = format!("{HEADER}{CYL_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Cylindrical(_)));
    let n = face.to_nurbs().expect("cylinder face → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 1));
    assert_eq!(n.control_points.len(), 9); // u = full circle
    assert_eq!(n.control_points[0].len(), 2); // v = two levels
    assert!(approx(
        &n.knots_u,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    ));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    // +x at height 0 and 3; +y at height 0.
    assert!(approx(&n.control_points[0][0], &[2., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[2., 0., 3.]));
    assert!(approx(&n.control_points[2][0], &[0., 2., 0.]));
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!((n.weights[1][0] - w).abs() < 1e-9);

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A cone (radius 1 at v=0, half-angle 45°) with vertices at v=0 and v=1, so the
// radius grows to 2 at the top.
const CONE_FACE: &str = "\
#1=CARTESIAN_POINT('',(1.,0.,0.));\n\
#2=CARTESIAN_POINT('',(2.,0.,1.));\n\
#3=CARTESIAN_POINT('',(0.,0.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=DIRECTION('',(1.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#3,#4,#5);\n\
#7=CONICAL_SURFACE('',#6,1.0,0.7853981633974483);\n\
#8=VERTEX_POINT('',#1);\n\
#9=VERTEX_POINT('',#2);\n\
#10=DIRECTION('',(0.,0.,1.));\n\
#11=VECTOR('',#10,1.0);\n\
#12=LINE('',#1,#11);\n\
#13=EDGE_CURVE('',#8,#9,#12,.T.);\n\
#14=ORIENTED_EDGE('',*,*,#13,.T.);\n\
#15=EDGE_LOOP('',(#14));\n\
#16=FACE_OUTER_BOUND('',#15,.T.);\n\
#17=ADVANCED_FACE('',(#16),#7,.T.);\n\
#18=CLOSED_SHELL('',(#17));\n\
#19=MANIFOLD_SOLID_BREP('',#18);\n";

#[test]
fn cone_face_radius_grows_with_height() {
    let src = format!("{HEADER}{CONE_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Conical(_)));
    let n = face.to_nurbs().expect("cone face → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 1));
    // radius 1 at v=0, radius 1 + 1·tan(45°) = 2 at v=1.
    assert!(approx(&n.control_points[0][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[2., 0., 1.]));
    assert!(approx(&n.control_points[2][0], &[0., 1., 0.]));
    assert!(approx(&n.control_points[2][1], &[0., 2., 1.]));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A cone tip: the cone of half-angle 45° through radius 1 at z=0 closes at the
// apex (0,0,−1). Its face is bounded by the circle at z=0 and, at the apex, a
// VERTEX_LOOP (#30–#33) — or by the circle alone.
const CONE_TIP: &str = "\
#1=CARTESIAN_POINT('',(1.,0.,0.));\n\
#3=CARTESIAN_POINT('',(0.,0.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=DIRECTION('',(1.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#3,#4,#5);\n\
#7=CONICAL_SURFACE('',#6,1.0,0.7853981633974483);\n\
#8=VERTEX_POINT('',#1);\n\
#12=CIRCLE('',#6,1.0);\n\
#13=EDGE_CURVE('',#8,#8,#12,.T.);\n\
#14=ORIENTED_EDGE('',*,*,#13,.T.);\n\
#15=EDGE_LOOP('',(#14));\n\
#16=FACE_OUTER_BOUND('',#15,.T.);\n\
#30=CARTESIAN_POINT('',(0.,0.,-1.));\n\
#31=VERTEX_POINT('',#30);\n\
#32=VERTEX_LOOP('',#31);\n\
#33=FACE_BOUND('',#32,.T.);\n\
#17=ADVANCED_FACE('',(#16,#33),#7,.T.);\n\
#18=CLOSED_SHELL('',(#17));\n\
#19=MANIFOLD_SOLID_BREP('',#18);\n";

/// A cone face that closes at its apex converts, its patch reaching the apex —
/// whether the apex is a `VERTEX_LOOP` boundary, reached by `Bound::vertex`,
/// or left implicit with the circle the only boundary.
#[test]
fn a_cone_tip_reaches_its_apex() {
    let without_apex = CONE_TIP.replace("(#16,#33)", "(#16)");
    // The circle tilted by 1e-12, as rounding leaves a circle that sits at
    // one level in the writer's model: still one level, not a sliver.
    let noisy = without_apex.replace(
        "#12=CIRCLE('',#6,1.0);",
        "#40=DIRECTION('',(0.,1.E-12,1.));\n#41=AXIS2_PLACEMENT_3D('',#3,#40,#5);\n#12=CIRCLE('',#41,1.0);",
    );
    for (src, apex_bound) in [
        (CONE_TIP.to_owned(), true),
        (without_apex, false),
        (noisy, false),
    ] {
        let src = format!("{HEADER}{src}{FOOTER}");
        let (model, _rep) = read(src.as_bytes()).expect("read");
        let scene = model.scene();
        let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Conical(_)));

        let apexes: Vec<_> = face
            .bounds()
            .filter_map(|b| b.vertex())
            .map(|v| v.point().expect("the apex point").xyz())
            .collect();
        assert_eq!(
            apexes,
            if apex_bound {
                vec![[0., 0., -1.]]
            } else {
                vec![]
            }
        );

        let n = face.to_nurbs().expect("a cone tip → nurbs");
        assert!(approx(&n.control_points[0][0], &[0., 0., -1.]), "apex row");
        assert!(approx(&n.control_points[0][1], &[1., 0., 0.]), "circle row");
        assert!(
            scene.warnings().is_empty(),
            "warnings: {:?}",
            scene.warnings()
        );
    }
}

// A plane-angle unit of degrees, for the cone below.
const DEGREE_UNITS: &str = "\
#40=( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
#41=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#40);\n\
#42=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\n\
#43=( CONVERSION_BASED_UNIT('DEGREE',#41) NAMED_UNIT(#42) PLANE_ANGLE_UNIT() );\n\
#44=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#45=( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNIT_ASSIGNED_CONTEXT((#44,#43)) REPRESENTATION_CONTEXT('','') );\n\
#46=SHAPE_REPRESENTATION('',(#19),#45);\n";

/// The cone's half-angle is in the file's plane-angle unit: written as `45.`
/// under a degree unit, it is the same 45° cone.
#[test]
fn cone_half_angle_follows_the_angle_unit() {
    let in_degrees = CONE_FACE.replace("0.7853981633974483", "45.");
    let src = format!("{HEADER}{in_degrees}{DEGREE_UNITS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Conical(_)));
    let n = face.to_nurbs().expect("cone face → nurbs");
    assert!(approx(&n.control_points[0][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[2., 0., 1.]));
    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A circle (radius 1 in the xy plane) extruded along z. The vector's magnitude
// is deliberately 5.0 — a parameter scale, not an extrusion length — while the
// face's vertices sit at z=0 and z=2, so the patch must span [0, 2].
const EXTRUDE_FACE: &str = "\
#1=CARTESIAN_POINT('',(1.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,0.,2.));\n\
#3=CARTESIAN_POINT('',(0.,0.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=DIRECTION('',(1.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#3,#4,#5);\n\
#7=CIRCLE('',#6,1.0);\n\
#8=VECTOR('',#4,5.0);\n\
#9=SURFACE_OF_LINEAR_EXTRUSION('',#7,#8);\n\
#10=VERTEX_POINT('',#1);\n\
#11=VERTEX_POINT('',#2);\n\
#12=VECTOR('',#4,1.0);\n\
#13=LINE('',#1,#12);\n\
#14=EDGE_CURVE('',#10,#11,#13,.T.);\n\
#15=ORIENTED_EDGE('',*,*,#14,.T.);\n\
#16=EDGE_LOOP('',(#15));\n\
#17=FACE_OUTER_BOUND('',#16,.T.);\n\
#18=ADVANCED_FACE('',(#17),#9,.T.);\n\
#19=CLOSED_SHELL('',(#18));\n\
#20=MANIFOLD_SOLID_BREP('',#19);\n";

#[test]
fn extruded_circle_face_becomes_ruled_patch() {
    let src = format!("{HEADER}{EXTRUDE_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| {
        matches!(s.kind(), SurfaceKind::LinearExtrusion(_))
    });
    assert!(
        face.surface().to_nurbs().is_none(),
        "unbounded at surface level"
    );
    let n = face.to_nurbs().expect("extrusion face → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 1));
    assert_eq!(n.control_points.len(), 9); // u = the full-circle profile
    assert_eq!(n.control_points[0].len(), 2); // v = two levels
    assert!(approx(
        &n.knots_u,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    ));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    // The span follows the vertices (z 0..2), not the vector magnitude (5).
    assert!(approx(&n.control_points[0][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[1., 0., 2.]));
    assert!(approx(&n.control_points[2][0], &[0., 1., 0.]));
    assert!(approx(&n.control_points[2][1], &[0., 1., 2.]));
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!((n.weights[1][0] - w).abs() < 1e-9);
    assert!((n.weights[1][1] - w).abs() < 1e-9);

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A trimmed line at rho=2 parallel to the axis (t 0..3), revolved a full turn:
// a bounded profile converts whole at surface level, matching the CYL_FACE
// cylinder patch values.
const REVOLVE_SURF: &str = "\
#1=CARTESIAN_POINT('',(2.,0.,0.));\n\
#2=CARTESIAN_POINT('',(0.,0.,0.));\n\
#3=DIRECTION('',(0.,0.,1.));\n\
#4=VECTOR('',#3,1.0);\n\
#5=LINE('',#1,#4);\n\
#6=TRIMMED_CURVE('',#5,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(3.)),.T.,.PARAMETER.);\n\
#7=AXIS1_PLACEMENT('',#2,#3);\n\
#8=SURFACE_OF_REVOLUTION('',#6,#7);\n\
#10=VERTEX_POINT('',#1);\n\
#11=CARTESIAN_POINT('',(2.,0.,3.));\n\
#12=VERTEX_POINT('',#11);\n\
#13=EDGE_CURVE('',#10,#12,#5,.T.);\n\
#14=ORIENTED_EDGE('',*,*,#13,.T.);\n\
#15=EDGE_LOOP('',(#14));\n\
#16=FACE_OUTER_BOUND('',#15,.T.);\n\
#17=ADVANCED_FACE('',(#16),#8,.T.);\n\
#18=CLOSED_SHELL('',(#17));\n\
#19=MANIFOLD_SOLID_BREP('',#18);\n";

#[test]
fn revolved_trimmed_line_converts_whole_at_surface_level() {
    let src = format!("{HEADER}{REVOLVE_SURF}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let surface = find(&scene, |s| matches!(s.kind(), SurfaceKind::Revolution(_)));
    let n = surface.to_nurbs().expect("revolution → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 1));
    assert_eq!(n.control_points.len(), 9); // u = the full circle
    assert_eq!(n.control_points[0].len(), 2); // v = the trimmed segment
    assert!(approx(
        &n.knots_u,
        &[0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
    ));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    // The same values as the CYL_FACE cylinder patch (radius 2, height 0..3).
    assert!(approx(&n.control_points[0][0], &[2., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[2., 0., 3.]));
    // The odd corner: the perpendicular part rotated 45° and scaled √2.
    assert!(approx(&n.control_points[1][0], &[2., 2., 0.]));
    assert!(approx(&n.control_points[2][0], &[0., 2., 0.]));
    let w = std::f64::consts::FRAC_1_SQRT_2;
    assert!((n.weights[1][0] - w).abs() < 1e-9);
    assert!((n.weights[1][1] - w).abs() < 1e-9);

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// An unbounded LINE profile (lathe-style export) placed off the meridian at
// +y, revolved around z. Unconvertible at surface level; the face's vertices
// (z 0 and 2) bound it into a cylinder patch.
const REVOLVE_LINE_FACE: &str = "\
#1=CARTESIAN_POINT('',(0.,1.,0.));\n\
#2=CARTESIAN_POINT('',(0.,0.,0.));\n\
#3=DIRECTION('',(0.,0.,1.));\n\
#4=VECTOR('',#3,1.0);\n\
#5=LINE('',#1,#4);\n\
#6=AXIS1_PLACEMENT('',#2,#3);\n\
#7=SURFACE_OF_REVOLUTION('',#5,#6);\n\
#10=VERTEX_POINT('',#1);\n\
#11=CARTESIAN_POINT('',(0.,1.,2.));\n\
#12=VERTEX_POINT('',#11);\n\
#13=EDGE_CURVE('',#10,#12,#5,.T.);\n\
#14=ORIENTED_EDGE('',*,*,#13,.T.);\n\
#15=EDGE_LOOP('',(#14));\n\
#16=FACE_OUTER_BOUND('',#15,.T.);\n\
#17=ADVANCED_FACE('',(#16),#7,.T.);\n\
#18=CLOSED_SHELL('',(#17));\n\
#19=MANIFOLD_SOLID_BREP('',#18);\n";

#[test]
fn revolved_line_profile_face_becomes_patch() {
    let src = format!("{HEADER}{REVOLVE_LINE_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Revolution(_)));
    assert!(
        face.surface().to_nurbs().is_none(),
        "a LINE profile is unbounded at surface level"
    );
    let n = face.to_nurbs().expect("revolved line face → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (2, 1));
    assert_eq!(n.control_points.len(), 9);
    assert_eq!(n.control_points[0].len(), 2);
    // The u origin is the profile's own azimuth (+y); +90° lands on -x.
    assert!(approx(&n.control_points[0][0], &[0., 1., 0.]));
    assert!(approx(&n.control_points[0][1], &[0., 1., 2.]));
    assert!(approx(&n.control_points[2][0], &[-1., 0., 0.]));
    assert!(approx(&n.control_points[2][1], &[-1., 0., 2.]));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A quasi-uniform B-spline surface: no stored knots — the standard clamped
// rule derives them (here degree 1×1 over a 2×2 grid → a single span).
const QUASI_SURF: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(0.,1.,0.));\n\
#3=CARTESIAN_POINT('',(1.,0.,0.));\n\
#4=CARTESIAN_POINT('',(1.,1.,1.));\n\
#5=QUASI_UNIFORM_SURFACE('',1,1,((#1,#2),(#3,#4)),.UNSPECIFIED.,.F.,.F.,.F.);\n\
#10=VERTEX_POINT('',#1);\n\
#11=VERTEX_POINT('',#4);\n\
#12=DIRECTION('',(0.,0.,1.));\n\
#13=VECTOR('',#12,1.0);\n\
#14=LINE('',#1,#13);\n\
#15=EDGE_CURVE('',#10,#11,#14,.T.);\n\
#16=ORIENTED_EDGE('',*,*,#15,.T.);\n\
#17=EDGE_LOOP('',(#16));\n\
#18=FACE_OUTER_BOUND('',#17,.T.);\n\
#19=ADVANCED_FACE('',(#18),#5,.T.);\n\
#20=CLOSED_SHELL('',(#19));\n\
#21=MANIFOLD_SOLID_BREP('',#20);\n";

#[test]
fn quasi_uniform_surface_derives_its_knots() {
    let src = format!("{HEADER}{QUASI_SURF}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let surface = find(&scene, |s| matches!(s.kind(), SurfaceKind::QuasiUniform(_)));
    let n = surface.to_nurbs().expect("quasi-uniform surface → nurbs");

    assert_eq!((n.degree_u, n.degree_v), (1, 1));
    assert!(approx(&n.knots_u, &[0., 0., 1., 1.]));
    assert!(approx(&n.knots_v, &[0., 0., 1., 1.]));
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.]));
    assert!(approx(&n.control_points[1][1], &[1., 1., 1.]));
    assert!(approx(&n.weights[0], &[1., 1.]));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A plane face bounded by an upper semicircle plus its closing chord: both
// vertices sit at y=0, so sizing by vertices alone would be a degenerate
// (zero-height) box — the arc's control points carry the y=1 bulge.
const ARC_PLANE_FACE: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=PLANE('',#4);\n\
#6=CIRCLE('',#4,1.0);\n\
#7=CARTESIAN_POINT('',(1.,0.,0.));\n\
#8=CARTESIAN_POINT('',(-1.,0.,0.));\n\
#9=DIRECTION('',(-1.,0.,0.));\n\
#10=VECTOR('',#9,1.0);\n\
#11=LINE('',#7,#10);\n\
#20=VERTEX_POINT('',#7);\n\
#21=VERTEX_POINT('',#8);\n\
#30=EDGE_CURVE('',#20,#21,#6,.T.);\n\
#31=EDGE_CURVE('',#21,#20,#11,.T.);\n\
#40=ORIENTED_EDGE('',*,*,#30,.T.);\n\
#41=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#42=EDGE_LOOP('',(#40,#41));\n\
#43=FACE_OUTER_BOUND('',#42,.T.);\n\
#44=ADVANCED_FACE('',(#43),#5,.T.);\n\
#45=CLOSED_SHELL('',(#44));\n\
#46=MANIFOLD_SOLID_BREP('',#45);\n";

#[test]
fn curved_edge_bulge_is_covered_by_the_patch() {
    let src = format!("{HEADER}{ARC_PLANE_FACE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Plane(_)));
    let n = face.to_nurbs().expect("semicircle plane face → nurbs");

    // The 180° arc's control points span x ∈ [−1,1] and y ∈ [0,1]; the patch
    // covers the bulge that the vertices (both at y=0) cannot see.
    assert_eq!((n.degree_u, n.degree_v), (1, 1));
    assert!(approx(&n.control_points[0][0], &[-1., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[-1., 1., 0.]));
    assert!(approx(&n.control_points[1][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[1][1], &[1., 1., 0.]));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn bspline_surface_with_a_huge_multiplicity_is_refused() {
    let src = format!(
        "{HEADER}{}{FOOTER}",
        SURFACES.replacen(
            "(2,2),(2,2),(0.,1.)",
            "(1000000000000000,2),(2,2),(0.,1.)",
            1
        )
    );
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| {
        matches!(s.kind(), SurfaceKind::BSplineWithKnots(_))
    });
    assert!(face.surface().to_nurbs().is_none());
    assert!(face.to_nurbs().is_none());
    let warnings = scene.warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("inconsistent weight/knot count")),
        "warnings: {warnings:?}"
    );
}

/// A point of a NURBS surface, by Cox–de Boor in homogeneous coordinates.
fn surface_point(n: &NurbsSurface, u: f64, v: f64) -> [f64; 3] {
    fn basis(knots: &[f64], degree: usize, t: f64) -> Vec<f64> {
        let mut b: Vec<f64> = knots
            .windows(2)
            .map(|k| f64::from(u8::from(k[0] <= t && t < k[1])))
            .collect();
        for p in 1..=degree {
            for i in 0..knots.len() - 1 - p {
                let left = knots[i + p] - knots[i];
                let right = knots[i + p + 1] - knots[i + 1];
                let a = if left > 0.0 {
                    (t - knots[i]) / left * b[i]
                } else {
                    0.0
                };
                let c = if right > 0.0 {
                    (knots[i + p + 1] - t) / right * b[i + 1]
                } else {
                    0.0
                };
                b[i] = a + c;
            }
        }
        b.truncate(knots.len() - degree - 1);
        b
    }
    let (bu, bv) = (
        basis(&n.knots_u, n.degree_u, u),
        basis(&n.knots_v, n.degree_v, v),
    );
    let mut sum = [0.0; 4];
    for (i, row) in n.control_points.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            let w = bu[i] * bv[j] * n.weights[i][j];
            for k in 0..3 {
                sum[k] += w * p[k];
            }
            sum[3] += w;
        }
    }
    [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3]]
}

/// Points spread over the open parameter ranges of `n`, as (u, v, point).
fn surface_samples(n: &NurbsSurface) -> Vec<(f64, f64, [f64; 3])> {
    let span = |k: &[f64]| (k[0], k[k.len() - 1]);
    let ((u0, u1), (v0, v1)) = (span(&n.knots_u), span(&n.knots_v));
    let mut out = Vec::new();
    for i in 0..17 {
        for j in 0..23 {
            let u = u0 + (u1 - u0) * (f64::from(i) + 0.5) / 17.0;
            let v = v0 + (v1 - v0) * (f64::from(j) + 0.5) / 23.0;
            out.push((u, v, surface_point(n, u, v)));
        }
    }
    out
}

/// The direction of `S_u × S_v` at (u, v), by central differences.
fn surface_normal(n: &NurbsSurface, u: f64, v: f64) -> [f64; 3] {
    let step = 1e-6;
    let diff = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let su = diff(surface_point(n, u + step, v), surface_point(n, u - step, v));
    let sv = diff(surface_point(n, u, v + step), surface_point(n, u, v - step));
    let cross = [
        su[1] * sv[2] - su[2] * sv[1],
        su[2] * sv[0] - su[0] * sv[2],
        su[0] * sv[1] - su[1] * sv[0],
    ];
    let len = dot(cross, cross).sqrt();
    cross.map(|c| c / len)
}

/// The unit direction away from the axis at the surface point (u, v).
fn outward(n: &NurbsSurface, u: f64, v: f64) -> [f64; 3] {
    let point = surface_point(n, u, v);
    let len = point[0].hypot(point[1]);
    [point[0] / len, point[1] / len, 0.0]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Distance from `p` to the axis (`z`) and its height, in the standard frame.
fn radius_height(p: [f64; 3]) -> (f64, f64) {
    (p[0].hypot(p[1]), p[2])
}

// Degenerate tori on the standard frame: major 1, minor 2 (outer lobe and
// inner spindle), and major = minor 2 (a horn torus).
const DEGENERATE_TORI: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,0.,0.));\n\
#3=DIRECTION('',(0.,0.,1.));\n\
#4=DIRECTION('',(1.,0.,0.));\n\
#5=AXIS2_PLACEMENT_3D('',#1,#3,#4);\n\
#10=DEGENERATE_TOROIDAL_SURFACE('apple',#5,1.0,2.0,.T.);\n\
#11=DEGENERATE_TOROIDAL_SURFACE('lemon',#5,1.0,2.0,.F.);\n\
#12=DEGENERATE_TOROIDAL_SURFACE('horn',#5,2.0,2.0,.T.);\n\
#13=DEGENERATE_TOROIDAL_SURFACE('no spindle',#5,2.0,2.0,.F.);\n\
#20=VECTOR('',#4,1.0);\n\
#21=LINE('',#1,#20);\n\
#22=VERTEX_POINT('',#1);\n\
#23=VERTEX_POINT('',#2);\n\
#24=EDGE_CURVE('',#22,#23,#21,.T.);\n\
#25=ORIENTED_EDGE('',*,*,#24,.T.);\n\
#26=EDGE_LOOP('',(#25));\n\
#27=FACE_OUTER_BOUND('',#26,.T.);\n\
#30=ADVANCED_FACE('',(#27),#10,.T.);\n\
#31=ADVANCED_FACE('',(#27),#11,.T.);\n\
#32=ADVANCED_FACE('',(#27),#12,.T.);\n\
#33=ADVANCED_FACE('',(#27),#13,.T.);\n\
#40=CLOSED_SHELL('',(#30,#31,#32,#33));\n\
#41=MANIFOLD_SOLID_BREP('',#40);\n";

fn degenerate_torus<'m>(scene: &'m Scene<'m>, name: &str) -> Surface<'m> {
    find(
        scene,
        |s| matches!(s.kind(), SurfaceKind::DegenerateToroidal(t) if scene.model().text(t.name) == name),
    )
}

/// The outer lobe of a degenerate torus: the tube circle between its two
/// crossings of the axis, through the outer equator.
#[test]
fn a_degenerate_torus_selecting_the_outer_lobe_becomes_the_apple() {
    let src = format!("{HEADER}{DEGENERATE_TORI}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let n = degenerate_torus(&scene, "apple").to_nurbs().expect("apple");

    // Both ends of the profile meet the axis, at height ±√3.
    let h = 3.0_f64.sqrt();
    let last = n.control_points[0].len() - 1;
    for row in &n.control_points {
        assert!(approx(&row[0], &[0., 0., -h]), "{:?}", row[0]);
        assert!(approx(&row[last], &[0., 0., h]), "{:?}", row[last]);
    }
    // Every point lies on the tube circle about radius 1, on the near side.
    for (_, _, p) in surface_samples(&n) {
        let (rho, z) = radius_height(p);
        assert!(((rho - 1.0).hypot(z) - 2.0).abs() < 1e-9, "{p:?}");
    }
    // Halfway along the profile is the outer equator, radius 3.
    let mid = n.knots_v[n.knots_v.len() - 1] / 2.0;
    assert!(approx(&surface_point(&n, 0.0, mid), &[3., 0., 0.]));
    // The torus's own normal there points away from the axis.
    let (normal, out) = (surface_normal(&n, 0.1, mid), outward(&n, 0.1, mid));
    assert!(dot(normal, out) > 1.0 - 1e-9, "{normal:?}");
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// The inner spindle: the rest of the tube circle, on the far side of the
/// axis, with its equator at radius `minor − major`.
#[test]
fn a_degenerate_torus_selecting_the_inner_part_becomes_the_lemon() {
    let src = format!("{HEADER}{DEGENERATE_TORI}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let n = degenerate_torus(&scene, "lemon").to_nurbs().expect("lemon");

    let h = 3.0_f64.sqrt();
    let last = n.control_points[0].len() - 1;
    for row in &n.control_points {
        assert!(approx(&row[0], &[0., 0., h]), "{:?}", row[0]);
        assert!(approx(&row[last], &[0., 0., -h]), "{:?}", row[last]);
    }
    for (_, _, p) in surface_samples(&n) {
        let (rho, z) = radius_height(p);
        assert!(((rho + 1.0).hypot(z) - 2.0).abs() < 1e-9, "{p:?}");
    }
    // The profile's middle is across the axis from u = 0, at radius 1.
    let mid = n.knots_v[n.knots_v.len() - 1] / 2.0;
    assert!(approx(&surface_point(&n, 0.0, mid), &[-1., 0., 0.]));
    // The torus's own normal there, `(major + minor·cos v)·(cos v, sin v)`
    // with both factors negative at v = π, points towards the axis.
    let (normal, out) = (surface_normal(&n, 0.1, mid), outward(&n, 0.1, mid));
    assert!(dot(normal, out) < -1.0 + 1e-9, "{normal:?}");
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// A tube that only touches the axis is whole as its outer lobe, and has no
/// inner spindle.
#[test]
fn a_horn_torus_is_the_whole_tube() {
    let src = format!("{HEADER}{DEGENERATE_TORI}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let n = degenerate_torus(&scene, "horn").to_nurbs().expect("horn");

    // The profile is a closed circle, starting and ending on the axis.
    let last = n.control_points[0].len() - 1;
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.]));
    assert!(approx(&n.control_points[0][last], &[0., 0., 0.]));
    for (_, _, p) in surface_samples(&n) {
        let (rho, z) = radius_height(p);
        assert!(((rho - 2.0).hypot(z) - 2.0).abs() < 1e-9, "{p:?}");
    }
    let mid = n.knots_v[n.knots_v.len() - 1] / 2.0;
    assert!(approx(&surface_point(&n, 0.0, mid), &[4., 0., 0.]));

    assert!(degenerate_torus(&scene, "no spindle").to_nurbs().is_none());
    let warnings = scene.warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("no inner spindle")),
        "{warnings:?}"
    );
}

// Offsets of the surfaces above: of the B-spline (no exact form), of the
// sphere (radius 2) out by 1.5 and in past its centre, and of the torus
// (major 5, minor 1) out by 1 and out past the axis.
const OFFSETS: &str = "\
#50=OFFSET_SURFACE('bspline',#17,0.5,.F.);\n\
#51=OFFSET_SURFACE('sphere out',#10,1.5,.F.);\n\
#52=OFFSET_SURFACE('sphere past centre',#10,-3.0,.F.);\n\
#53=OFFSET_SURFACE('torus out',#11,1.0,.F.);\n\
#54=OFFSET_SURFACE('torus past axis',#11,5.0,.F.);\n\
#60=ADVANCED_FACE('',(#27),#50,.T.);\n\
#61=ADVANCED_FACE('',(#27),#51,.T.);\n\
#62=ADVANCED_FACE('',(#27),#52,.T.);\n\
#63=ADVANCED_FACE('',(#27),#53,.T.);\n\
#64=ADVANCED_FACE('',(#27),#54,.T.);\n\
#65=CLOSED_SHELL('',(#60,#61,#62,#63,#64));\n\
#66=MANIFOLD_SOLID_BREP('',#65);\n";

fn offset<'m>(scene: &'m Scene<'m>, name: &str) -> Surface<'m> {
    find(
        scene,
        |s| matches!(s.kind(), SurfaceKind::Offset(o) if scene.model().text(o.name) == name),
    )
}

/// An offset reaches its basis as a handle; over a free-form surface it has
/// no exact NURBS of its own.
#[test]
fn an_offset_surface_gives_its_basis() {
    let src = format!("{HEADER}{SURFACES}{OFFSETS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let surface = offset(&scene, "bspline");

    let SurfaceKind::Offset(o) = surface.kind() else {
        panic!("an offset")
    };
    assert!((o.distance - 0.5).abs() < 1e-12);
    let basis = surface.basis_surface().expect("the basis");
    assert!(matches!(basis.kind(), SurfaceKind::BSplineWithKnots(_)));
    assert!(basis.to_nurbs().is_some());
    assert!(surface.to_nurbs().is_none());
    assert!(basis.basis_surface().is_none());
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// An offset sphere or torus is the same kind of surface with the radius
/// moved; one turned inside out or crossing itself is not converted.
#[test]
fn an_offset_sphere_or_torus_converts_whole() {
    let src = format!("{HEADER}{SURFACES}{OFFSETS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let sphere = offset(&scene, "sphere out").to_nurbs().expect("sphere");
    for (u, v, p) in surface_samples(&sphere) {
        let r = dot(p, p).sqrt();
        assert!((r - 3.5).abs() < 1e-9, "{p:?}");
        // Still pointing away from the centre, as the basis does.
        let normal = surface_normal(&sphere, u, v);
        assert!(dot(normal, p) / r > 1.0 - 1e-6, "{normal:?} at {p:?}");
    }
    let torus = offset(&scene, "torus out").to_nurbs().expect("torus");
    for (_, _, p) in surface_samples(&torus) {
        let (rho, z) = radius_height(p);
        assert!(((rho - 5.0).hypot(z) - 2.0).abs() < 1e-9, "{p:?}");
    }

    assert!(offset(&scene, "sphere past centre").to_nurbs().is_none());
    assert!(offset(&scene, "torus past axis").to_nurbs().is_none());
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// The cylinder face above, its radius-2 surface written as an offset of a
/// radius-1 cylinder: the face converts to the same patch. Offset inward past
/// the axis instead, it is not converted.
#[test]
fn an_offset_cylinder_face_becomes_ruled_patch() {
    let as_offset = |d: &str| {
        CYL_FACE.replace(
            "#7=CYLINDRICAL_SURFACE('',#6,2.0);",
            &format!("#7=OFFSET_SURFACE('',#30,{d},.F.);\n#30=CYLINDRICAL_SURFACE('',#6,1.0);"),
        )
    };
    let plain = format!("{HEADER}{CYL_FACE}{FOOTER}");
    let (plain, _rep) = read(plain.as_bytes()).expect("read");
    let expected = plain.scene().all_faces().next().expect("face").to_nurbs();

    let src = format!("{HEADER}{}{FOOTER}", as_offset("1.0"));
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Offset(_)));
    let n = face.to_nurbs().expect("offset cylinder face");
    assert_eq!(Some(&n), expected.as_ref());
    for (u, v, p) in surface_samples(&n) {
        let (rho, _) = radius_height(p);
        assert!((rho - 2.0).abs() < 1e-9, "{p:?}");
        assert!(dot(surface_normal(&n, u, v), outward(&n, u, v)) > 1.0 - 1e-6);
    }

    let src = format!("{HEADER}{}{FOOTER}", as_offset("-3.0"));
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Offset(_)));
    assert!(face.to_nurbs().is_none());
}

/// An offset cone is a cone on the same placement, its radius grown by
/// `d / cos(semi_angle)` — the angle read in the file's unit. The 45° cone
/// face above, written as an offset of a cone through the origin.
#[test]
fn an_offset_cone_face_follows_the_angle_unit() {
    let in_degrees = CONE_FACE.replace(
        "#7=CONICAL_SURFACE('',#6,1.0,0.7853981633974483);",
        "#7=OFFSET_SURFACE('',#30,0.7071067811865476,.F.);\n#30=CONICAL_SURFACE('',#6,0.,45.);",
    );
    let src = format!("{HEADER}{in_degrees}{DEGREE_UNITS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Offset(_)));
    let n = face.to_nurbs().expect("offset cone face");
    assert!(approx(&n.control_points[0][0], &[1., 0., 0.]));
    assert!(approx(&n.control_points[0][1], &[2., 0., 1.]));
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// The unit-square plane face above, on a plane one below written as an
/// offset of it: the same patch.
#[test]
fn an_offset_plane_face_becomes_bounded_patch() {
    let as_offset = PLANE_FACE.replace(
        "#8=PLANE('',#7);",
        "#8=OFFSET_SURFACE('',#33,1.,.F.);\n#33=PLANE('',#34);\n\
         #34=AXIS2_PLACEMENT_3D('',#35,#5,#6);\n#35=CARTESIAN_POINT('',(0.,0.,-1.));",
    );
    let src = format!("{HEADER}{as_offset}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = find_face(&scene, |s| matches!(s.kind(), SurfaceKind::Offset(_)));
    let n = face.to_nurbs().expect("offset plane face");
    assert!(approx(&n.control_points[0][0], &[0., 0., 0.]));
    assert!(approx(&n.control_points[1][1], &[1., 1., 0.]));
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}
