//! Units API: read the length / angle units and precision from a model's
//! representation context. Both the context and the units are STEP complex
//! instances (the context bundles the geometric / unit / uncertainty facets;
//! each unit bundles its dimension marker with an `SI_UNIT`), so this exercises
//! the two-level complex-part traversal. A shape representation anchors the
//! context so settling does not prune it.

use step_io::read;

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const UNITS: &str = "\
#10=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#11=( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
#12=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#10,'closure','');\n\
#13=( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#12)) \
GLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11)) REPRESENTATION_CONTEXT('','') );\n\
#14=SHAPE_REPRESENTATION('',(),#13);\n";

#[test]
fn reads_length_angle_units_and_precision() {
    let src = format!("{HEADER}{UNITS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let units = scene.units();

    let length = units.length.expect("a length unit");
    assert!(
        length.name.contains("METRE"),
        "length name = {}",
        length.name
    );
    assert!(
        (length.to_si - 1e-3).abs() < 1e-12,
        "millimetre to_si = {}",
        length.to_si
    );

    let angle = units.angle.expect("an angle unit");
    assert!(
        (angle.to_si - 1.0).abs() < 1e-12,
        "radian to_si = {}",
        angle.to_si
    );

    let precision = units.precision.expect("a precision");
    assert!((precision - 0.01).abs() < 1e-12, "precision = {precision}");
}
