//! The raw graph and the raw entity stream: file id order, input chunking,
//! repeated ids, and how a file ends.

use std::collections::BTreeMap;
use std::io::{self, Read};

use step_io::parser::{Attribute, RawEntity, parse, stream};

const HEADER: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION(('raw graph'),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

struct Chunks<'a>(&'a [u8], usize);

impl Read for Chunks<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = buffer.len().min(self.0.len()).min(self.1);
        buffer[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

#[test]
fn unordered_ids_preserve_values_spans_and_header_across_chunks() {
    let body = "#90=UNKNOWN('a;''b',(#3,(LENGTH_MEASURE(1.25),$,*)),.T.);\n\
                #3=CARTESIAN_POINT('',(1.,2.,3.));\n\
                #20=(ALPHA('x') BETA((#90,#3)));\n";
    let source = format!("{HEADER}{body}{FOOTER}");
    let whole = parse(source.as_bytes()).unwrap();
    assert_eq!(
        whole.entities.keys().copied().collect::<Vec<_>>(),
        [3, 20, 90]
    );
    let RawEntity::Simple { attributes, .. } = &whole.entities[&90] else {
        panic!("simple entity expected");
    };
    assert_eq!(attributes[0], Attribute::String("a;'b".into()));
    assert_eq!(
        usize::try_from(whole.entities[&90].span().start).unwrap(),
        HEADER.len()
    );
    for size in [1, 7, 64, 65_536] {
        let chunks = parse(Chunks(source.as_bytes(), size)).unwrap();
        assert_eq!(chunks.entities, whole.entities);
        assert_eq!(chunks.header, whole.header);
        assert_eq!(chunks.warnings, whole.warnings);
        assert_eq!(chunks.anchors, whole.anchors);
        assert_eq!(chunks.external_references, whole.external_references);
        assert_eq!(chunks.schema, whole.schema);
    }
}

#[test]
fn first_repeat_in_file_order_precedes_later_parse_errors() {
    let body = "#90=A();\n#3=B();\n#90=C();\n#3=D();\n#90=E();\n";
    for footer in [FOOTER, "ENDSEC;\n", "#5=A(\n"] {
        let source = format!("{HEADER}{body}{footer}");
        for size in [1, 7, 65_536] {
            assert!(matches!(
                parse(Chunks(source.as_bytes(), size)),
                Err(step_io::Error::DuplicateEntityId {
                    id: 90,
                    line: 10,
                    first_line: 8
                })
            ));
        }
    }
    let source = format!("{HEADER}#90=A(;\n#90=B();\n{FOOTER}");
    assert!(
        matches!(parse(source.as_bytes()), Err(e) if !matches!(e, step_io::Error::DuplicateEntityId { .. }))
    );
}

#[test]
fn duplicate_precedence_preserves_an_earlier_input_error() {
    struct Failing;
    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("input disconnected"))
        }
    }
    let repeated = format!("{HEADER}#90=A();\n#3=B();\n#90=C();\n");
    assert!(matches!(
        parse(Chunks(repeated.as_bytes(), 1).chain(Failing)),
        Err(step_io::Error::DuplicateEntityId {
            id: 90,
            line: 10,
            first_line: 8
        })
    ));
    let unique = format!("{HEADER}#90=A();\n#3=B();\n");
    let error = parse(Chunks(unique.as_bytes(), 1).chain(Failing)).unwrap_err();
    assert!(
        matches!(&error, step_io::Error::Io(inner) if inner.to_string() == "input disconnected")
    );
}

/// `HEADER` with the edition 3 sections, which come before `DATA;`.
const ED3_HEADER: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION(('raw graph'),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\n\
ANCHOR;\n<Origin>=#3;\nENDSEC;\n\
REFERENCE;\n#123=<other.stp#Point>;\nENDSEC;\nDATA;\n";

/// Whether an error is the one expected.
type ErrorCheck = fn(&step_io::Error) -> bool;

/// Ends that are not `ENDSEC; END-ISO-10303-21;` and nothing after, each with
/// the error it is.
fn bad_trailers() -> [(&'static str, ErrorCheck); 2] {
    [
        ("ENDSEC;\n", |e| {
            matches!(e, step_io::Error::UnexpectedEof { .. })
        }),
        ("ENDSEC;\nEND-ISO-10303-21;\n#7=A();\n", |e| {
            matches!(
                e,
                step_io::Error::UnexpectedToken {
                    expected: "end of file",
                    ..
                }
            )
        }),
    ]
}

#[test]
fn stream_yields_in_file_order_what_parse_collects() {
    let body = "#90=UNKNOWN('a',(#3,$),.T.);\n\
                #3=CARTESIAN_POINT('',(1.,,3.));\n\
                #20=(ALPHA('x') BETA((#90,#123)));\n";
    let source = format!("{ED3_HEADER}{body}{FOOTER}");
    let graph = parse(source.as_bytes()).unwrap();
    assert_eq!(graph.warnings.len(), 1);
    assert_eq!(graph.anchors.len(), 1);
    assert_eq!(graph.external_references.len(), 1);
    for size in [1, 7, 65_536] {
        let mut entities = stream(Chunks(source.as_bytes(), size)).unwrap();
        assert_eq!(entities.schema(), &graph.schema);
        assert_eq!(entities.header(), graph.header.as_slice());
        assert_eq!(entities.anchors(), graph.anchors.as_slice());
        assert_eq!(entities.external_references(), &graph.external_references);
        let read: Vec<RawEntity> = (&mut entities).collect::<Result<_, _>>().unwrap();
        assert_eq!(
            read.iter().map(RawEntity::id).collect::<Vec<_>>(),
            [90, 3, 20]
        );
        let by_id: BTreeMap<u64, RawEntity> = read.into_iter().map(|e| (e.id(), e)).collect();
        assert_eq!(by_id, graph.entities);
        assert_eq!(entities.warnings(), graph.warnings.as_slice());
        assert!(entities.next().is_none());
    }
}

#[test]
fn stream_passes_a_repeated_id_through() {
    let source = format!("{HEADER}#90=A();\n#3=B();\n#90=C();\n{FOOTER}");
    let ids: Vec<u64> = stream(source.as_bytes())
        .unwrap()
        .map(|e| e.unwrap().id())
        .collect();
    assert_eq!(ids, [90, 3, 90]);
    assert!(matches!(
        parse(source.as_bytes()),
        Err(step_io::Error::DuplicateEntityId { id: 90, .. })
    ));
}

#[test]
fn a_bad_trailer_is_the_last_item_and_fails_parse() {
    for (trailer, is_expected) in bad_trailers() {
        let source = format!("{HEADER}#90=A();\n#3=B();\n{trailer}");
        let mut entities = stream(source.as_bytes()).unwrap();
        assert!(matches!(entities.next(), Some(Ok(_))));
        assert!(matches!(entities.next(), Some(Ok(_))));
        assert!(
            matches!(entities.next(), Some(Err(e)) if is_expected(&e)),
            "trailer {trailer:?}"
        );
        assert!(entities.next().is_none());
        assert!(
            matches!(parse(source.as_bytes()), Err(e) if is_expected(&e)),
            "trailer {trailer:?}"
        );
    }
}

#[test]
fn stream_ends_after_a_malformed_entity() {
    let source = format!("{HEADER}#90=A();\n#3=B(;\n#4=C();\n{FOOTER}");
    let mut entities = stream(source.as_bytes()).unwrap();
    assert!(matches!(entities.next(), Some(Ok(_))));
    assert!(matches!(entities.next(), Some(Err(_))));
    assert!(entities.next().is_none());
    assert!(entities.next().is_none());
}

#[test]
fn a_malformed_header_fails_to_open_a_stream() {
    let source = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(";
    assert!(stream(source.as_bytes()).is_err());
}
