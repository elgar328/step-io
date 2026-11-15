//! Lossless packed raw records keep the ordinary graph's complete observations.

use std::io::{self, Read};

use step_io::parser::{Attribute, RawEntity, parse, parse_compact};

const HEADER: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION(('compact graph'),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

struct Chunks<'a>(&'a [u8], usize);

impl Read for Chunks<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = out.len().min(self.0.len()).min(self.1);
        out[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

fn equal(source: &[u8], chunks: usize) {
    let raw = parse(Chunks(source, chunks)).unwrap();
    let packed = parse_compact(Chunks(source, chunks)).unwrap();
    assert_eq!(packed.len(), raw.entities.len());
    assert_eq!(packed.is_empty(), raw.entities.is_empty());
    assert_eq!(packed.schema, raw.schema);
    assert_eq!(packed.header, raw.header);
    assert_eq!(packed.warnings, raw.warnings);
    assert_eq!(packed.external_references, raw.external_references);
    assert_eq!(packed.anchors, raw.anchors);
    assert_eq!(
        packed.ids().collect::<Vec<_>>(),
        raw.entities.keys().copied().collect::<Vec<_>>()
    );
    for (&id, original) in &raw.entities {
        assert_eq!(packed.entity(id).as_ref(), Some(original));
    }
    assert!(packed.entity(919_191).is_none());
}

#[test]
fn every_parameter_kind_and_full_width_ids_round_trip() {
    let source = format!(
        "{HEADER}DATA;\n\
         #18446744073709551615=UNKNOWN(-9223372036854775808,9223372036854775807,-0.,1.E-300,\
         'a;''b',.T.,\"0ABC\",#0,$,*,((#9),LENGTH_MEASURE(1.25)));\n\
         #0=(ALPHA('x') BETA((#18446744073709551615,#9)));\n#9=A();\n{FOOTER}"
    );
    for size in [1, 7, 64, 65_536] {
        equal(source.as_bytes(), size);
    }
    let packed = parse_compact(source.as_bytes()).unwrap();
    let RawEntity::Simple { attributes, .. } = packed.entity(u64::MAX).unwrap() else {
        panic!("simple entity expected");
    };
    assert!(
        matches!(attributes[2], Attribute::Real(value) if value.to_bits() == (-0.0f64).to_bits())
    );
}

#[test]
fn selected_queries_match_owned_records_without_expanding_siblings() {
    let text = "x".repeat(2 * 1024 * 1024 + 13);
    let source = format!(
        "{HEADER}DATA;\n#1=A('{text}',(1,(#9,-0.)),LENGTH_MEASURE(1.25),.T.);\n\
         #2=(ALPHA('{text}',((#1))) BETA(\"0ABC\",#1));\n{FOOTER}"
    );
    let graph = parse_compact(source.as_bytes()).unwrap();
    assert_eq!(graph.kind(1), Some("A"));
    assert_eq!(graph.kind(2), Some("COMPLEX"));
    assert_eq!(graph.kind(99), None);
    assert_eq!(
        graph.attribute_at(1, None, &[1, 1, 0]),
        Some(Attribute::EntityRef(9))
    );
    assert!(
        matches!(graph.attribute_at(1, None, &[1, 1, 1]), Some(Attribute::Real(v)) if v.to_bits() == (-0.0_f64).to_bits())
    );
    assert_eq!(
        graph.attribute_at(1, None, &[3]),
        Some(Attribute::Enum("T".into()))
    );
    assert_eq!(
        graph.attribute_at(2, Some("ALPHA"), &[1, 0, 0]),
        Some(Attribute::EntityRef(1))
    );
    assert_eq!(
        graph.attribute_at(2, Some("BETA"), &[1]),
        Some(Attribute::EntityRef(1))
    );
    for (id, part, path) in [
        (1, None, vec![]),
        (1, None, vec![4]),
        (1, None, vec![0, 0]),
        (1, Some("A"), vec![0]),
        (2, None, vec![0]),
        (2, Some("MISSING"), vec![0]),
        (99, None, vec![0]),
    ] {
        assert_eq!(graph.attribute_at(id, part, &path), None);
    }
}

#[test]
fn record_shape_is_not_inferred_from_an_unsupported_keyword() {
    let source = format!("{HEADER}DATA;\n#1=COMPLEX();\n#2=(ALPHA() BETA());\n{FOOTER}");
    let graph = parse_compact(source.as_bytes()).unwrap();
    assert_eq!(graph.kind(1), graph.kind(2));
    assert_eq!(graph.is_complex(1), Some(false));
    assert_eq!(graph.is_complex(2), Some(true));
    assert_eq!(graph.is_complex(3), None);
    equal(source.as_bytes(), 1);
}

#[test]
fn record_payload_can_cross_many_storage_blocks() {
    let text = "x".repeat(2 * 1024 * 1024 + 31);
    let source =
        format!("{HEADER}DATA;\n#3=UNKNOWN('{text}',(#1,#2));\n#2=A();\n#1=B();\n{FOOTER}");
    equal(source.as_bytes(), 65_536);
    let packed = parse_compact(source.as_bytes()).unwrap();
    assert!(packed.stored_bytes() > 2 * 1024 * 1024);
}

#[test]
fn large_unordered_index_keeps_original_spans_and_values() {
    use std::fmt::Write;
    let mut source = format!("{HEADER}DATA;\n");
    for i in 0..65_536_u64 {
        let id = (i * 37) % 65_536 + 1;
        writeln!(source, "#{id}=CARTESIAN_POINT('p',(1.,2.,3.));").unwrap();
    }
    source.push_str(FOOTER);
    equal(source.as_bytes(), 65_536);
}

#[test]
fn header_warnings_and_edition_three_metadata_survive() {
    let source = format!(
        "{HEADER}ANCHOR;\n<part>=#7;\nENDSEC;\n\
         REFERENCE;\n#99=<external.stp#item>;\nENDSEC;\n\
         DATA;\n#7=UNKNOWN(,,'bad\\Qescape');\n{FOOTER}"
    );
    let raw = parse(source.as_bytes()).unwrap();
    assert_ne!(raw.warnings, [] as [step_io::ParseWarning; 0]);
    assert_ne!(raw.anchors, [] as [(String, u64); 0]);
    assert!(!raw.external_references.is_empty());
    equal(source.as_bytes(), 1);
}

#[test]
fn latin_one_and_utf8_strings_reconstruct_the_same_unicode() {
    for text in [b"caf\xe9".as_slice(), "日本語é".as_bytes()] {
        let mut source = format!("{HEADER}DATA;\n#1=UNKNOWN('").into_bytes();
        source.extend_from_slice(text);
        source.extend_from_slice(format!("');\n{FOOTER}").as_bytes());
        equal(&source, 1);
    }
}

#[test]
fn duplicate_and_later_failures_keep_the_same_diagnostic() {
    struct Failing;
    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("input disconnected"))
        }
    }
    let body = "DATA;\n#90=A();\n#3=B();\n#90=C();\n#3=D();\n";
    for ending in [FOOTER, "ENDSEC;\n", "#5=A(\n"] {
        let source = format!("{HEADER}{body}{ending}");
        for chunk in [1, 7, 65_536] {
            let ordinary = parse(Chunks(source.as_bytes(), chunk)).unwrap_err();
            let compact = parse_compact(Chunks(source.as_bytes(), chunk)).unwrap_err();
            assert_eq!(compact.to_string(), ordinary.to_string());
        }
    }
    for body in [body, "DATA;\n#90=A();\n#3=B();\n"] {
        let source = format!("{HEADER}{body}");
        assert_eq!(
            parse_compact(Chunks(source.as_bytes(), 1).chain(Failing))
                .unwrap_err()
                .to_string(),
            parse(Chunks(source.as_bytes(), 1).chain(Failing))
                .unwrap_err()
                .to_string()
        );
    }
}

#[test]
fn errors_before_any_repeat_and_nesting_limit_are_unchanged() {
    for body in [
        "#1=A(;\n#1=A();\n".to_owned(),
        format!("#1=A({}{});\n", "(".repeat(1000), ")".repeat(1000)),
        "#1=A(1.E999);\n".to_owned(),
    ] {
        let source = format!("{HEADER}DATA;\n{body}{FOOTER}");
        assert_eq!(
            parse_compact(source.as_bytes()).unwrap_err().to_string(),
            parse(source.as_bytes()).unwrap_err().to_string()
        );
    }
}

#[test]
fn empty_data_section_and_normal_nested_lists_round_trip() {
    equal(format!("{HEADER}DATA;\n{FOOTER}").as_bytes(), 1);
    let source = format!(
        "{HEADER}DATA;\n#1=A({}#1{});\n{FOOTER}",
        "(".repeat(24),
        ")".repeat(24)
    );
    equal(source.as_bytes(), 7);
}
