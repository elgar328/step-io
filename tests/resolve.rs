//! Forward Ref resolver spike: holding a reference (a `LINE`'s `pnt` /
//! `dir`), `resolve(&model)` returns the concrete target object — the forward
//! dual of `RefGraph`'s backward `referrers`. Uses a list-free chain that reads
//! cleanly.

use step_io::generated::resolve::{CartesianPointRefView, VectorRefView};
use step_io::read;

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const BODY: &str = "\
#1=CARTESIAN_POINT('',(2.0,0.0,0.0));\n\
#2=DIRECTION('',(1.0,0.0,0.0));\n\
#3=VECTOR('',#2,1.0);\n\
#4=LINE('',#1,#3);\n";

#[test]
fn resolves_forward_references_to_concrete_targets() {
    let src = format!("{HEADER}{BODY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");

    let line = model.line_arena.get(0);

    // LINE.pnt is a CartesianPointRef → its concrete CARTESIAN_POINT.
    match line.pnt.resolve(&model) {
        CartesianPointRefView::CartesianPoint(p) => assert_eq!(p.coordinates, [2.0, 0.0, 0.0]),
        other => panic!("expected a cartesian point, got {other:?}"),
    }

    // LINE.dir is a VectorRef → its concrete VECTOR.
    assert!(matches!(line.dir.resolve(&model), VectorRefView::Vector(_)));
}
