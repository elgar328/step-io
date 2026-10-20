use std::collections::BTreeMap;

use crate::parser::lexer::{LexError, Span};

/// A parsed attribute value from a Part 21 entity instance.
///
/// Covers every construct allowed in a parameter position by ISO 10303-21.
#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    Integer(i64),
    Real(f64),
    /// A string's value: quotes stripped and every escape decoded (`''`,
    /// `\\`, `\X\HH`, `\X2\…\X0\`, `\X4\…\X0\`, `\S\`), with the line
    /// breaks writers insert into long strings dropped.
    String(String),
    /// Enumeration value without the surrounding dots (e.g. `.T.` → `"T"`).
    Enum(String),
    /// Hex-encoded binary without the surrounding quotes.
    Binary(String),
    /// Entity reference `#N`.
    EntityRef(u64),
    /// Unset / omitted attribute (`$`).
    Unset,
    /// Derived attribute (`*`).
    Derived,
    /// Parenthesised list of parameters. May be nested.
    List(Vec<Attribute>),
    /// Typed parameter such as `LENGTH_MEASURE(1.E-07)`.
    Typed {
        type_name: String,
        value: Box<Attribute>,
    },
}

/// A single sub-entity inside a complex entity instance.
#[derive(Debug, Clone, PartialEq)]
pub struct RawEntityPart {
    /// Entity type name (always stored **upper-cased**).
    pub name: String,
    pub attributes: Vec<Attribute>,
}

/// A parsed Part 21 entity instance (one line in the DATA section).
#[derive(Debug, Clone, PartialEq)]
pub enum RawEntity {
    Simple {
        id: u64,
        /// Entity type name (always stored **upper-cased**).
        name: String,
        attributes: Vec<Attribute>,
        /// Position of the `#N` token that opened this instance.
        span: Span,
    },
    Complex {
        id: u64,
        parts: Vec<RawEntityPart>,
        /// Position of the `#N` token that opened this instance.
        span: Span,
    },
}

impl RawEntity {
    /// The numeric identifier (`#N`).
    #[must_use]
    pub fn id(&self) -> u64 {
        match self {
            Self::Simple { id, .. } | Self::Complex { id, .. } => *id,
        }
    }

    /// Source position of the `#N` token.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Simple { span, .. } | Self::Complex { span, .. } => *span,
        }
    }
}

/// The complete result of parsing a Part 21 file.
#[derive(Debug)]
#[non_exhaustive]
pub struct Graph {
    /// Identified application protocol (AP family, edition, stage), with the
    /// `FILE_SCHEMA` names as written.
    pub schema: crate::SchemaId,
    /// Raw HEADER entities (`FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`).
    pub header: Vec<RawEntity>,
    /// DATA section entities keyed by their `#N` identifier, in id order.
    pub entities: BTreeMap<u64, RawEntity>,
    /// P21 edition 3 `REFERENCE` section: `#N -> "<url#anchor>"`, in id
    /// order. Each entry is an entity id resolved outside the file rather
    /// than in its DATA section.
    pub external_references: BTreeMap<u64, String>,
    /// P21 edition 3 `ANCHOR` section: `(<name>, #N)` pairs naming an
    /// entity, in file order.
    pub anchors: Vec<(String, u64)>,
    /// What the parser repaired or skipped without stopping, such as a
    /// missing semicolon, a blank attribute slot or a SIGNATURE section.
    pub warnings: Vec<ParseWarning>,
}

/// Non-fatal issues observed during parsing.
///
/// The parser admits a handful of spec-bending inputs to survive
/// real-world STEP files; each tolerance records a [`ParseWarning`], so a
/// caller can show what was repaired or discarded. What the parser hands on
/// is always in the standard form.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseWarning {
    /// A HEADER entity is missing its terminating `;`. The next keyword
    /// is treated as the start of the next entity; the IR shows no trace.
    MissingHeaderSemicolon {
        entity_name: String,
        span: super::lexer::Span,
    },
    /// An attribute position is blank — `(a, , b)` or trailing `(a, )`.
    /// The slot is normalised to [`Attribute::Unset`] (the spec form `$`).
    EmptyAttribute { span: super::lexer::Span },
    /// A P21 edition 3 section (`ANCHOR`, `REFERENCE` or `SIGNATURE`) was
    /// discarded. [`read`](crate::read) discards all three, as the model has
    /// no place for them; [`parse`](crate::parser::parse) keeps `ANCHOR`
    /// and `REFERENCE` in its [`Graph`] and discards `SIGNATURE`, or a
    /// section whose lines it cannot read.
    Ed3SectionDiscarded {
        section: String,
        span: super::lexer::Span,
    },
    /// A string holds an escape that cannot be decoded — a lone `\`, an
    /// unterminated or broken `\X2\` / `\X4\` run, a `\P?\` code page other
    /// than `A`, or an unknown directive. It is kept as written, so the value
    /// holds the backslash as a character (and writes back as `\\`).
    MalformedStringEscape { span: super::lexer::Span },
    /// A string's hex escape uses lower-case digits (`\X2\c5d0\X0\`), which
    /// Part 21 does not allow. It is decoded all the same.
    LowercaseHexEscape { span: super::lexer::Span },
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (span, what) = match self {
            Self::MissingHeaderSemicolon { entity_name, span } => (
                span,
                format!("HEADER entity {entity_name} has no terminating ';'"),
            ),
            Self::EmptyAttribute { span } => (span, "blank attribute read as $".to_owned()),
            Self::Ed3SectionDiscarded { section, span } => {
                (span, format!("{section} section skipped (not supported)"))
            }
            Self::MalformedStringEscape { span } => (
                span,
                "string escape could not be decoded; kept as written".to_owned(),
            ),
            Self::LowercaseHexEscape { span } => {
                (span, "string escape uses lower-case hex digits".to_owned())
            }
        };
        write!(
            f,
            "warning at line {}, column {}: {what}",
            span.line, span.column
        )
    }
}

impl Graph {
    /// Look up an entity by its `#N` identifier.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&RawEntity> {
        self.entities.get(&id)
    }
}

/// Why [`read`](crate::read) or [`parse`](crate::parser::parse) could not
/// read a file.
///
/// Lexer errors are wrapped in [`Error::Lex`] and a failing input in
/// [`Error::Io`]; the remaining variants describe structural problems detected
/// during parsing. Each prints as a one-line message that includes the wrapped
/// error's, so [`source`](std::error::Error::source) is always `None`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A lexical error bubbled up from the tokenizer.
    Lex(LexError),
    /// The token stream ended before the parser expected.
    UnexpectedEof { expected: &'static str },
    /// A token was found where a different kind was expected.
    UnexpectedToken {
        expected: &'static str,
        /// The token found, about as written: `FOO`, `')'`, `#12`, `.T.`, a
        /// string in quotes (cut at 40 characters); a real number as its
        /// value.
        found: String,
        span: Span,
    },
    /// Two entities share the same `#N` identifier: the first repeat in the
    /// file, at `line`, of the entity at `first_line`.
    DuplicateEntityId { id: u64, line: u32, first_line: u32 },
    /// A required HEADER entity is missing.
    MissingHeaderEntity { name: &'static str },
    /// `FILE_SCHEMA` does not contain a list of strings.
    MalformedFileSchema { span: Span },
    /// An attribute appeared in an invalid position (e.g. `$` inside a list).
    InvalidAttributePosition { span: Span, detail: &'static str },
    /// A parameter is nested deeper than [`MAX_NESTING_DEPTH`] — guards against a
    /// stack overflow on adversarially deep `(((…)))` / `A(B(C(…)))` input.
    ///
    /// [`MAX_NESTING_DEPTH`]: crate::parser::MAX_NESTING_DEPTH
    NestingTooDeep { span: Span },
    /// The file holds more than the reader can index: at most `u32::MAX`
    /// entities, and as many reference and text slots.
    /// `span` is the entity where reading stopped.
    TooLarge { span: Span },
    /// Reading the input failed, with the input's own error.
    Io(std::io::Error),
}

impl From<LexError> for Error {
    fn from(err: LexError) -> Self {
        Self::Lex(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(inner) => inner.fmt(f),
            Self::UnexpectedEof { expected } => {
                write!(
                    f,
                    "parse error: unexpected end of file, expected {expected}"
                )
            }
            Self::UnexpectedToken {
                expected,
                found,
                span,
            } => write!(
                f,
                "parse error at line {}, column {}: expected {expected}, found {found}",
                span.line, span.column,
            ),
            Self::DuplicateEntityId {
                id,
                line,
                first_line,
            } => write!(
                f,
                "parse error at line {line}: duplicate entity #{id} (first at line {first_line})",
            ),
            Self::MissingHeaderEntity { name } => {
                write!(f, "parse error: missing required HEADER entity {name}")
            }
            Self::MalformedFileSchema { span } => write!(
                f,
                "parse error at line {}, column {}: malformed FILE_SCHEMA",
                span.line, span.column,
            ),
            Self::InvalidAttributePosition { span, detail } => write!(
                f,
                "parse error at line {}, column {}: {detail}",
                span.line, span.column,
            ),
            Self::NestingTooDeep { span } => write!(
                f,
                "parse error at line {}, column {}: parameter nesting too deep",
                span.line, span.column,
            ),
            Self::TooLarge { span } => write!(
                f,
                "read error at line {}, column {}: the file is too large to read",
                span.line, span.column,
            ),
            Self::Io(e) => write!(f, "read error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_entity_id_simple() {
        let e = RawEntity::Simple {
            id: 42,
            name: "TEST".into(),
            attributes: vec![],
            span: Span {
                start: 0,
                end: 3,
                line: 1,
                column: 1,
            },
        };
        assert_eq!(e.id(), 42);
    }

    #[test]
    fn raw_entity_id_complex() {
        let e = RawEntity::Complex {
            id: 99,
            parts: vec![],
            span: Span {
                start: 0,
                end: 3,
                line: 1,
                column: 1,
            },
        };
        assert_eq!(e.id(), 99);
    }

    #[test]
    fn raw_entity_span_accessor() {
        let span = Span {
            start: 10,
            end: 20,
            line: 3,
            column: 5,
        };
        let e = RawEntity::Simple {
            id: 1,
            name: "A".into(),
            attributes: vec![],
            span,
        };
        assert_eq!(e.span(), span);
    }

    #[test]
    fn parse_error_from_lex_error() {
        let lex_err = LexError {
            kind: crate::parser::lexer::LexErrorKind::UnexpectedCharacter,
            span: Span {
                start: 0,
                end: 1,
                line: 1,
                column: 1,
            },
            snippet: "@".into(),
        };
        let parse_err = Error::from(lex_err.clone());
        assert!(matches!(parse_err, Error::Lex(e) if e == lex_err));
    }

    #[test]
    fn parse_error_display_delegates_for_lex() {
        let lex_err = LexError {
            kind: crate::parser::lexer::LexErrorKind::UnexpectedCharacter,
            span: Span {
                start: 0,
                end: 1,
                line: 1,
                column: 1,
            },
            snippet: "@".into(),
        };
        let parse_err = Error::Lex(lex_err.clone());
        // Lex variant delegates to LexError's Display — no "parse error:" prefix.
        assert_eq!(parse_err.to_string(), lex_err.to_string());
    }

    #[test]
    fn parse_error_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let err = Error::UnexpectedEof {
            expected: "semicolon",
        };
        assert_error(&err);
    }

    #[test]
    fn entity_graph_get_returns_entity() {
        let span = Span {
            start: 0,
            end: 1,
            line: 1,
            column: 1,
        };
        let entity = RawEntity::Simple {
            id: 1,
            name: "X".into(),
            attributes: vec![],
            span,
        };
        let mut entities = BTreeMap::new();
        entities.insert(1, entity.clone());
        let graph = Graph {
            schema: super::super::schema::SchemaId::default(),
            header: vec![],
            entities,
            external_references: BTreeMap::new(),
            anchors: vec![],
            warnings: vec![],
        };
        assert_eq!(graph.get(1), Some(&entity));
        assert_eq!(graph.get(999), None);
    }

    #[test]
    fn parse_warnings_print_as_one_line() {
        let span = Span {
            start: 0,
            end: 1,
            line: 12,
            column: 5,
        };
        let cases = [
            (
                ParseWarning::MissingHeaderSemicolon {
                    entity_name: "FILE_NAME".into(),
                    span,
                },
                "warning at line 12, column 5: HEADER entity FILE_NAME has no terminating ';'",
            ),
            (
                ParseWarning::EmptyAttribute { span },
                "warning at line 12, column 5: blank attribute read as $",
            ),
            (
                ParseWarning::Ed3SectionDiscarded {
                    section: "ANCHOR".into(),
                    span,
                },
                "warning at line 12, column 5: ANCHOR section skipped (not supported)",
            ),
            (
                ParseWarning::MalformedStringEscape { span },
                "warning at line 12, column 5: string escape could not be decoded; kept as written",
            ),
            (
                ParseWarning::LowercaseHexEscape { span },
                "warning at line 12, column 5: string escape uses lower-case hex digits",
            ),
        ];
        for (warning, want) in cases {
            assert_eq!(warning.to_string(), want);
        }
    }
}
