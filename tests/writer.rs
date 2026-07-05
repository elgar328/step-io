//! `write` is a pure serializer — the model's own header (`FILE_SCHEMA`
//! included) plus every entity, verbatim, lossless. `dump_universal` re-marks
//! the model as step-io's non-standard universal union (`FILE_SCHEMA` and the
//! APD/`application_context` entities agree). Inline STEP docs, not valid-CAD
//! fixtures.

use step_io::{dump_universal, read, write};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn doc(body: &str) -> String {
    format!("{HEADER}{body}{FOOTER}")
}

#[test]
fn write_reserializes_faithfully_under_source_schema() {
    // No projection: every kept entity survives the write, and the source
    // FILE_SCHEMA is echoed — step-io does not convert between APs.
    let src = doc("#1=PRE_DEFINED_PRESENTATION_STYLE('pps');\n#2=STATE_OBSERVED('st',$);\n");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(rep.dropped.len(), 0, "input clean: {:?}", rep.dropped);

    let out = write(&model);
    assert!(out.contains("STATE_OBSERVED"), "{out}");
    assert!(out.contains("PRE_DEFINED_PRESENTATION_STYLE"), "{out}");
    assert!(
        out.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'))"),
        "source schema echoed:\n{out}"
    );

    // The output re-reads with zero drops.
    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "output not clean: {:?}",
        rep2.dropped
    );
}

#[test]
fn dump_universal_marks_header_and_apd() {
    // Input declares AP214-flavoured APD/AC; the dump absolutely sets BOTH the
    // FILE_SCHEMA header and the APD/AC entities to the universal marker so
    // the dump is internally consistent (header ↔ APD agree).
    let body = "#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
                #2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n";
    let (mut model, _) = read(doc(body).as_bytes()).expect("read");

    let uni = dump_universal(&mut model);
    assert!(uni.contains("FILE_SCHEMA(('STEPIO_UNIVERSAL'))"), "{uni}");
    assert!(
        uni.contains("APPLICATION_PROTOCOL_DEFINITION('not a standard','stepio_universal',0"),
        "{uni}"
    );
    assert!(
        uni.contains(
            "APPLICATION_CONTEXT('step-io universal union (non-standard, all-AP superset)')"
        ),
        "{uni}"
    );
}
