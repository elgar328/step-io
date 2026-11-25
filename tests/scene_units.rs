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

/// The uncertainty written as a complex instance with its length measure.
#[test]
fn reads_a_complex_uncertainty_as_the_precision() {
    let units = UNITS.replace(
        "#12=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#10,'closure','');",
        "#12=(LENGTH_MEASURE_WITH_UNIT()MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#10)\
         UNCERTAINTY_MEASURE_WITH_UNIT('closure',''));",
    );
    let src = format!("{HEADER}{units}{FOOTER}");
    let (model, report) = read(src.as_bytes()).expect("read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let precision = model.scene().units().precision.expect("a precision");
    assert!((precision - 0.01).abs() < 1e-12, "precision = {precision}");
}

/// An inch whose conversion factor is a complex measure or a positive length
/// measure: both resolve to 25.4 mm.
#[test]
fn reads_a_complex_or_positive_conversion_factor() {
    for factor in [
        "#20=(LENGTH_MEASURE_WITH_UNIT()MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#10));",
        "#20=POSITIVE_LENGTH_MEASURE_WITH_UNIT(POSITIVE_LENGTH_MEASURE(25.4),#10);",
    ] {
        let units = UNITS.replace(
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11))",
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#21,#11))",
        ) + factor
            + "\n#22=DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n\
               #21=( CONVERSION_BASED_UNIT('INCH',#20) LENGTH_UNIT() NAMED_UNIT(#22) );\n";
        let src = format!("{HEADER}{units}{FOOTER}");
        let (model, report) = read(src.as_bytes()).expect("read");
        assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
        let length = model.scene().units().length.expect("a length unit");
        assert_eq!(length.name, "INCH");
        assert!(
            (length.to_si - 0.0254).abs() < 1e-12,
            "inch to_si = {}",
            length.to_si
        );
    }
}

/// A length unit that is neither SI nor conversion-based cannot be read; the
/// warning names the unit context.
#[test]
fn an_unreadable_length_unit_is_reported() {
    let units = UNITS.replace(
        "#10=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
        "#10=( CONTEXT_DEPENDENT_UNIT('foot') LENGTH_UNIT() NAMED_UNIT(#15) );\n\
         #15=DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);",
    );
    let src = format!("{HEADER}{units}{FOOTER}");
    let (model, report) = read(src.as_bytes()).expect("read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    let warnings = scene.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(matches!(
        warnings[0].entity,
        Some(step_io::model::EntityKey::ComplexEntity(_))
    ));
}
