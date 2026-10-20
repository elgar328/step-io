//! Part 21 HEADER authoring through `StepBuilder::header`: explicit fields
//! land verbatim (with escaping), the timestamp and preprocessor stamp are
//! automatic by default, and header parsing stays harmless on re-read.

mod common;

use step_io::build::HeaderInput;
use step_io::{StepBuilder, read};

#[test]
fn explicit_header_lands_verbatim() {
    let mut b = StepBuilder::new();
    b.header(&HeaderInput {
        file_name: Some("wheel.step".to_owned()),
        description: Some("wheel assembly".to_owned()),
        authors: vec!["John".to_owned()],
        organizations: vec!["ACME".to_owned()],
        originating_system: Some("KernelX 2.1".to_owned()),
        timestamp: Some("2026-01-01T00:00:00Z".to_owned()),
        ..Default::default()
    });
    b.part("wheel");
    let text = common::text(&b.finish());

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
    let b = StepBuilder::new();
    let text = common::text(&b.finish());

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
    let mut b = StepBuilder::new();
    b.header(&HeaderInput {
        authors: vec!["O'Brien".to_owned()],
        timestamp: Some(String::new()),
        ..Default::default()
    });
    let text = common::text(&b.finish());
    assert!(text.contains("('O''Brien')"), "escaped author: {text}");

    // Explicit empty timestamp stays blank (reproducible-output escape hatch).
    let line = text
        .lines()
        .find(|l| l.starts_with("FILE_NAME("))
        .expect("FILE_NAME line");
    assert!(line.starts_with("FILE_NAME('','',"), "blank stamp: {line}");
}

#[test]
fn non_ascii_and_backslashes_round_trip_through_plain_ascii() {
    let file_name = "부품 ①.step";
    let description = "it's a \\ test 😀\nsecond line";
    let author = "홍길동";
    let part_name = r"C:\parts\받침대";

    let mut b = StepBuilder::new();
    b.header(&HeaderInput {
        file_name: Some(file_name.to_owned()),
        description: Some(description.to_owned()),
        authors: vec![author.to_owned()],
        timestamp: Some("2026-01-01T00:00:00Z".to_owned()),
        ..Default::default()
    });
    b.part(part_name);
    let text = common::text(&b.finish());
    assert!(text.is_ascii(), "non-ASCII output:\n{text}");
    assert!(
        text.contains(r"'\X2\BD80D488\X0\ \X2\2460\X0\.step'"),
        "file name: {text}"
    );
    assert!(
        text.contains(r"'C:\\parts\\\X2\BC1BCE68B300\X0\'"),
        "part: {text}"
    );

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert!(
        report.parse_warnings.is_empty(),
        "{:?}",
        report.parse_warnings
    );
    let header = model.header();
    assert_eq!(header.file_name, file_name);
    assert_eq!(header.description, description);
    assert_eq!(header.authors, [author]);
    let scene = model.scene();
    let names: Vec<_> = scene.all_products().map(|p| p.name().to_owned()).collect();
    assert_eq!(names, [part_name]);
}
