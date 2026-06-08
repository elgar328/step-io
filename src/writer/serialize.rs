//! Render `WriterEntity` / `HeaderEntity` / `Attribute` values to Part 21 text.

use std::io::Write;

use super::WriteError;
use super::buffer::step_id_cache::StepIdCache;
use super::entity::{HeaderEntity, WriterBody, WriterEntity};
use super::lexical::{format_enum, format_real, format_ref, format_string};
use crate::ir::StepModel;
use crate::parser::entity::Attribute;

/// Pre-rendered P21 edition 3 `ANCHOR` / `REFERENCE` section body lines, built
/// from the model's external references and the ids reserved for them in the
/// writer pass. Empty when the model carries no edition-3 sections (the
/// overwhelmingly common case).
pub(super) struct Ed3Sections {
    /// `<name>=#N;` lines for the ANCHOR section.
    anchor_lines: Vec<String>,
    /// `#N=<url#anchor>;` lines for the REFERENCE section.
    reference_lines: Vec<String>,
}

impl Ed3Sections {
    pub(super) fn build(model: &StepModel, step_ids: &StepIdCache) -> Self {
        let reference_lines = model
            .external_references
            .iter_with_ids()
            .map(|(id, ext)| format!("{}={};", format_ref(step_ids.get(id)), ext.anchor))
            .collect();
        let anchor_lines = model
            .anchors
            .iter()
            .map(|a| format!("{}={};", a.name, format_ref(step_ids.get(a.target))))
            .collect();
        Self {
            anchor_lines,
            reference_lines,
        }
    }
}

/// Render a single attribute value. Lists recurse through this function.
pub(super) fn serialize_attr(attr: &Attribute) -> Result<String, WriteError> {
    match attr {
        Attribute::Integer(n) => Ok(n.to_string()),
        Attribute::Real(f) => format_real(*f),
        Attribute::String(s) => Ok(format_string(s)),
        Attribute::Enum(name) => Ok(format_enum(name)),
        Attribute::Binary(hex) => Ok(format!("\"{hex}\"")),
        Attribute::EntityRef(id) => Ok(format_ref(*id)),
        Attribute::Unset => Ok("$".into()),
        Attribute::Derived => Ok("*".into()),
        Attribute::List(items) => {
            let mut out = String::from("(");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serialize_attr(item)?);
            }
            out.push(')');
            Ok(out)
        }
        Attribute::Typed { type_name, value } => {
            Ok(format!("{type_name}({})", serialize_attr(value)?))
        }
    }
}

fn serialize_attr_list(attrs: &[Attribute]) -> Result<String, WriteError> {
    let mut out = String::from("(");
    for (i, attr) in attrs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_attr(attr)?);
    }
    out.push(')');
    Ok(out)
}

/// Render a DATA-section entity as `#N = NAME(attrs);` or `#N = ( ... );`.
pub(super) fn serialize_entity(entity: &WriterEntity) -> Result<String, WriteError> {
    match &entity.body {
        WriterBody::Simple { name, attrs } => Ok(format!(
            "#{} = {name}{};",
            entity.id,
            serialize_attr_list(attrs)?
        )),
        WriterBody::Complex { parts } => {
            let mut out = format!("#{} = (", entity.id);
            for (i, (name, attrs)) in parts.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str(name);
                out.push_str(&serialize_attr_list(attrs)?);
            }
            out.push_str(");");
            Ok(out)
        }
    }
}

/// Render a HEADER-section entity as `NAME(attrs);` — no `#N` prefix.
pub(super) fn serialize_header_entity(entity: &HeaderEntity) -> Result<String, WriteError> {
    Ok(format!(
        "{}{};",
        entity.name,
        serialize_attr_list(&entity.attrs)?
    ))
}

/// Stream the complete Part 21 file (HEADER + DATA sections) to `writer`.
///
/// Per-entity text is built as a short local `String` and then flushed with
/// a single `write_all`, which keeps peak memory bounded while preserving
/// the simple `serialize_entity` / `serialize_attr` interface for recursive
/// attribute rendering.
pub(super) fn write_file<W: Write>(
    writer: &mut W,
    headers: &[HeaderEntity],
    ed3: &Ed3Sections,
    entities: &[WriterEntity],
) -> Result<(), WriteError> {
    writer.write_all(b"ISO-10303-21;\n")?;
    writer.write_all(b"HEADER;\n")?;
    for h in headers {
        writer.write_all(serialize_header_entity(h)?.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.write_all(b"ENDSEC;\n")?;
    // P21 edition 3 ANCHOR / REFERENCE sections (only when present).
    if !ed3.anchor_lines.is_empty() {
        writer.write_all(b"ANCHOR;\n")?;
        for line in &ed3.anchor_lines {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.write_all(b"ENDSEC;\n")?;
    }
    if !ed3.reference_lines.is_empty() {
        writer.write_all(b"REFERENCE;\n")?;
        for line in &ed3.reference_lines {
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.write_all(b"ENDSEC;\n")?;
    }
    writer.write_all(b"DATA;\n")?;
    for e in entities {
        writer.write_all(serialize_entity(e)?.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    writer.write_all(b"ENDSEC;\n")?;
    writer.write_all(b"END-ISO-10303-21;\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_attr() {
        assert_eq!(serialize_attr(&Attribute::Integer(7)).unwrap(), "7");
        assert_eq!(serialize_attr(&Attribute::Integer(-42)).unwrap(), "-42");
    }

    #[test]
    fn real_attr_rounds_through_lexical() {
        assert_eq!(serialize_attr(&Attribute::Real(1.0)).unwrap(), "1.");
        assert_eq!(serialize_attr(&Attribute::Real(1e-7)).unwrap(), "1.E-07");
    }

    #[test]
    fn real_attr_surfaces_nan_error() {
        assert!(matches!(
            serialize_attr(&Attribute::Real(f64::NAN)),
            Err(WriteError::InvalidFloat { .. })
        ));
    }

    #[test]
    fn string_attr_escapes() {
        assert_eq!(
            serialize_attr(&Attribute::String("a'b".into())).unwrap(),
            "'a''b'"
        );
        assert_eq!(
            serialize_attr(&Attribute::String(String::new())).unwrap(),
            "''"
        );
    }

    #[test]
    fn enum_attr() {
        assert_eq!(
            serialize_attr(&Attribute::Enum("MILLI".into())).unwrap(),
            ".MILLI."
        );
    }

    #[test]
    fn entity_ref_attr() {
        assert_eq!(serialize_attr(&Attribute::EntityRef(7)).unwrap(), "#7");
    }

    #[test]
    fn unset_and_derived_attrs() {
        assert_eq!(serialize_attr(&Attribute::Unset).unwrap(), "$");
        assert_eq!(serialize_attr(&Attribute::Derived).unwrap(), "*");
    }

    #[test]
    fn list_attr_comma_separated() {
        let list = Attribute::List(vec![
            Attribute::Real(1.0),
            Attribute::Real(2.0),
            Attribute::Real(3.0),
        ]);
        assert_eq!(serialize_attr(&list).unwrap(), "(1.,2.,3.)");
    }

    #[test]
    fn nested_list_attr() {
        let list = Attribute::List(vec![
            Attribute::List(vec![Attribute::Integer(1), Attribute::Integer(2)]),
            Attribute::EntityRef(9),
        ]);
        assert_eq!(serialize_attr(&list).unwrap(), "((1,2),#9)");
    }

    #[test]
    fn typed_attr() {
        let typed = Attribute::Typed {
            type_name: "LENGTH_MEASURE".into(),
            value: Box::new(Attribute::Real(1e-7)),
        };
        assert_eq!(serialize_attr(&typed).unwrap(), "LENGTH_MEASURE(1.E-07)");
    }

    #[test]
    fn simple_entity_renders_with_hash_and_semicolon() {
        let e = WriterEntity {
            id: 5,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(1.0),
                        Attribute::Real(2.0),
                        Attribute::Real(3.0),
                    ]),
                ],
            },
        };
        assert_eq!(
            serialize_entity(&e).unwrap(),
            "#5 = CARTESIAN_POINT('',(1.,2.,3.));"
        );
    }

    #[test]
    fn complex_entity_renders_space_separated_parts() {
        let e = WriterEntity {
            id: 42,
            body: WriterBody::Complex {
                parts: vec![
                    ("LENGTH_UNIT".into(), vec![]),
                    ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                    (
                        "SI_UNIT".into(),
                        vec![
                            Attribute::Enum("MILLI".into()),
                            Attribute::Enum("METRE".into()),
                        ],
                    ),
                ],
            },
        };
        assert_eq!(
            serialize_entity(&e).unwrap(),
            "#42 = (LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));"
        );
    }

    #[test]
    fn header_entity_omits_hash_prefix() {
        let h = HeaderEntity {
            name: "FILE_DESCRIPTION".into(),
            attrs: vec![
                Attribute::List(vec![Attribute::String("hello".into())]),
                Attribute::String("2;1".into()),
            ],
        };
        assert_eq!(
            serialize_header_entity(&h).unwrap(),
            "FILE_DESCRIPTION(('hello'),'2;1');"
        );
    }

    #[test]
    fn empty_file_still_emits_wrapper() {
        let mut buf = Vec::new();
        let ed3 = Ed3Sections {
            anchor_lines: vec![],
            reference_lines: vec![],
        };
        write_file(&mut buf, &[], &ed3, &[]).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(
            text,
            "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n"
        );
    }

    #[test]
    fn file_renders_headers_then_entities() {
        let headers = vec![HeaderEntity {
            name: "FILE_SCHEMA".into(),
            attrs: vec![Attribute::List(vec![Attribute::String("X".into())])],
        }];
        let entities = vec![WriterEntity {
            id: 1,
            body: WriterBody::Simple {
                name: "POINT".into(),
                attrs: vec![Attribute::Real(0.0)],
            },
        }];
        let mut buf = Vec::new();
        let ed3 = Ed3Sections {
            anchor_lines: vec![],
            reference_lines: vec![],
        };
        write_file(&mut buf, &headers, &ed3, &entities).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("FILE_SCHEMA(('X'));\n"));
        assert!(text.contains("#1 = POINT(0.);\n"));
    }
}
