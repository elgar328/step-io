use std::collections::BTreeMap;

use crate::parser::lexer::{LexError, Span};

/// A parsed attribute value from a Part 21 entity instance.
///
/// Covers every construct allowed in a parameter position by ISO 10303-21.
#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    Integer(i64),
    Real(f64),
    /// String with outer quotes stripped. The `''` escape is decoded to a
    /// single `'` by the lexer. Wide-char escapes (`\X\HH`, `\X2\...\X0\`)
    /// are **not** decoded — that step is deferred to a later stage.
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
pub struct Graph {
    /// Identified application protocol. The raw `FILE_SCHEMA` string list
    /// is preserved inside [`StepSchema`] itself — see its docs.
    pub schema: super::schema::StepSchema,
    /// Raw HEADER entities (`FILE_DESCRIPTION`, `FILE_NAME`, `FILE_SCHEMA`).
    pub header: Vec<RawEntity>,
    /// DATA section entities keyed by their `#N` identifier.
    /// `BTreeMap` gives deterministic iteration order (useful for the writer stage).
    pub entities: BTreeMap<u64, RawEntity>,
    /// P21 edition 3 `REFERENCE` section: `#N -> "<url#anchor>"`. Each entry
    /// is an entity id resolved externally rather than in the DATA section.
    /// `BTreeMap` keeps the deterministic id order the writer relies on.
    pub external_references: BTreeMap<u64, String>,
    /// P21 edition 3 `ANCHOR` section: `(<name>, #N)` pairs naming an entity
    /// (in this corpus, always an external reference). Order-preserving.
    pub anchors: Vec<(String, u64)>,
    /// Non-fatal issues observed while parsing. Lenient recoveries
    /// (missing semicolons, empty attribute slots) and P21 edition 3
    /// sections that step-io does not yet model land here so callers can
    /// surface them without aborting the parse.
    pub warnings: Vec<ParseWarning>,
}

/// Non-fatal issues observed during parsing.
///
/// The parser admits a handful of spec-bending inputs to survive
/// real-world STEP files; each tolerance pushes a [`ParseWarning`] so
/// downstream stages can surface what was repaired or discarded. The
/// IR itself never carries the non-standard form — input is normalised
/// to a spec-conformant shape before it reaches [`Graph`].
#[derive(Debug, Clone, PartialEq)]
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
    /// A P21 edition 3 section was encountered (`ANCHOR`, `REFERENCE`,
    /// or `SIGNATURE`) and discarded. step-io does not yet model these
    /// sections in the IR.
    Ed3SectionDiscarded {
        section: String,
        span: super::lexer::Span,
    },
}

impl Graph {
    /// Look up an entity by its `#N` identifier.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&RawEntity> {
        self.entities.get(&id)
    }
}

/// Parse error kinds specific to the Part 21 parser.
///
/// Lexer errors are wrapped in [`Error::Lex`]; the remaining variants
/// describe structural problems detected during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A lexical error bubbled up from the tokenizer.
    Lex(LexError),
    /// The token stream ended before the parser expected.
    UnexpectedEof { expected: &'static str },
    /// A token was found where a different kind was expected.
    UnexpectedToken {
        expected: &'static str,
        found: super::lexer::TokenKind,
        span: Span,
    },
    /// Two entities share the same `#N` identifier.
    DuplicateEntityId { id: u64, span: Span },
    /// A required HEADER entity is missing.
    MissingHeaderEntity { name: &'static str },
    /// `FILE_SCHEMA` does not contain a list of strings.
    MalformedFileSchema { span: Span },
    /// An attribute appeared in an invalid position (e.g. `$` inside a list).
    InvalidAttributePosition { span: Span, detail: &'static str },
    /// A parameter is nested deeper than [`MAX_NESTING_DEPTH`] — guards against a
    /// stack overflow on adversarially deep `(((…)))` / `A(B(C(…)))` input.
    ///
    /// [`MAX_NESTING_DEPTH`]: super::p21::MAX_NESTING_DEPTH
    NestingTooDeep { span: Span },
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
                "parse error at line {}, column {}: expected {expected}, found {found:?}",
                span.line, span.column,
            ),
            Self::DuplicateEntityId { id, span } => write!(
                f,
                "parse error at line {}, column {}: duplicate entity #{id}",
                span.line, span.column,
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
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lex(inner) => Some(inner),
            _ => None,
        }
    }
}

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
        assert_eq!(parse_err, Error::Lex(lex_err));
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
            schema: super::super::schema::StepSchema::default(),
            header: vec![],
            entities,
            external_references: BTreeMap::new(),
            anchors: vec![],
            warnings: vec![],
        };
        assert_eq!(graph.get(1), Some(&entity));
        assert_eq!(graph.get(999), None);
    }
}
