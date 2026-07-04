//! `read()` surfaces the identified source schema in `Report.schema` — how a
//! caller (e.g. a CAD kernel) learns the file's precise AP / edition / stage.
//! Inline STEP docs, not valid-CAD fixtures.

use step_io::{ApFamily, Stage, read};

fn doc_with_schema(file_schema: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\n\
         FILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\n\
         FILE_SCHEMA(('{file_schema}'));\n\
         ENDSEC;\nDATA;\n#1=APPLICATION_CONTEXT('');\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

#[test]
fn read_reports_ap242_ed2() {
    let src =
        doc_with_schema("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }");
    let (_model, report) = read(src.as_bytes()).expect("read ok");
    assert_eq!(report.schema.family, ApFamily::Ap242);
    assert_eq!(report.schema.edition, Some(2));
    assert_eq!(report.schema.stage, Stage::Is);
    assert_eq!(report.schema.to_string(), "AP242 ed2 (IS)");
    // Raw FILE_SCHEMA preserved verbatim for byte-exact round-trip.
    assert!(
        report
            .schema
            .raw()
            .is_some_and(|r| r.as_slice()[0].contains("442 3 1 4"))
    );
}

#[test]
fn read_reports_ap214_ed3() {
    let src = doc_with_schema("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }");
    let (_model, report) = read(src.as_bytes()).expect("read ok");
    assert_eq!(report.schema.family, ApFamily::Ap214);
    assert_eq!(report.schema.edition, Some(3));
    assert_eq!(report.schema.stage, Stage::Is);
}

#[test]
fn read_reports_unrecognized_schema() {
    let src = doc_with_schema("SOME_PRIVATE_SCHEMA");
    let (_model, report) = read(src.as_bytes()).expect("read ok");
    assert_eq!(report.schema.family, ApFamily::Other);
    assert!(!report.schema.is_recognized());
}
