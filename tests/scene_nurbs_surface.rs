//! Opt-in NURBS surface conversion (`surface.to_nurbs()`): a sphere and a torus
//! become rational surfaces of revolution; an (unbounded) plane returns `None`.
//! Surfaces are reached through anchored faces (`all_faces().surface()`).

use step_io::read;
use step_io::scene::geometry::{Surface, SurfaceKind};
use step_io::scene::{NurbsSurface, Scene};

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
) -> step_io::scene::geometry::Face<'m> {
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
