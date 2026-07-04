//! Tessellated (display-mesh) handles: `scene.all_meshes()` exposes vertex
//! positions, triangles expanded from strips/fans, normals, and the link back
//! to the precise b-rep face.

use step_io::read;
use step_io::scene::MeshNormals;

fn approx3(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
}

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

// #11 a strip mesh (one quad as a 4-vertex strip), #12 a fan mesh, #14 a
// pnindex mesh picking 3 of 6 coordinates, #15 a mesh linked to a b-rep face.
const MESHES: &str = "\
#10=COORDINATES_LIST('',4,((0.,0.,0.),(1.,0.,0.),(0.,1.,0.),(1.,1.,0.)));\n\
#11=COMPLEX_TRIANGULATED_FACE('quad',#10,4,((0.,0.,1.)),$,(),((1,2,3,4)),());\n\
#12=COMPLEX_TRIANGULATED_FACE('fan',#10,4,(),$,(),(),((1,2,3,4)));\n\
#13=COORDINATES_LIST('',6,((0.,0.,0.),(9.,9.,9.),(1.,0.,0.),(9.,9.,9.),(0.,1.,0.),(9.,9.,9.)));\n\
#14=COMPLEX_TRIANGULATED_FACE('picked',#13,3,(),$,(1,3,5),((1,2,3)),());\n\
#15=COMPLEX_TRIANGULATED_FACE('linked',#10,4,(),#37,(),((1,2,3)),());\n\
#20=REPRESENTATION_CONTEXT('','');\n\
#21=TESSELLATED_SHAPE_REPRESENTATION('',(#11,#12,#14,#15),#20);\n\
#30=CARTESIAN_POINT('',(0.,0.,0.));\n\
#31=DIRECTION('',(0.,0.,1.));\n\
#32=DIRECTION('',(1.,0.,0.));\n\
#33=AXIS2_PLACEMENT_3D('',#30,#31,#32);\n\
#34=PLANE('',#33);\n\
#35=CARTESIAN_POINT('',(1.,0.,0.));\n\
#40=VERTEX_POINT('',#30);\n\
#41=VERTEX_POINT('',#35);\n\
#42=DIRECTION('',(1.,0.,0.));\n\
#43=VECTOR('',#42,1.0);\n\
#44=LINE('',#30,#43);\n\
#45=EDGE_CURVE('',#40,#41,#44,.T.);\n\
#46=ORIENTED_EDGE('',*,*,#45,.T.);\n\
#47=EDGE_LOOP('',(#46));\n\
#48=FACE_OUTER_BOUND('',#47,.T.);\n\
#37=ADVANCED_FACE('brep face',(#48),#34,.T.);\n\
#49=CLOSED_SHELL('',(#37));\n\
#50=MANIFOLD_SOLID_BREP('the part',#49);\n\
#80=TESSELLATED_SOLID('group A',(#11),#50);\n\
#81=TESSELLATED_SHELL('group B',(#12),$);\n\
#82=TESSELLATED_SHELL('group C',(#15),#49);\n";

#[test]
fn meshes_expose_points_triangles_and_normals() {
    let src = format!("{HEADER}{MESHES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let meshes: Vec<_> = scene.all_meshes().collect();
    assert_eq!(meshes.len(), 4);

    // The strip quad: two triangles with alternating winding unified.
    let quad = meshes.iter().find(|m| m.name() == "quad").expect("quad");
    assert_eq!(quad.points().len(), 4);
    assert!(approx3(quad.points()[3], [1., 1., 0.]));
    assert_eq!(quad.triangles(), vec![[0, 2, 1], [1, 2, 3]]);
    assert!(matches!(quad.normals(), MeshNormals::Uniform(n) if approx3(n, [0., 0., 1.])));
    assert!(quad.face().is_none());

    // The fan: centre 0, ring 1-2-3.
    let fan = meshes.iter().find(|m| m.name() == "fan").expect("fan");
    assert_eq!(fan.triangles(), vec![[0, 2, 1], [0, 3, 2]]);
    assert!(matches!(fan.normals(), MeshNormals::None));

    // pnindex picks coordinates 1, 3, 5 (1-based) out of six.
    let picked = meshes
        .iter()
        .find(|m| m.name() == "picked")
        .expect("picked");
    let picked_pts = picked.points();
    assert_eq!(picked_pts.len(), 3);
    for (got, want) in picked_pts
        .iter()
        .zip([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
    {
        assert!(approx3(*got, want));
    }
    assert_eq!(picked.triangles(), vec![[0, 2, 1]]);

    // The geometric link reaches the precise b-rep face.
    let linked = meshes
        .iter()
        .find(|m| m.name() == "linked")
        .expect("linked");
    let face = linked.face().expect("linked b-rep face");
    assert_eq!(face.name(), "brep face");

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn mesh_groups_bundle_faces_and_reach_the_solid() {
    let src = format!("{HEADER}{MESHES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let groups: Vec<_> = scene.all_mesh_groups().collect();
    assert_eq!(groups.len(), 3);

    // A tessellated solid: faces plus a direct geometric link to the b-rep.
    let a = groups.iter().find(|g| g.name() == "group A").expect("A");
    let meshes = a.meshes();
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].name(), "quad");
    assert_eq!(a.solid().expect("linked solid").name(), "the part");

    // A shell without a topological link: no solid.
    let b = groups.iter().find(|g| g.name() == "group B").expect("B");
    assert_eq!(b.meshes()[0].name(), "fan");
    assert!(b.solid().is_none());

    // A shell linked to a closed shell: reached back through its owner.
    let c = groups.iter().find(|g| g.name() == "group C").expect("C");
    assert_eq!(c.solid().expect("owner solid").name(), "the part");

    // The reverse direction: a mesh knows its group; an unbundled mesh has none.
    let quad = scene
        .all_meshes()
        .find(|m| m.name() == "quad")
        .expect("quad");
    assert_eq!(quad.group().expect("group").name(), "group A");
    let picked = scene
        .all_meshes()
        .find(|m| m.name() == "picked")
        .expect("picked");
    assert!(picked.group().is_none());

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}
