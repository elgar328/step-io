//! Units API: read the length / angle units and precision from a model's
//! representation context. Both the context and the units are STEP complex
//! instances (the context bundles the geometric / unit / uncertainty facets;
//! each unit bundles its dimension marker with an `SI_UNIT`), so this exercises
//! the two-level complex-part traversal. A shape representation uses the
//! context.

use step_io::model::{EntityKey, ShapeRepresentationId};
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
    let subject = warnings[0].entity.expect("the warning names an entity");
    assert!(matches!(subject, EntityKey::ComplexEntity(_)));
    // The context, which the shape representation references.
    assert!(
        model
            .ref_graph()
            .referrers(subject)
            .contains(&EntityKey::ShapeRepresentation(ShapeRepresentationId(0))),
        "{subject:?} is not the unit context"
    );
}

/// The model's units with its length unit replaced by `#21`, defined by `extra`.
fn units_with_length(extra: &str) -> (step_io::StepModel, step_io::Report) {
    let units = UNITS.replace(
        "GLOBAL_UNIT_ASSIGNED_CONTEXT((#10,#11))",
        "GLOBAL_UNIT_ASSIGNED_CONTEXT((#21,#11))",
    ) + "#22=DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n"
        + extra;
    let src = format!("{HEADER}{units}{FOOTER}");
    let (model, report) = read(src.as_bytes()).expect("read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    (model, report)
}

/// A conversion-based unit whose factor is expressed in the unit itself stops
/// there and is reported, where it used to recurse until the stack overflowed.
#[test]
fn a_unit_defined_through_itself_is_reported() {
    let (model, _) = units_with_length(
        "#20=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#21);\n\
         #21=( CONVERSION_BASED_UNIT('INCH',#20) LENGTH_UNIT() NAMED_UNIT(#22) );\n",
    );
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    let warnings = scene.warnings();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("INCH: defined through itself")),
        "{warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("INCH: its base unit is not read")),
        "{warnings:?}"
    );
}

/// A factor that cannot be read is the only thing reported: the unit does have
/// its conversion-based part.
#[test]
fn an_unreadable_conversion_factor_is_reported_once() {
    let (model, _) = units_with_length(
        "#20=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() REPRESENTATION_ITEM('') );\n\
         #21=( CONVERSION_BASED_UNIT('INCH',#20) LENGTH_UNIT() NAMED_UNIT(#22) );\n",
    );
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    let warnings = scene.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].message.contains("conversion factor"),
        "{warnings:?}"
    );
}

/// A length unit converted from an area unit, which `Units` does not read, is
/// reported.
#[test]
fn a_length_unit_with_a_base_of_another_kind_is_reported() {
    let (model, _) = units_with_length(
        "#24=DERIVED_UNIT_ELEMENT(#10,2.);\n\
         #23=AREA_UNIT((#24));\n\
         #20=MEASURE_WITH_UNIT(AREA_MEASURE(2.),#23);\n\
         #21=( CONVERSION_BASED_UNIT('ODD',#20) LENGTH_UNIT() NAMED_UNIT(#22) );\n",
    );
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    let warnings = scene.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].message.contains("its base unit is not read"),
        "{warnings:?}"
    );
}

/// A length unit whose conversion leads to a unit of another kind (a second)
/// is reported, where it used to be left out silently.
#[test]
fn a_length_unit_of_another_kind_is_reported() {
    let (model, _) = units_with_length(
        "#23=( NAMED_UNIT(*) SI_UNIT($,.SECOND.) TIME_UNIT() );\n\
         #20=MEASURE_WITH_UNIT(TIME_MEASURE(60.),#23);\n\
         #21=( CONVERSION_BASED_UNIT('MINUTE',#20) LENGTH_UNIT() NAMED_UNIT(#22) );\n",
    );
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    let warnings = scene.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].message.contains("a unit of another kind"),
        "{warnings:?}"
    );
}

/// An area unit is not one `Units` holds: it is skipped without a report, even
/// when converted from another unit.
#[test]
fn an_area_unit_is_skipped_quietly() {
    let (model, _) = units_with_length(
        "#24=DERIVED_UNIT_ELEMENT(#10,2.);\n\
         #23=AREA_UNIT((#24));\n\
         #20=MEASURE_WITH_UNIT(AREA_MEASURE(645.16),#23);\n\
         #21=( CONVERSION_BASED_UNIT('SQUARE INCH',#20) NAMED_UNIT(#22) );\n",
    );
    let scene = model.scene();
    assert!(scene.units().length.is_none());
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}
