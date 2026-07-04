//! `MAPPED_ITEM` placement handles: `scene.all_mapped_instances()` exposes the
//! frame pair (like an occurrence transform), the source solids, and nested
//! mapped items of an assembly chain.

use step_io::read;

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

// An ABSR whose item is a mapped instance (#60) of a shape representation
// (#52) placed at (10,0,0); that source representation itself contains a
// nested mapped instance (#61) of a second solid's representation (#54).
const MAPPED: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=CARTESIAN_POINT('',(10.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#5,#2,#3);\n\
#10=CARTESIAN_POINT('',(1.,0.,0.));\n\
#11=VERTEX_POINT('',#1);\n\
#12=VERTEX_POINT('',#10);\n\
#13=VECTOR('',#3,1.0);\n\
#14=LINE('',#1,#13);\n\
#15=EDGE_CURVE('',#11,#12,#14,.T.);\n\
#16=ORIENTED_EDGE('',*,*,#15,.T.);\n\
#17=EDGE_LOOP('',(#16));\n\
#18=FACE_OUTER_BOUND('',#17,.T.);\n\
#19=PLANE('',#4);\n\
#20=ADVANCED_FACE('',(#18),#19,.T.);\n\
#21=CLOSED_SHELL('',(#20));\n\
#22=MANIFOLD_SOLID_BREP('part A',#21);\n\
#31=CLOSED_SHELL('',(#20));\n\
#32=MANIFOLD_SOLID_BREP('part B',#31);\n\
#40=REPRESENTATION_CONTEXT('','');\n\
#52=SHAPE_REPRESENTATION('source A',(#22,#61),#40);\n\
#54=SHAPE_REPRESENTATION('source B',(#32),#40);\n\
#55=REPRESENTATION_MAP(#4,#52);\n\
#56=REPRESENTATION_MAP(#4,#54);\n\
#60=MAPPED_ITEM('outer',#55,#6);\n\
#61=MAPPED_ITEM('inner',#56,#4);\n\
#8=DIRECTION('',(0.,1.,0.));\n\
#9=AXIS2_PLACEMENT_3D('',#1,#2,#8);\n\
#62=MAPPED_ITEM('rotated',#55,#9);\n\
#70=ADVANCED_BREP_SHAPE_REPRESENTATION('assembly',(#60,#62),#40);\n";

#[test]
fn mapped_instances_expose_transform_solids_and_children() {
    let src = format!("{HEADER}{MAPPED}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let instances: Vec<_> = scene.all_mapped_instances().collect();
    assert_eq!(instances.len(), 3);

    let outer = instances
        .iter()
        .find(|i| i.name() == "outer")
        .expect("outer");
    let t = outer.transform().expect("transform");
    assert!(t.from.origin.iter().all(|v| v.abs() < 1e-12));
    assert!((t.to.origin[0] - 10.0).abs() < 1e-12);
    let solids = outer.solids();
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].name(), "part A");
    let children = outer.mapped_children();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name(), "inner");
    assert_eq!(children[0].solids().len(), 1);
    assert_eq!(children[0].solids()[0].name(), "part B");

    // matrix(): outer is a pure translation — identity rotation, t = (10,0,0).
    let m = t.matrix().expect("matrix");
    for (i, row) in m.iter().enumerate() {
        for (j, v) in row.iter().enumerate().take(3) {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((v - want).abs() < 1e-12, "rotation m[{i}][{j}]={v}");
        }
    }
    assert!((m[0][3] - 10.0).abs() < 1e-12);
    assert!(m[1][3].abs() < 1e-12 && m[2][3].abs() < 1e-12);
    assert!(
        m[3].iter()
            .zip([0.0, 0.0, 0.0, 1.0])
            .all(|(a, b)| (a - b).abs() < 1e-12)
    );

    // The inner chain maps #4 onto itself: the identity matrix.
    let mi = children[0]
        .transform()
        .expect("inner transform")
        .matrix()
        .expect("inner matrix");
    for (i, row) in mi.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((v - want).abs() < 1e-12, "identity m[{i}][{j}]={v}");
        }
    }

    // The rotated target turns the frame 90 deg about z: x → y.
    let rot = instances
        .iter()
        .find(|i| i.name() == "rotated")
        .expect("rotated");
    let mr = rot
        .transform()
        .expect("rot transform")
        .matrix()
        .expect("rot matrix");
    assert!((mr[1][0] - 1.0).abs() < 1e-12 && (mr[0][1] + 1.0).abs() < 1e-12);
    assert!(mr[0][0].abs() < 1e-12 && mr[1][1].abs() < 1e-12);
    // p = (1,0,0) lands on (0,1,0).
    let p = [1.0, 0.0, 0.0, 1.0];
    let mapped: Vec<f64> = (0..4)
        .map(|i| (0..4).map(|j| mr[i][j] * p[j]).sum())
        .collect();
    assert!((mapped[0]).abs() < 1e-12 && (mapped[1] - 1.0).abs() < 1e-12);

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A mapped item whose target is a 2D placement (a drawing view): the frame
// pair cannot be given as 3D frames, so transform() refuses with a warning.
const MAPPED_2D: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=DIRECTION('',(0.,0.,1.));\n\
#3=DIRECTION('',(1.,0.,0.));\n\
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);\n\
#5=CARTESIAN_POINT('',(0.,0.));\n\
#6=DIRECTION('',(1.,0.));\n\
#7=AXIS2_PLACEMENT_2D('',#5,#6);\n\
#40=REPRESENTATION_CONTEXT('','');\n\
#50=SHAPE_REPRESENTATION('src',(#4),#40);\n\
#55=REPRESENTATION_MAP(#4,#50);\n\
#60=MAPPED_ITEM('to 2d',#55,#7);\n\
#70=SHAPE_REPRESENTATION('parent',(#60),#40);\n";

#[test]
fn mapped_instance_with_2d_target_warns() {
    let src = format!("{HEADER}{MAPPED_2D}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let inst = scene.all_mapped_instances().next().expect("instance");
    assert!(inst.transform().is_none());
    assert_eq!(scene.warnings().len(), 1);
    assert!(scene.warnings()[0].contains("AXIS2_PLACEMENT_3D"));
}
