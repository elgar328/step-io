//! Raw graph identity is independent of file id order and input chunking.

use std::io::{self, Read};

use step_io::parser::{Attribute, RawEntity, parse};

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
