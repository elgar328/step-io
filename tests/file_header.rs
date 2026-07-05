//! `read()` decodes the full Part 21 HEADER into `model.header()` —
//! `FILE_DESCRIPTION`, `FILE_NAME`, and `FILE_SCHEMA` (as the identified
//! schema). Inline STEP docs, not valid-CAD fixtures.

use step_io::{ApFamily, StepBuilder, read, write};

#[test]
fn read_fills_model_header() {
    let src = "ISO-10303-21;\nHEADER;\n\
         FILE_DESCRIPTION(('a wheel model'),'2;1');\n\
         FILE_NAME('wheel.step','2026-07-05T12:00:00',('Alice','Bob'),('ACME'),\
         'kernel 1.2','ACME CAD','release');\n\
         FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
         ENDSEC;\nDATA;\n#1=APPLICATION_CONTEXT('');\nENDSEC;\nEND-ISO-10303-21;\n";
    let (model, _report) = read(src.as_bytes()).expect("read ok");
    let h = model.header();
    assert_eq!(h.description, "a wheel model");
    assert_eq!(h.file_name, "wheel.step");
    assert_eq!(h.time_stamp, "2026-07-05T12:00:00");
    assert_eq!(h.authors, ["Alice", "Bob"]);
    assert_eq!(h.organizations, ["ACME"]);
    assert_eq!(h.preprocessor_version, "kernel 1.2");
    assert_eq!(h.originating_system, "ACME CAD");
    assert_eq!(h.authorisation, "release");
    assert_eq!(h.schema.family, ApFamily::Ap242);
    assert_eq!(h.schema.edition, Some(2));
}

#[test]
fn write_round_trips_header() {
    // read → write → re-read: the header (FILE_SCHEMA included) survives.
    let src = "ISO-10303-21;\nHEADER;\n\
         FILE_DESCRIPTION(('a wheel model'),'2;1');\n\
         FILE_NAME('wheel.step','2026-07-05T12:00:00',('Alice'),('ACME'),\
         'kernel 1.2','ACME CAD','release');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
         ENDSEC;\nDATA;\n#1=APPLICATION_CONTEXT('');\nENDSEC;\nEND-ISO-10303-21;\n";
    let (model, _report) = read(src.as_bytes()).expect("read ok");
    let out = write(&model);
    let (m2, _report2) = read(out.as_bytes()).expect("re-read ok");
    let h = m2.header();
    assert_eq!(h.description, "a wheel model");
    assert_eq!(h.file_name, "wheel.step");
    assert_eq!(h.time_stamp, "2026-07-05T12:00:00");
    assert_eq!(h.authors, ["Alice"]);
    assert_eq!(h.organizations, ["ACME"]);
    assert_eq!(h.preprocessor_version, "kernel 1.2");
    assert_eq!(h.originating_system, "ACME CAD");
    assert_eq!(h.authorisation, "release");
    assert_eq!(h.schema.family, ApFamily::Ap214);
    assert_eq!(h.schema.edition, Some(3));
}

#[test]
fn builder_output_carries_ap242_schema() {
    // The authoring layer stamps the AP242 ed2 identity on the header.
    let mut b = StepBuilder::new().expect("builder");
    b.part("wheel").expect("part");
    let text = b.finish().expect("finish");
    assert!(
        text.contains(
            "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'))"
        ),
        "{text}"
    );
    let (model, _report) = read(text.as_bytes()).expect("re-read ok");
    assert_eq!(model.header().schema.family, ApFamily::Ap242);
    assert_eq!(model.header().schema.edition, Some(2));
}

#[test]
fn read_tolerates_sparse_header() {
    // Unset (`$`) and empty fields read as empty, never fail the read.
    let src = "ISO-10303-21;\nHEADER;\n\
         FILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('',$,(''),(''),$,'','');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
         ENDSEC;\nDATA;\n#1=APPLICATION_CONTEXT('');\nENDSEC;\nEND-ISO-10303-21;\n";
    let (model, _report) = read(src.as_bytes()).expect("read ok");
    let h = model.header();
    assert_eq!(h.file_name, "");
    assert_eq!(h.time_stamp, "");
    assert!(h.authors.is_empty());
    assert!(h.organizations.is_empty());
    assert_eq!(h.preprocessor_version, "");
    assert_eq!(h.schema.family, ApFamily::Ap214);
}
