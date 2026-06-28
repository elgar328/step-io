use std::collections::BTreeMap;

use super::entity::{Attribute, Error, Graph, ParseWarning, RawEntity, RawEntityPart};
use super::lexer::{Lexer, Span, Token, TokenKind};
use super::schema::StepSchema;

/// Convenience function: parse a complete Part 21 source into an [`Graph`].
///
/// # Errors
///
/// Returns the first [`Error`] encountered.
pub fn parse(source: &str) -> Result<Graph, Error> {
    Parser::new(source).parse()
}

/// Parse a Part 21 source given as raw bytes. Tries UTF-8 first; on
/// failure falls back to ISO 8859-1 (Latin-1), which ISO 10303-21 §3.2
/// defines as the file format's default encoding. Real-world STEP files
/// frequently embed raw non-ASCII bytes (Cyrillic, Latin-1) directly
/// instead of using the spec's `\X\` / `\X2\` / `\X4\` escapes — those
/// bytes decode losslessly under the fallback (every byte 0x00..0xFF
/// maps 1:1 to U+0000..U+00FF).
///
/// # Errors
///
/// Returns the first [`Error`] encountered by the underlying parser.
pub fn parse_bytes(bytes: &[u8]) -> Result<Graph, Error> {
    if let Ok(s) = std::str::from_utf8(bytes) {
        parse(s)
    } else {
        let s: String = bytes.iter().map(|&b| b as char).collect();
        parse(&s)
    }
}

/// Recursive-descent parser for ISO 10303-21 (Part 21) files.
///
/// Consumes `self` on [`Parser::parse`] because the underlying [`Lexer`] is
/// a one-pass iterator and cannot be rewound.
/// Maximum parameter nesting depth. In practice STEP parameters nest only a few
/// levels (nested LIST OF LIST, typed-wrapped measures); this bound of 64 is a
/// large margin over any realistic file yet small enough that the
/// recursive-descent frames stay well within a thread's stack (a debug build
/// spans several frames per nesting level). It turns adversarially deep
/// `(((…)))` input into a graceful [`Error::NestingTooDeep`] instead of a
/// stack-overflow abort.
pub const MAX_NESTING_DEPTH: usize = 64;

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    warnings: Vec<ParseWarning>,
    /// Current parameter nesting depth (guards against stack overflow).
    depth: usize,
}

impl<'src> Parser<'src> {
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self {
            lexer: Lexer::new(source),
            warnings: Vec::new(),
            depth: 0,
        }
    }

    /// Parse the entire Part 21 file and return an [`Graph`].
    ///
    /// # Errors
    ///
    /// Returns a [`Error`] on the first structural or lexical problem.
    pub fn parse(self) -> Result<Graph, Error> {
        let mut this = self;
        this.parse_file()
    }

    // ------------------------------------------------------------------
    // Top-level grammar
    // ------------------------------------------------------------------

    fn parse_file(&mut self) -> Result<Graph, Error> {
        // ISO-10303-21;
        self.expect_token_kind(&TokenKind::IsoStart, "ISO-10303-21")?;
        self.expect_semicolon()?;

        // HEADER; ... ENDSEC;
        self.expect_token_kind(&TokenKind::Header, "HEADER")?;
        self.expect_semicolon()?;
        let (header, schema) = self.parse_header_section()?;
        self.expect_token_kind(&TokenKind::EndSec, "ENDSEC")?;
        self.expect_semicolon()?;

        // Optional P21 edition 3 sections (ANCHOR / REFERENCE / SIGNATURE).
        // ANCHOR / REFERENCE are parsed (so external references survive);
        // SIGNATURE and any unrecognised line shape fall back to a discard +
        // ParseWarning.
        let mut external_references: BTreeMap<u64, String> = BTreeMap::new();
        let mut anchors: Vec<(String, u64)> = Vec::new();
        while self.peek_is_ed3_section()? {
            self.parse_or_skip_ed3_section(&mut external_references, &mut anchors)?;
        }

        // DATA; ... ENDSEC;
        self.expect_token_kind(&TokenKind::Data, "DATA")?;
        self.expect_semicolon()?;
        let entities = self.parse_data_section()?;
        self.expect_token_kind(&TokenKind::EndSec, "ENDSEC")?;
        self.expect_semicolon()?;

        // END-ISO-10303-21;
        self.expect_token_kind(&TokenKind::IsoEnd, "END-ISO-10303-21")?;
        self.expect_semicolon()?;

        // EOF verification — nothing should follow.
        if let Some(result) = self.lexer.next() {
            let tok = result?;
            return Err(Error::UnexpectedToken {
                expected: "end of file",
                found: tok.kind,
                span: tok.span,
            });
        }

        Ok(Graph {
            schema,
            header,
            entities,
            external_references,
            anchors,
            warnings: std::mem::take(&mut self.warnings),
        })
    }

    // ------------------------------------------------------------------
    // P21 edition 3 sections (ANCHOR / REFERENCE / SIGNATURE)
    // ------------------------------------------------------------------

    fn peek_is_ed3_section(&mut self) -> Result<bool, Error> {
        Ok(matches!(
            self.peek_kind()?,
            TokenKind::Keyword(k)
                if matches!(k.to_uppercase().as_str(),
                            "ANCHOR" | "REFERENCE" | "SIGNATURE")
        ))
    }

    /// Parse an ANCHOR or REFERENCE section into `external_references` /
    /// `anchors`; SIGNATURE (and any line shape we don't recognise) falls back
    /// to a discard + `Ed3SectionDiscarded` warning so the parse never fails.
    fn parse_or_skip_ed3_section(
        &mut self,
        external_references: &mut BTreeMap<u64, String>,
        anchors: &mut Vec<(String, u64)>,
    ) -> Result<(), Error> {
        let tok = self.next_token()?;
        let section = match &tok.kind {
            TokenKind::Keyword(k) => k.to_uppercase(),
            _ => unreachable!("peek_is_ed3_section guarantees a Keyword"),
        };
        let span = tok.span;
        self.expect_semicolon()?;

        let mut discarded = false;
        while !matches!(self.peek_kind()?, TokenKind::EndSec) {
            // Each entry is one `lhs = rhs ;` line. REFERENCE lines are
            // `#N = <anchor>`; ANCHOR lines are `<name> = #N`. Anything else
            // (SIGNATURE bodies, unexpected shapes) is consumed and the whole
            // section is flagged as discarded.
            let lhs = self.next_token()?;
            if !matches!(self.peek_kind()?, TokenKind::Equals) {
                discarded = true;
                continue;
            }
            self.next_token()?; // consume `=`
            let rhs = self.next_token()?;
            match (section.as_str(), lhs.kind, rhs.kind) {
                ("REFERENCE", TokenKind::EntityRef(id), TokenKind::AnchorRef(s)) => {
                    external_references.insert(id, s);
                }
                ("ANCHOR", TokenKind::AnchorRef(name), TokenKind::EntityRef(id)) => {
                    anchors.push((name, id));
                }
                _ => discarded = true,
            }
            // Consume the line's terminating `;` (tolerate its absence).
            if matches!(self.peek_kind()?, TokenKind::Semicolon) {
                self.next_token()?;
            }
        }
        self.expect_token_kind(&TokenKind::EndSec, "ENDSEC")?;
        self.expect_semicolon()?;
        if discarded || section == "SIGNATURE" {
            self.warnings
                .push(ParseWarning::Ed3SectionDiscarded { section, span });
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // HEADER section
    // ------------------------------------------------------------------

    /// Parse the HEADER section content (between `HEADER;` and `ENDSEC;`).
    ///
    /// HEADER entities are "uninstantiated": they have no `#N =` prefix, just
    /// `KEYWORD(...);`. The section must contain at least `FILE_DESCRIPTION`,
    /// `FILE_NAME`, and `FILE_SCHEMA` in order.
    fn parse_header_section(&mut self) -> Result<(Vec<RawEntity>, StepSchema), Error> {
        let mut header = Vec::new();
        let mut schema_raw = Vec::new();

        // Read header entities until we see ENDSEC.
        // HEADER entities use an auto-incrementing pseudo-id (not in the file).
        let mut pseudo_id = 0u64;
        while !matches!(self.peek_kind()?, TokenKind::EndSec) {
            let tok = self.next_token()?;
            let name = match tok.kind {
                TokenKind::Keyword(name) => name.to_uppercase(),
                other => {
                    return Err(Error::UnexpectedToken {
                        expected: "HEADER entity name",
                        found: other,
                        span: tok.span,
                    });
                }
            };

            let attributes = self.parse_parameter_list()?;

            // Extract FILE_SCHEMA strings before consuming the semicolon.
            if name == "FILE_SCHEMA" {
                schema_raw = Self::extract_file_schema_strings(&attributes, tok.span)?;
            }

            // Some non-spec writers omit the trailing `;` after a HEADER
            // entity. If the next token already starts the following
            // entity (keyword) or closes the section (ENDSEC), accept
            // the missing semicolon with a ParseWarning.
            match self.peek_kind()? {
                TokenKind::Semicolon => {
                    self.next_token()?;
                }
                TokenKind::Keyword(_) | TokenKind::EndSec => {
                    self.warnings.push(ParseWarning::MissingHeaderSemicolon {
                        entity_name: name.clone(),
                        span: tok.span,
                    });
                }
                _ => {
                    self.expect_semicolon()?;
                }
            }

            pseudo_id += 1;
            header.push(RawEntity::Simple {
                id: pseudo_id,
                name,
                attributes,
                span: tok.span,
            });
        }

        // Validate required entities.
        let names: Vec<&str> = header
            .iter()
            .filter_map(|e| match e {
                RawEntity::Simple { name, .. } => Some(name.as_str()),
                RawEntity::Complex { .. } => None,
            })
            .collect();

        if !names.contains(&"FILE_DESCRIPTION") {
            return Err(Error::MissingHeaderEntity {
                name: "FILE_DESCRIPTION",
            });
        }
        if !names.contains(&"FILE_NAME") {
            return Err(Error::MissingHeaderEntity { name: "FILE_NAME" });
        }
        if !names.contains(&"FILE_SCHEMA") {
            return Err(Error::MissingHeaderEntity {
                name: "FILE_SCHEMA",
            });
        }

        let schema = super::schema::identify_schema(&schema_raw);

        Ok((header, schema))
    }

    /// Extract all string values from the first (and typically only) list
    /// attribute of a `FILE_SCHEMA((...))` entity.
    fn extract_file_schema_strings(
        attributes: &[Attribute],
        span: Span,
    ) -> Result<Vec<String>, Error> {
        // FILE_SCHEMA has exactly one attribute: a list of strings.
        let list = attributes
            .first()
            .ok_or(Error::MalformedFileSchema { span })?;
        match list {
            Attribute::List(items) => {
                let mut result = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Attribute::String(s) => result.push(s.clone()),
                        _ => return Err(Error::MalformedFileSchema { span }),
                    }
                }
                Ok(result)
            }
            _ => Err(Error::MalformedFileSchema { span }),
        }
    }

    // ------------------------------------------------------------------
    // DATA section
    // ------------------------------------------------------------------

    /// Parse the DATA section content (between `DATA;` and `ENDSEC;`).
    fn parse_data_section(&mut self) -> Result<BTreeMap<u64, RawEntity>, Error> {
        let mut entities = BTreeMap::new();

        while !matches!(self.peek_kind()?, TokenKind::EndSec) {
            let entity = self.parse_entity_instance()?;
            let id = entity.id();
            let span = entity.span();
            if entities.insert(id, entity).is_some() {
                return Err(Error::DuplicateEntityId { id, span });
            }
        }

        Ok(entities)
    }

    // ------------------------------------------------------------------
    // Entity instance parsing
    // ------------------------------------------------------------------

    /// Parse one entity instance: `#N = NAME(...);` or `#N = ( NAME(...) ... );`
    fn parse_entity_instance(&mut self) -> Result<RawEntity, Error> {
        // #N
        let id_tok = self.expect(
            |k| matches!(k, TokenKind::EntityRef(_)),
            "entity reference #N",
        )?;
        let TokenKind::EntityRef(id) = id_tok.kind else {
            unreachable!()
        };
        let span = id_tok.span;

        // =
        self.expect_equals()?;

        // Peek to distinguish Simple vs Complex.
        let kind = self.peek_kind()?.clone();
        match kind {
            TokenKind::Keyword(name) => {
                self.next_token()?; // consume keyword
                self.parse_simple_body(id, &name, span)
            }
            TokenKind::LParen => {
                self.next_token()?; // consume `(`
                self.parse_complex_body(id, span)
            }
            _ => {
                let tok = self.next_token()?;
                Err(Error::UnexpectedToken {
                    expected: "entity type name or '('",
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn parse_simple_body(&mut self, id: u64, name: &str, span: Span) -> Result<RawEntity, Error> {
        let attributes = self.parse_parameter_list()?;
        self.expect_semicolon()?;
        Ok(RawEntity::Simple {
            id,
            name: name.to_uppercase(),
            attributes,
            span,
        })
    }

    fn parse_complex_body(&mut self, id: u64, span: Span) -> Result<RawEntity, Error> {
        let mut parts = Vec::new();

        // At least one part is required — `( )` is an error.
        if matches!(self.peek_kind()?, TokenKind::RParen) {
            let tok = self.next_token()?;
            return Err(Error::UnexpectedToken {
                expected: "entity type name",
                found: tok.kind,
                span: tok.span,
            });
        }

        loop {
            let kind = self.peek_kind()?.clone();
            match kind {
                TokenKind::Keyword(name) => {
                    self.next_token()?; // consume keyword
                    let attributes = self.parse_parameter_list()?;
                    parts.push(RawEntityPart {
                        name: name.to_uppercase(),
                        attributes,
                    });
                }
                TokenKind::RParen => {
                    self.next_token()?; // consume `)`
                    break;
                }
                _ => {
                    let tok = self.next_token()?;
                    return Err(Error::UnexpectedToken {
                        expected: "entity type name or ')'",
                        found: tok.kind,
                        span: tok.span,
                    });
                }
            }
        }

        self.expect_semicolon()?;
        Ok(RawEntity::Complex { id, parts, span })
    }

    // ------------------------------------------------------------------
    // Attribute parsing
    // ------------------------------------------------------------------

    /// Parse a comma-separated parameter list between parentheses.
    /// Expects the opening `(` to have been consumed already? No — this
    /// method consumes `(`, reads parameters, and consumes `)`.
    fn parse_parameter_list(&mut self) -> Result<Vec<Attribute>, Error> {
        self.expect_lparen()?;
        let attrs = self.parse_list_value()?;
        self.expect_rparen()?;
        Ok(attrs)
    }

    /// Parse comma-separated parameters until a `)` is encountered.
    /// The `)` is **not** consumed — the caller is responsible.
    fn parse_list_value(&mut self) -> Result<Vec<Attribute>, Error> {
        let mut items = Vec::new();
        // Empty list `()`.
        if matches!(self.peek_kind()?, TokenKind::RParen) {
            return Ok(items);
        }
        items.push(self.parse_parameter_or_unset_if_empty()?);
        while matches!(self.peek_kind()?, TokenKind::Comma) {
            // Consume the comma.
            self.next_token()?;
            items.push(self.parse_parameter_or_unset_if_empty()?);
        }
        Ok(items)
    }

    /// Like [`Self::parse_parameter`] but tolerates a blank attribute
    /// position — `(a, , b)` or trailing `(a, )`. Spec requires `$` for
    /// omitted slots, but some writers leave them empty. The slot is
    /// normalised to [`Attribute::Unset`] and a [`ParseWarning`] is
    /// recorded so the lenient repair surfaces to the caller.
    fn parse_parameter_or_unset_if_empty(&mut self) -> Result<Attribute, Error> {
        if matches!(self.peek_kind()?, TokenKind::Comma | TokenKind::RParen) {
            let span = self.peek_span().unwrap_or(Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            });
            self.warnings.push(ParseWarning::EmptyAttribute { span });
            return Ok(Attribute::Unset);
        }
        self.parse_parameter()
    }

    /// Peek at the next token's span without consuming it. Returns
    /// `None` on EOF or a lex error (the warning path then falls back
    /// to a zero span — positional precision is non-critical).
    fn peek_span(&mut self) -> Option<Span> {
        match self.lexer.peek() {
            Some(Ok(tok)) => Some(tok.span),
            _ => None,
        }
    }

    /// Parse a single Part 21 parameter value.
    /// Parse one parameter, guarding nesting depth. The recursive cases (nested
    /// list, typed value) re-enter through this wrapper, so `depth` tracks the
    /// current nesting; exceeding [`MAX_NESTING_DEPTH`] is a graceful error rather
    /// than a stack overflow. Balanced inc/dec → siblings don't accumulate.
    fn parse_parameter(&mut self) -> Result<Attribute, Error> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            let span = self.peek_span().unwrap_or(Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            });
            return Err(Error::NestingTooDeep { span });
        }
        let r = self.parse_parameter_inner();
        self.depth -= 1;
        r
    }

    fn parse_parameter_inner(&mut self) -> Result<Attribute, Error> {
        let kind = self.peek_kind()?.clone();
        match kind {
            TokenKind::Integer(v) => {
                self.next_token()?;
                Ok(Attribute::Integer(v))
            }
            TokenKind::Real(v) => {
                self.next_token()?;
                Ok(Attribute::Real(v))
            }
            TokenKind::String(ref s) => {
                let s = s.clone();
                self.next_token()?;
                Ok(Attribute::String(s))
            }
            TokenKind::Enum(ref s) => {
                let s = s.clone();
                self.next_token()?;
                Ok(Attribute::Enum(s))
            }
            TokenKind::Binary(ref s) => {
                let s = s.clone();
                self.next_token()?;
                Ok(Attribute::Binary(s))
            }
            TokenKind::EntityRef(id) => {
                self.next_token()?;
                Ok(Attribute::EntityRef(id))
            }
            TokenKind::Dollar => {
                self.next_token()?;
                Ok(Attribute::Unset)
            }
            TokenKind::Asterisk => {
                self.next_token()?;
                Ok(Attribute::Derived)
            }
            TokenKind::LParen => {
                // Nested list.
                self.next_token()?; // consume `(`
                let items = self.parse_list_value()?;
                self.expect_rparen()?;
                Ok(Attribute::List(items))
            }
            TokenKind::Keyword(ref name) => {
                // Typed parameter: `NAME(value)`.
                let type_name = name.to_uppercase();
                self.next_token()?; // consume keyword
                self.expect_lparen()?;
                let value = self.parse_parameter()?;
                self.expect_rparen()?;
                Ok(Attribute::Typed {
                    type_name,
                    value: Box::new(value),
                })
            }
            _ => {
                let tok = self.next_token()?;
                Err(Error::InvalidAttributePosition {
                    span: tok.span,
                    detail: "unexpected token in attribute position",
                })
            }
        }
    }

    // ------------------------------------------------------------------
    // Token-level helpers
    // ------------------------------------------------------------------

    /// Consume the next token, returning [`Error::UnexpectedEof`] if the
    /// stream is exhausted.
    fn next_token(&mut self) -> Result<Token, Error> {
        match self.lexer.next() {
            Some(result) => Ok(result?),
            None => Err(Error::UnexpectedEof {
                expected: "any token",
            }),
        }
    }

    /// Peek at the next token's kind without consuming it.
    fn peek_kind(&mut self) -> Result<&TokenKind, Error> {
        match self.lexer.peek() {
            Some(Ok(tok)) => Ok(&tok.kind),
            Some(Err(err)) => Err(Error::Lex(err.clone())),
            None => Err(Error::UnexpectedEof {
                expected: "any token",
            }),
        }
    }

    /// Consume the next token if it matches `pred`; otherwise return an error.
    fn expect<F>(&mut self, pred: F, expected: &'static str) -> Result<Token, Error>
    where
        F: Fn(&TokenKind) -> bool,
    {
        let tok = self.next_token()?;
        if pred(&tok.kind) {
            Ok(tok)
        } else {
            Err(Error::UnexpectedToken {
                expected,
                found: tok.kind,
                span: tok.span,
            })
        }
    }

    /// Consume the next token and verify it matches a specific [`TokenKind`]
    /// variant (by discriminant, ignoring inner data).
    fn expect_token_kind(
        &mut self,
        kind: &TokenKind,
        expected: &'static str,
    ) -> Result<Token, Error> {
        self.expect(
            |k| std::mem::discriminant(k) == std::mem::discriminant(kind),
            expected,
        )
    }

    /// Expect and consume a semicolon.
    fn expect_semicolon(&mut self) -> Result<Span, Error> {
        Ok(self
            .expect(|k| matches!(k, TokenKind::Semicolon), ";")?
            .span)
    }

    /// Expect and consume a left parenthesis.
    fn expect_lparen(&mut self) -> Result<Span, Error> {
        Ok(self.expect(|k| matches!(k, TokenKind::LParen), "(")?.span)
    }

    /// Expect and consume a right parenthesis.
    fn expect_rparen(&mut self) -> Result<Span, Error> {
        Ok(self.expect(|k| matches!(k, TokenKind::RParen), ")")?.span)
    }

    /// Expect and consume an equals sign.
    fn expect_equals(&mut self) -> Result<Span, Error> {
        Ok(self.expect(|k| matches!(k, TokenKind::Equals), "=")?.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_input_errors() {
        let err = parse("").unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { .. }));
    }

    #[test]
    fn parser_next_token_returns_eof_on_empty() {
        let mut parser = Parser::new("");
        assert!(matches!(
            parser.next_token(),
            Err(Error::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn parser_peek_kind_returns_eof_on_empty() {
        let mut parser = Parser::new("");
        assert!(matches!(
            parser.peek_kind(),
            Err(Error::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn parser_expect_semicolon_ok() {
        let mut parser = Parser::new(";");
        assert!(parser.expect_semicolon().is_ok());
    }

    #[test]
    fn parser_expect_semicolon_wrong_token() {
        let mut parser = Parser::new("(");
        let err = parser.expect_semicolon().unwrap_err();
        assert!(matches!(err, Error::UnexpectedToken { expected: ";", .. }));
    }

    // --- parse_parameter helpers ---

    /// Parse a single parameter by feeding it directly to `parse_parameter`.
    fn parse_single_attr(param_text: &str) -> Attribute {
        let mut parser = Parser::new(param_text);
        parser
            .parse_parameter()
            .unwrap_or_else(|e| panic!("parse_parameter failed on {param_text:?}: {e}"))
    }

    // --- Scalar tests ---

    #[test]
    fn parse_param_integer() {
        assert_eq!(parse_single_attr("42"), Attribute::Integer(42));
    }

    #[test]
    fn parse_param_real() {
        assert_eq!(parse_single_attr("1.5"), Attribute::Real(1.5));
    }

    #[test]
    fn parse_param_real_exponent() {
        assert_eq!(parse_single_attr("1.E-07"), Attribute::Real(1e-7));
    }

    #[test]
    fn parse_param_string() {
        assert_eq!(
            parse_single_attr("'hello'"),
            Attribute::String("hello".into())
        );
    }

    #[test]
    fn parse_param_enum() {
        assert_eq!(
            parse_single_attr(".MILLI."),
            Attribute::Enum("MILLI".into())
        );
    }

    #[test]
    fn parse_param_binary() {
        assert_eq!(
            parse_single_attr("\"3FFA\""),
            Attribute::Binary("3FFA".into())
        );
    }

    #[test]
    fn parse_param_entity_ref() {
        assert_eq!(parse_single_attr("#42"), Attribute::EntityRef(42));
    }

    #[test]
    fn parse_param_unset() {
        assert_eq!(parse_single_attr("$"), Attribute::Unset);
    }

    #[test]
    fn parse_param_derived() {
        assert_eq!(parse_single_attr("*"), Attribute::Derived);
    }

    // --- List tests ---

    #[test]
    fn parse_param_empty_list() {
        assert_eq!(parse_single_attr("()"), Attribute::List(vec![]));
    }

    #[test]
    fn parse_param_flat_list() {
        assert_eq!(
            parse_single_attr("(1,2,3)"),
            Attribute::List(vec![
                Attribute::Integer(1),
                Attribute::Integer(2),
                Attribute::Integer(3),
            ])
        );
    }

    #[test]
    fn parse_param_nested_list() {
        assert_eq!(
            parse_single_attr("((#1),(#2,#3))"),
            Attribute::List(vec![
                Attribute::List(vec![Attribute::EntityRef(1)]),
                Attribute::List(vec![Attribute::EntityRef(2), Attribute::EntityRef(3)]),
            ])
        );
    }

    #[test]
    fn parse_param_enum_list() {
        assert_eq!(
            parse_single_attr("(.T.,.F.)"),
            Attribute::List(vec![
                Attribute::Enum("T".into()),
                Attribute::Enum("F".into()),
            ])
        );
    }

    // --- Typed literal tests ---

    #[test]
    fn parse_param_typed_real() {
        assert_eq!(
            parse_single_attr("LENGTH_MEASURE(1.E-07)"),
            Attribute::Typed {
                type_name: "LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(1e-7)),
            }
        );
    }

    #[test]
    fn parse_param_typed_positive_length() {
        assert_eq!(
            parse_single_attr("POSITIVE_LENGTH_MEASURE(0.1)"),
            Attribute::Typed {
                type_name: "POSITIVE_LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(0.1)),
            }
        );
    }

    #[test]
    fn parse_param_typed_of_list() {
        assert_eq!(
            parse_single_attr("BOUNDED_CURVE((#1,#2))"),
            Attribute::Typed {
                type_name: "BOUNDED_CURVE".into(),
                value: Box::new(Attribute::List(vec![
                    Attribute::EntityRef(1),
                    Attribute::EntityRef(2),
                ])),
            }
        );
    }

    #[test]
    fn parse_param_typed_nested() {
        // TYPE1(TYPE2(42)) — grammar allows recursive typed parameters.
        assert_eq!(
            parse_single_attr("TYPE1(TYPE2(42))"),
            Attribute::Typed {
                type_name: "TYPE1".into(),
                value: Box::new(Attribute::Typed {
                    type_name: "TYPE2".into(),
                    value: Box::new(Attribute::Integer(42)),
                }),
            }
        );
    }

    // --- Entity instance parsing ---

    fn parse_entity(src: &str) -> RawEntity {
        let mut parser = Parser::new(src);
        parser
            .parse_entity_instance()
            .unwrap_or_else(|e| panic!("parse_entity_instance failed: {e}"))
    }

    #[test]
    fn parse_simple_entity() {
        let e = parse_entity("#1 = LINE('', #2, #3);");
        match e {
            RawEntity::Simple {
                id,
                name,
                attributes,
                ..
            } => {
                assert_eq!(id, 1);
                assert_eq!(name, "LINE");
                assert_eq!(attributes.len(), 3);
                assert_eq!(attributes[0], Attribute::String(String::new()));
                assert_eq!(attributes[1], Attribute::EntityRef(2));
                assert_eq!(attributes[2], Attribute::EntityRef(3));
            }
            RawEntity::Complex { .. } => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_simple_entity_with_derived_and_unset() {
        let e = parse_entity("#20 = ORIENTED_EDGE('',*,*,#21,.T.);");
        match e {
            RawEntity::Simple { id, attributes, .. } => {
                assert_eq!(id, 20);
                assert_eq!(attributes[0], Attribute::String(String::new()));
                assert_eq!(attributes[1], Attribute::Derived);
                assert_eq!(attributes[2], Attribute::Derived);
                assert_eq!(attributes[3], Attribute::EntityRef(21));
                assert_eq!(attributes[4], Attribute::Enum("T".into()));
            }
            RawEntity::Complex { .. } => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_simple_entity_name_is_uppercased() {
        let e = parse_entity("#1 = cartesian_point('', (0., 0., 0.));");
        match e {
            RawEntity::Simple { name, .. } => {
                assert_eq!(name, "CARTESIAN_POINT");
            }
            RawEntity::Complex { .. } => panic!("expected Simple"),
        }
    }

    #[test]
    fn parse_complex_entity_four_parts() {
        let src = "#165 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3) \
                   GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#169)) \
                   GLOBAL_UNIT_ASSIGNED_CONTEXT((#166,#167,#168)) \
                   REPRESENTATION_CONTEXT('a','b') );";
        let e = parse_entity(src);
        match e {
            RawEntity::Complex { id, parts, .. } => {
                assert_eq!(id, 165);
                assert_eq!(parts.len(), 4);
                assert_eq!(parts[0].name, "GEOMETRIC_REPRESENTATION_CONTEXT");
                assert_eq!(parts[1].name, "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT");
                assert_eq!(parts[2].name, "GLOBAL_UNIT_ASSIGNED_CONTEXT");
                assert_eq!(parts[3].name, "REPRESENTATION_CONTEXT");
                assert_eq!(parts[0].attributes, vec![Attribute::Integer(3)]);
                assert_eq!(parts[3].attributes.len(), 2);
            }
            RawEntity::Simple { .. } => panic!("expected Complex"),
        }
    }

    #[test]
    fn parse_complex_entity_three_parts_mixed_attrs() {
        let src = "#166 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );";
        let e = parse_entity(src);
        match e {
            RawEntity::Complex { id, parts, .. } => {
                assert_eq!(id, 166);
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0].name, "LENGTH_UNIT");
                assert_eq!(parts[0].attributes, vec![]);
                assert_eq!(parts[1].name, "NAMED_UNIT");
                assert_eq!(parts[1].attributes, vec![Attribute::Derived]);
                assert_eq!(parts[2].name, "SI_UNIT");
                assert_eq!(parts[2].attributes.len(), 2);
            }
            RawEntity::Simple { .. } => panic!("expected Complex"),
        }
    }

    #[test]
    fn parse_complex_entity_name_uppercased() {
        let src = "#1 = ( length_unit() named_unit(*) );";
        let e = parse_entity(src);
        match e {
            RawEntity::Complex { parts, .. } => {
                assert_eq!(parts[0].name, "LENGTH_UNIT");
                assert_eq!(parts[1].name, "NAMED_UNIT");
            }
            RawEntity::Simple { .. } => panic!("expected Complex"),
        }
    }

    #[test]
    fn parse_empty_complex_entity_errors() {
        let mut parser = Parser::new("#1 = ( );");
        let err = parser.parse_entity_instance().unwrap_err();
        assert!(matches!(
            err,
            Error::UnexpectedToken {
                expected: "entity type name",
                ..
            }
        ));
    }

    #[test]
    fn parse_entity_missing_semicolon_errors() {
        let mut parser = Parser::new("#1 = LINE('', #2)");
        let err = parser.parse_entity_instance().unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { .. }));
    }

    // --- parse_bytes: non-UTF-8 input ---

    fn minimal_step_with_string_attr(s_bytes: &[u8]) -> Vec<u8> {
        let prefix = b"ISO-10303-21;\n\
                      HEADER;\n\
                      FILE_DESCRIPTION((' ',";
        let mid = b"),'2;1');\n\
                    FILE_NAME('n','t',(' '),(' '),'p','o','a');\n\
                    FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
                    ENDSEC;\n\
                    DATA;\n\
                    ENDSEC;\n\
                    END-ISO-10303-21;\n";
        let mut buf = Vec::with_capacity(prefix.len() + s_bytes.len() + mid.len() + 2);
        buf.extend_from_slice(prefix);
        buf.push(b'\'');
        buf.extend_from_slice(s_bytes);
        buf.push(b'\'');
        buf.extend_from_slice(mid);
        buf
    }

    #[test]
    fn parse_bytes_accepts_latin1_byte_via_fallback() {
        // 0xC0 alone is not valid UTF-8 (it is a leading byte for a 2-byte
        // sequence). Latin-1 maps it to U+00C0 ('À').
        let buf = minimal_step_with_string_attr(&[0xC0]);
        assert!(
            std::str::from_utf8(&buf).is_err(),
            "fixture must be invalid UTF-8 to exercise the fallback"
        );
        let graph = parse_bytes(&buf).expect("parse_bytes must accept Latin-1 fallback");
        // FILE_DESCRIPTION is the first header entity; its first attribute
        // is a list of strings. The second string of that list carries our
        // injected byte.
        let RawEntity::Simple {
            name, attributes, ..
        } = &graph.header[0]
        else {
            panic!("expected Simple HEADER entity");
        };
        assert_eq!(name, "FILE_DESCRIPTION");
        let Attribute::List(items) = &attributes[0] else {
            panic!("FILE_DESCRIPTION attr[0] must be a list");
        };
        let Attribute::String(s) = &items[1] else {
            panic!("expected String attribute");
        };
        assert_eq!(s.chars().count(), 1);
        assert_eq!(s.chars().next().unwrap(), '\u{00C0}');
    }

    #[test]
    fn parse_bytes_passes_through_valid_utf8() {
        // Valid UTF-8 'À' (0xC3 0x80) — the UTF-8 path is taken and the
        // character decodes identically.
        let buf = minimal_step_with_string_attr("À".as_bytes());
        assert!(std::str::from_utf8(&buf).is_ok());
        let graph = parse_bytes(&buf).expect("valid UTF-8 must parse");
        let RawEntity::Simple { attributes, .. } = &graph.header[0] else {
            panic!();
        };
        let Attribute::List(items) = &attributes[0] else {
            panic!();
        };
        let Attribute::String(s) = &items[1] else {
            panic!();
        };
        assert_eq!(s, "\u{00C0}");
    }

    // --- ParseWarning: lenient acceptance of non-spec / ed.3 inputs ---

    #[test]
    fn parse_records_ed3_anchor_and_reference_sections() {
        let src = "ISO-10303-21;\n\
                   HEADER;\n\
                   FILE_DESCRIPTION((''),'2;1');\n\
                   FILE_NAME('n','t',(''),(''),'p','o','a');\n\
                   FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
                   ENDSEC;\n\
                   ANCHOR;\n\
                   <ParentAnchor>=#123;\n\
                   ENDSEC;\n\
                   REFERENCE;\n\
                   #123=<testAnchorAndData.stp#TestAnchor>;\n\
                   ENDSEC;\n\
                   DATA;\n\
                   #1=CIRCULAR_AREA('testarea',#123,2.);\n\
                   ENDSEC;\n\
                   END-ISO-10303-21;\n";
        let graph = parse(src).expect("ed.3 ANCHOR/REFERENCE must be tolerated");
        assert_eq!(graph.entities.len(), 1);
        // REFERENCE recorded: #123 -> external anchor string.
        assert_eq!(
            graph.external_references.get(&123).map(String::as_str),
            Some("<testAnchorAndData.stp#TestAnchor>")
        );
        // ANCHOR recorded: <ParentAnchor> -> #123.
        assert_eq!(graph.anchors, vec![("<ParentAnchor>".to_string(), 123)]);
        // Both sections parsed cleanly — no discard warning.
        assert!(
            !graph
                .warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::Ed3SectionDiscarded { .. }))
        );
    }

    #[test]
    fn parse_discards_unrecognised_ed3_signature_section() {
        let src = "ISO-10303-21;\n\
                   HEADER;\n\
                   FILE_DESCRIPTION((''),'2;1');\n\
                   FILE_NAME('n','t',(''),(''),'p','o','a');\n\
                   FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
                   ENDSEC;\n\
                   SIGNATURE;\n\
                   some opaque signature payload;\n\
                   ENDSEC;\n\
                   DATA;\n\
                   #1 = CARTESIAN_POINT('',(0.,0.,0.));\n\
                   ENDSEC;\n\
                   END-ISO-10303-21;\n";
        let graph = parse(src).expect("ed.3 SIGNATURE must be tolerated");
        assert_eq!(graph.entities.len(), 1);
        assert!(
            graph
                .warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::Ed3SectionDiscarded { section, .. } if section == "SIGNATURE"))
        );
    }

    #[test]
    fn parse_warns_on_missing_header_semicolon() {
        // FILE_DESCRIPTION lacks its trailing `;` — non-spec but common.
        let src = "ISO-10303-21;\n\
                   HEADER;\n\
                   FILE_DESCRIPTION((''),'2;1')\n\
                   FILE_NAME('n','t',(''),(''),'p','o','a');\n\
                   FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
                   ENDSEC;\n\
                   DATA;\n\
                   ENDSEC;\n\
                   END-ISO-10303-21;\n";
        let graph = parse(src).expect("missing `;` must be tolerated");
        assert!(graph.warnings.iter().any(|w| matches!(
            w,
            ParseWarning::MissingHeaderSemicolon { entity_name, .. }
                if entity_name == "FILE_DESCRIPTION"
        )));
    }

    #[test]
    fn parse_normalises_empty_attribute_to_unset_with_warning() {
        // Trailing blank attribute: `#0,   )` — spec wants `$`.
        let src = "ISO-10303-21;\n\
                   HEADER;\n\
                   FILE_DESCRIPTION((''),'2;1');\n\
                   FILE_NAME('n','t',(''),(''),'p','o','a');\n\
                   FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
                   ENDSEC;\n\
                   DATA;\n\
                   #1 = EDGE_CURVE('', #2, #3, #0,   );\n\
                   #2 = VERTEX_POINT('', #4);\n\
                   #3 = VERTEX_POINT('', #4);\n\
                   #4 = CARTESIAN_POINT('',(0.,0.,0.));\n\
                   ENDSEC;\n\
                   END-ISO-10303-21;\n";
        let graph = parse(src).expect("empty attribute slot must be tolerated");
        // The fifth attribute of #1 is the omitted slot, normalised to Unset.
        let RawEntity::Simple { attributes, .. } = &graph.entities[&1] else {
            panic!("expected Simple entity");
        };
        assert_eq!(attributes.len(), 5);
        assert!(matches!(attributes[4], Attribute::Unset));
        assert!(
            graph
                .warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::EmptyAttribute { .. }))
        );
    }
}
