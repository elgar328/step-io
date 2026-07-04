//! Part 21 HEADER authoring through `StepBuilder::header`: explicit fields
//! land verbatim (with escaping), the timestamp and preprocessor stamp are
//! automatic by default, and header parsing stays harmless on re-read.

use step_io::build::HeaderInput;
use step_io::{StepBuilder, read};

#[test]
fn explicit_header_lands_verbatim() {
    let mut b = StepBuilder::new().expect("builder");
    b.header(&HeaderInput {
        file_name: Some("wheel.step".to_owned()),
        description: Some("wheel assembly".to_owned()),
        authors: vec!["John".to_owned()],
        organizations: vec!["ACME".to_owned()],
        originating_system: Some("KernelX 2.1".to_owned()),
        timestamp: Some("2026-01-01T00:00:00Z".to_owned()),
        ..Default::default()
    });
    b.part("wheel").expect("part");
    let text = b.finish().expect("finish");

    let expected_file_name = format!(
        "FILE_NAME('wheel.step','2026-01-01T00:00:00Z',('John'),('ACME'),'step-io {}','KernelX 2.1','');",
        env!("CARGO_PKG_VERSION")
    );
    assert!(text.contains(&expected_file_name), "header: {text}");
    assert!(
        text.contains("FILE_DESCRIPTION(('wheel assembly'),'2;1');"),
        "description: {text}"
    );

    let (_model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
}

#[test]
fn default_header_stamps_time_and_preprocessor() {
    let b = StepBuilder::new().expect("builder");
    let text = b.finish().expect("finish");

    // FILE_NAME('','<timestamp>',...) — second field auto-stamped.
    let line = text
        .lines()
        .find(|l| l.starts_with("FILE_NAME("))
        .expect("FILE_NAME line");
    let stamp = line.split('\'').nth(3).expect("timestamp field");
    assert!(!stamp.is_empty(), "timestamp must be auto-stamped: {line}");
    assert!(
        stamp.contains('T') && stamp.ends_with('Z'),
        "ISO UTC form: {stamp}"
    );
    assert!(line.contains("'step-io "), "preprocessor stamp: {line}");
}

#[test]
fn header_strings_are_escaped() {
    let mut b = StepBuilder::new().expect("builder");
    b.header(&HeaderInput {
        authors: vec!["O'Brien".to_owned()],
        timestamp: Some(String::new()),
        ..Default::default()
    });
    let text = b.finish().expect("finish");
    assert!(text.contains("('O''Brien')"), "escaped author: {text}");

    // Explicit empty timestamp stays blank (reproducible-output escape hatch).
    let line = text
        .lines()
        .find(|l| l.starts_with("FILE_NAME("))
        .expect("FILE_NAME line");
    assert!(line.starts_with("FILE_NAME('','',"), "blank stamp: {line}");
}
