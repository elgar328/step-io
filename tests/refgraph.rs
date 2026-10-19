//! `RefGraph` (reverse-reference index) spike: a `CARTESIAN_POINT` referenced by a
//! `LINE` (via `pnt`) and a `VERTEX_POINT` (via `vertex_geometry`) should report
//! both as its referrers — built mechanically over every entity type.

use step_io::EntityKey;
use step_io::generated::model::{CartesianPointId, LineId, PolylineId};
use step_io::read;

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const BODY: &str = "\
#1=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#2=DIRECTION('',(1.0,0.0,0.0));\n\
#3=VECTOR('',#2,1.0);\n\
#4=LINE('',#1,#3);\n\
#5=VERTEX_POINT('',#1);\n";

#[test]
fn reverse_index_reports_referrers() {
    let src = format!("{HEADER}{BODY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");

    // The lone cartesian point is arena index 0.
    let cp = EntityKey::CartesianPoint(CartesianPointId(0));
    let graph = model.ref_graph();
    let refs = graph.referrers(cp);

    assert!(
        refs.iter().any(|a| matches!(a, EntityKey::Line(_))),
        "LINE.pnt references the point: {refs:?}"
    );
    assert!(
        refs.iter().any(|a| matches!(a, EntityKey::VertexPoint(_))),
        "VERTEX_POINT.vertex_geometry references the point: {refs:?}"
    );

    // An unreferenced entity has no referrers (the line is a leaf here).
    // (Sanity: a target with no referrers returns an empty slice.)
    assert!(
        graph
            .referrers(EntityKey::CartesianPoint(CartesianPointId(999)))
            .is_empty(),
        "out-of-range / unreferenced target yields empty"
    );
}

#[test]
fn referrers_come_in_walk_order_with_repeats() {
    let body = "\
#1=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#2=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#3=DIRECTION('',(1.0,0.0,0.0));\n\
#4=VECTOR('',#3,1.0);\n\
#5=VERTEX_POINT('',#1);\n\
#6=LINE('',#1,#4);\n\
#7=POLYLINE('',(#1,#2,#1));\n";
    let src = format!("{HEADER}{body}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "{:?}", rep.dropped);
    let graph = model.ref_graph();

    // Entities are walked kind by kind (LINE, POLYLINE, VERTEX_POINT), each in
    // file order; a polyline that lists the point twice references it twice.
    let first = EntityKey::CartesianPoint(CartesianPointId(0));
    let names: Vec<String> = graph
        .referrers(first)
        .iter()
        .map(|k| format!("{k:?}"))
        .collect();
    assert_eq!(
        names,
        [
            "Line(LineId(0))",
            "Polyline(PolylineId(0))",
            "Polyline(PolylineId(0))",
            "VertexPoint(VertexPointId(0))"
        ]
    );

    // Referenced by nobody, though in range: the line and the polyline.
    assert_eq!(graph.referrers(EntityKey::Line(LineId(0))), []);
    assert_eq!(graph.referrers(EntityKey::Polyline(PolylineId(0))), []);
}
