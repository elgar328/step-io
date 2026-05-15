use logos::Logos;

/// Position information for a lexed token.
///
/// `start`/`end` are byte offsets into the source. `line` is 1-indexed by
/// `\n` occurrences. `column` is 1-indexed and counts Unicode scalar values
/// (chars), so multibyte characters advance the column by one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    /// Return the slice of `source` that produced this token.
    #[must_use]
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

/// A lexed token carrying its kind and position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Lexical error categories.
///
/// `UnexpectedCharacter` is currently the only variant actually produced by
/// the lexer. The remaining variants are declared for forward compatibility
/// so that a later stage can wire a custom token-level error type through
/// `logos`'s `error` attribute without breaking public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedCharacter,
    UnterminatedString,
    InvalidNumber,
    InvalidBinary,
}

impl LexErrorKind {
    fn as_message(&self) -> &'static str {
        match self {
            Self::UnexpectedCharacter => "unexpected character",
            Self::UnterminatedString => "unterminated string literal",
            Self::InvalidNumber => "invalid numeric literal",
            Self::InvalidBinary => "invalid binary literal",
        }
    }
}

/// Self-contained lexical error including the offending span and a short
/// snippet of the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
    pub snippet: String,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lex error at line {}, column {}: {} (snippet: {:?})",
            self.span.line,
            self.span.column,
            self.kind.as_message(),
            self.snippet,
        )
    }
}

impl std::error::Error for LexError {}

/// Tokenize a source string into a `Vec<Token>`, halting at the first
/// lexical error. Callers that need streaming or custom recovery should
/// construct a [`Lexer`] directly.
///
/// # Errors
///
/// Returns the first [`LexError`] encountered while scanning `source`.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).collect()
}

/// Stateful Part 21 lexer that tracks line/column positions and wraps
/// logos's byte-level lexer.
///
/// `Lexer` owns its own single-token lookahead buffer (`peeked`) rather than
/// relying on the standard `Peekable` adapter. This avoids the awkward
/// `Option<&Result<Token, LexError>>` return type that `Peekable::peek`
/// would produce and lets the parser stage use a simple `Option<&Token>`
/// style API.
pub struct Lexer<'src> {
    source: &'src str,
    inner: logos::Lexer<'src, TokenKind>,
    prev_end: usize,
    line: u32,
    column: u32,
    peeked: Option<Result<Token, LexError>>,
}

impl<'src> Lexer<'src> {
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            inner: TokenKind::lexer(source),
            prev_end: 0,
            line: 1,
            column: 1,
            peeked: None,
        }
    }

    /// Return a reference to the next token without consuming it. Repeated
    /// calls return the same token until [`Lexer::next`] is called.
    pub fn peek(&mut self) -> Option<&Result<Token, LexError>> {
        if self.peeked.is_none() {
            self.peeked = self.next_from_inner();
        }
        self.peeked.as_ref()
    }

    /// Walk through `source[start..end]` updating `line`/`column`.
    fn advance_over(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        for ch in self.source[start..end].chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    /// Pull the next token directly from logos, bypassing the peek buffer.
    ///
    /// This is the raw implementation used by both [`Lexer::next`] and
    /// [`Lexer::peek`]; callers that want the buffered behavior should use
    /// those public entry points instead.
    fn next_from_inner(&mut self) -> Option<Result<Token, LexError>> {
        let kind_result = self.inner.next()?;
        let range = self.inner.span();

        // Whitespace and comments skipped since the previous token.
        self.advance_over(self.prev_end, range.start);

        // Position recorded on the span is the token's *start*.
        let token_line = self.line;
        let token_column = self.column;

        // Walk through the token body so that the next iteration begins at
        // the correct line/column. This is important for multiline string
        // literals which may contain embedded newlines.
        self.advance_over(range.start, range.end);
        self.prev_end = range.end;

        let span = Span {
            start: range.start,
            end: range.end,
            line: token_line,
            column: token_column,
        };

        let Ok(kind) = kind_result else {
            let raw = span.slice(self.source);
            let snippet = truncate_to_chars(raw, 40);
            return Some(Err(LexError {
                kind: LexErrorKind::UnexpectedCharacter,
                span,
                snippet,
            }));
        };
        Some(Ok(Token { kind, span }))
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(buffered) = self.peeked.take() {
            return Some(buffered);
        }
        self.next_from_inner()
    }
}

/// Truncate `s` to at most `max_chars` Unicode scalar values, never slicing
/// through a multibyte character.
fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => s[..byte_idx].to_string(),
        None => s.to_string(),
    }
}

/// Part 21 token kinds.
///
/// Variants cover every lexical construct defined in ISO 10303-21.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"/\*([^*]|\*+[^*/])*\*+/")]
pub enum TokenKind {
    // --- Section keywords (priority 10 protects them from `Keyword`) ---
    //
    // Part 21 is case insensitive for keywords, so every section marker is
    // matched with a case-insensitive regex via the `(?i)` flag. The hyphen
    // in `ISO-10303-21` / `END-ISO-10303-21` keeps them outside the
    // `Keyword` regex, which matches `[A-Za-z_][A-Za-z0-9_]*` only.
    #[regex(r"(?i)ISO-10303-21", priority = 10)]
    IsoStart,

    #[regex(r"(?i)END-ISO-10303-21", priority = 10)]
    IsoEnd,

    #[regex(r"(?i)HEADER", priority = 10)]
    Header,

    #[regex(r"(?i)DATA", priority = 10)]
    Data,

    #[regex(r"(?i)ENDSEC", priority = 10)]
    EndSec,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    #[token("=")]
    Equals,

    /// Derived attribute marker.
    #[token("*")]
    Asterisk,

    /// Unset / optional attribute marker.
    #[token("$")]
    Dollar,

    /// Real number — must contain a `.`; captures optional sign and exponent.
    #[regex(
        r"[+-]?[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?",
        |lex| lex.slice().parse::<f64>().ok()
    )]
    Real(f64),

    /// Integer — no decimal point; may carry a sign.
    #[regex(
        r"[+-]?[0-9]+",
        |lex| lex.slice().parse::<i64>().ok()
    )]
    Integer(i64),

    /// String literal enclosed in single quotes. Outer quotes are stripped
    /// and the `''` escape is decoded to a single `'`. Part 21 wide-char
    /// escapes (`\X\HH`, `\X2\…\X0\`, `\X4\…\X0\`) remain raw — decoding
    /// those is deferred to a later stage.
    #[regex(
        r"'([^']|'')*'",
        |lex| {
            let s = lex.slice();
            s[1..s.len() - 1].replace("''", "'")
        }
    )]
    String(String),

    /// Entity reference (`#N`). The numeric identifier is stored as `u64`.
    #[regex(
        r"#[0-9]+",
        |lex| lex.slice()[1..].parse::<u64>().ok()
    )]
    EntityRef(u64),

    /// Enumeration literal such as `.T.` or `.MILLI.`. The inner name is
    /// stored without the surrounding dots.
    #[regex(
        r"\.[A-Za-z_][A-Za-z0-9_]*\.",
        |lex| {
            let s = lex.slice();
            s[1..s.len() - 1].to_string()
        }
    )]
    Enum(String),

    /// Hex-encoded binary literal, stored without the surrounding quotes.
    /// The leading hex digit (0–3) encodes the number of unused bits.
    #[regex(
        r#""[0-3][0-9A-Fa-f]*""#,
        |lex| {
            let s = lex.slice();
            s[1..s.len() - 1].to_string()
        }
    )]
    Binary(String),

    /// Identifier / entity type name. Does not include `-`, so hyphenated
    /// section markers (handled above) never match this variant.
    #[regex(
        r"[A-Za-z_][A-Za-z0-9_]*",
        |lex| lex.slice().to_string()
    )]
    Keyword(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_token(src: &str) -> TokenKind {
        TokenKind::lexer(src)
            .next()
            .expect("expected at least one token")
            .expect("expected Ok token")
    }

    #[test]
    fn lex_punctuation() {
        let mut lex = TokenKind::lexer("(),;=*$");
        assert_eq!(lex.next(), Some(Ok(TokenKind::LParen)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::RParen)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Comma)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Semicolon)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Equals)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Asterisk)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Dollar)));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_integer_basic() {
        assert_eq!(first_token("42"), TokenKind::Integer(42));
    }

    #[test]
    fn lex_integer_zero() {
        assert_eq!(first_token("0"), TokenKind::Integer(0));
    }

    #[test]
    fn lex_integer_signed() {
        assert_eq!(first_token("+7"), TokenKind::Integer(7));
        assert_eq!(first_token("-13"), TokenKind::Integer(-13));
    }

    #[test]
    fn lex_real_basic() {
        assert_eq!(first_token("1.23"), TokenKind::Real(1.23));
    }

    #[test]
    fn lex_real_trailing_dot() {
        // Part 21 allows "1." form (trailing dot, no fractional digits).
        assert_eq!(first_token("0."), TokenKind::Real(0.0));
        assert_eq!(first_token("100."), TokenKind::Real(100.0));
    }

    #[test]
    fn lex_real_exponent() {
        assert_eq!(first_token("1.E-07"), TokenKind::Real(1e-7));
        assert_eq!(first_token("1.23e5"), TokenKind::Real(1.23e5));
    }

    #[test]
    fn lex_real_signed_exponent() {
        assert_eq!(first_token("-9.80E+02"), TokenKind::Real(-9.80e2));
    }

    #[test]
    fn lex_real_wins_over_integer() {
        // Logos should match the longest pattern — "1.23" must not be lexed as Integer(1).
        let mut lex = TokenKind::lexer("1.23");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Real(1.23))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_integer_when_no_dot() {
        // A bare "1" has no decimal point, so it is Integer, not Real.
        let mut lex = TokenKind::lexer("1");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(1))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_string_empty() {
        assert_eq!(first_token("''"), TokenKind::String(String::new()));
    }

    #[test]
    fn lex_string_simple() {
        assert_eq!(first_token("'abc'"), TokenKind::String("abc".into()));
    }

    #[test]
    fn lex_string_escaped_quote() {
        // Part 21 uses `''` to embed a single quote; the lexer decodes it.
        assert_eq!(first_token("'a''b'"), TokenKind::String("a'b".into()));
    }

    #[test]
    fn lex_string_only_escaped_quote() {
        // `''''` is a string containing a single `'` (one escape sequence).
        assert_eq!(first_token("''''"), TokenKind::String("'".into()));
    }

    #[test]
    fn lex_string_with_newline() {
        // Newlines inside string literals are permitted by Part 21.
        assert_eq!(
            first_token("'line1\nline2'"),
            TokenKind::String("line1\nline2".into())
        );
    }

    #[test]
    fn lex_string_multibyte_korean() {
        // Logos is built on regex-syntax which treats `[^']` as "any UTF-8
        // codepoint other than `'`", so multibyte characters pass through.
        assert_eq!(first_token("'한글'"), TokenKind::String("한글".into()));
    }

    #[test]
    fn lex_string_multibyte_japanese() {
        assert_eq!(first_token("'日本語'"), TokenKind::String("日本語".into()));
    }

    #[test]
    fn lex_entity_ref_small() {
        assert_eq!(first_token("#1"), TokenKind::EntityRef(1));
    }

    #[test]
    fn lex_entity_ref_large() {
        assert_eq!(first_token("#1234567"), TokenKind::EntityRef(1_234_567));
    }

    #[test]
    fn lex_enum_bool_true() {
        assert_eq!(first_token(".T."), TokenKind::Enum("T".into()));
    }

    #[test]
    fn lex_enum_unit() {
        assert_eq!(first_token(".MILLI."), TokenKind::Enum("MILLI".into()));
    }

    #[test]
    fn lex_binary_zero() {
        assert_eq!(first_token("\"0\""), TokenKind::Binary("0".into()));
    }

    #[test]
    fn lex_binary_hex() {
        assert_eq!(first_token("\"3FFA\""), TokenKind::Binary("3FFA".into()));
    }

    #[test]
    fn lex_keyword_simple() {
        assert_eq!(
            first_token("CARTESIAN_POINT"),
            TokenKind::Keyword("CARTESIAN_POINT".into())
        );
    }

    #[test]
    fn lex_keyword_leading_underscore() {
        assert_eq!(first_token("_x1"), TokenKind::Keyword("_x1".into()));
    }

    #[test]
    fn lex_keyword_mixed_case() {
        // Logos preserves the original casing in the captured slice.
        assert_eq!(first_token("PlAnE"), TokenKind::Keyword("PlAnE".into()));
    }

    #[test]
    fn lex_section_iso_start_upper() {
        assert_eq!(first_token("ISO-10303-21"), TokenKind::IsoStart);
    }

    #[test]
    fn lex_section_iso_start_lower() {
        // Part 21 is case insensitive; hyphenated markers are matched via (?i).
        assert_eq!(first_token("iso-10303-21"), TokenKind::IsoStart);
    }

    #[test]
    fn lex_section_iso_end() {
        assert_eq!(first_token("END-ISO-10303-21"), TokenKind::IsoEnd);
    }

    #[test]
    fn lex_section_header_data_endsec() {
        assert_eq!(first_token("HEADER"), TokenKind::Header);
        assert_eq!(first_token("DATA"), TokenKind::Data);
        assert_eq!(first_token("ENDSEC"), TokenKind::EndSec);
    }

    #[test]
    fn lex_section_case_insensitive() {
        assert_eq!(first_token("header"), TokenKind::Header);
        assert_eq!(first_token("Data"), TokenKind::Data);
        assert_eq!(first_token("EndSec"), TokenKind::EndSec);
    }

    #[test]
    fn section_keyword_priority_wins_over_keyword() {
        // "HEADER" must lex as the Header token, not as Keyword("HEADER"),
        // because the section marker has a higher declared priority.
        assert_ne!(first_token("HEADER"), TokenKind::Keyword("HEADER".into()));
    }

    #[test]
    fn lex_whitespace_skipped_between_tokens() {
        let mut lex = TokenKind::lexer("HEADER  ;\n\tDATA");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Header)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Semicolon)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Data)));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_comment_skipped_between_tokens() {
        let mut lex = TokenKind::lexer("1 /* ignored */ 2");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(1))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(2))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_multiline_comment_skipped() {
        let mut lex = TokenKind::lexer("1/* line1\nline2 */2");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(1))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(2))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_minimal_entity_line() {
        // A realistic Part 21 line: `#1=LINE('',#2,#3);`
        let mut lex = TokenKind::lexer("#1=LINE('',#2,#3);");
        assert_eq!(lex.next(), Some(Ok(TokenKind::EntityRef(1))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Equals)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Keyword("LINE".into()))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::LParen)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::String(String::new()))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Comma)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::EntityRef(2))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Comma)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::EntityRef(3))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::RParen)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Semicolon)));
        assert_eq!(lex.next(), None);
    }

    // --- Lexer wrapper (Span / line / column tracking) ---

    fn collect(src: &str) -> Vec<Token> {
        Lexer::new(src)
            .collect::<Result<Vec<_>, _>>()
            .expect("expected all tokens to lex successfully")
    }

    #[test]
    fn span_tracks_single_token() {
        let tokens = collect("HEADER");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Header);
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 6);
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
    }

    #[test]
    fn span_tracks_columns_on_same_line() {
        // "a b" → Keyword("a") col 1, Keyword("b") col 3.
        let tokens = collect("a b");
        assert_eq!(tokens[0].span.column, 1);
        assert_eq!(tokens[1].span.column, 3);
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[1].span.line, 1);
    }

    #[test]
    fn span_tracks_line_after_newline() {
        // "a\nb" → second token starts at line 2, column 1.
        let tokens = collect("a\nb");
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.column, 1);
    }

    #[test]
    fn span_tracks_multiple_newlines() {
        let tokens = collect("a\n\n\nb");
        assert_eq!(tokens[1].span.line, 4);
        assert_eq!(tokens[1].span.column, 1);
    }

    #[test]
    fn span_advances_past_crlf() {
        // Windows-style line endings should advance lines by one per `\n`.
        let tokens = collect("a\r\nb");
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.column, 1);
    }

    #[test]
    fn span_tracks_column_after_multibyte_char() {
        // '한글' is one string token; the next token should see column 5
        // ('한글' = 2 chars + 2 quotes = 4 chars, next char at column 5).
        let tokens = collect("'한글' a");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::String(_)));
        assert_eq!(tokens[1].span.column, 6); // 4 chars + 1 space → col 6
        assert_eq!(tokens[1].span.line, 1);
    }

    #[test]
    fn span_advances_through_multiline_string() {
        // Newlines inside a string literal should still bump the line counter
        // so that the next token's line/column are correct.
        let tokens = collect("'line1\nline2' x");
        assert!(matches!(tokens[0].kind, TokenKind::String(_)));
        // After the string the cursor is at line 2. The `x` follows a space,
        // so column is 8 on line 2 (|line2'| = 6 chars + space + `x` at 8).
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.column, 8);
    }

    #[test]
    fn span_slice_roundtrip_matches_source() {
        let source = "#1=LINE('',#2,#3);";
        let tokens = collect(source);
        for tok in &tokens {
            let slice = tok.span.slice(source);
            // Slices must line up with byte offsets and decode as valid UTF-8.
            assert_eq!(&source[tok.span.start..tok.span.end], slice);
        }
    }

    // --- peek() ---

    #[test]
    fn peek_returns_same_token_twice() {
        let mut lex = Lexer::new("HEADER ; DATA");
        let first = lex.peek().cloned();
        let second = lex.peek().cloned();
        assert_eq!(first, second);
        assert!(matches!(
            first,
            Some(Ok(Token {
                kind: TokenKind::Header,
                ..
            }))
        ));
    }

    #[test]
    fn peek_then_next_returns_buffered_token() {
        let mut lex = Lexer::new("HEADER ; DATA");
        let peeked = lex.peek().cloned();
        let next = lex.next();
        assert_eq!(peeked, next);
    }

    #[test]
    fn peek_does_not_consume_token() {
        let mut lex = Lexer::new("HEADER ; DATA");
        let _ = lex.peek();
        // next() should yield the same token that was peeked, and then a
        // subsequent next() should yield the *following* token.
        assert!(matches!(
            lex.next(),
            Some(Ok(Token {
                kind: TokenKind::Header,
                ..
            }))
        ));
        assert!(matches!(
            lex.next(),
            Some(Ok(Token {
                kind: TokenKind::Semicolon,
                ..
            }))
        ));
        assert!(matches!(
            lex.next(),
            Some(Ok(Token {
                kind: TokenKind::Data,
                ..
            }))
        ));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn peek_at_end_returns_none() {
        let mut lex = Lexer::new("HEADER");
        let _ = lex.next();
        assert!(lex.peek().is_none());
        assert!(lex.next().is_none());
    }

    // --- tokenize() + LexError ---

    #[test]
    fn tokenize_returns_vec_of_tokens() {
        let toks = tokenize("HEADER ; ENDSEC ;").expect("should lex cleanly");
        let kinds: Vec<_> = toks.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Header,
                TokenKind::Semicolon,
                TokenKind::EndSec,
                TokenKind::Semicolon,
            ]
        );
    }

    #[test]
    fn tokenize_reports_unexpected_character() {
        let err = tokenize("#1 = @INVALID").expect_err("expected a lex error");
        assert_eq!(err.kind, LexErrorKind::UnexpectedCharacter);
        assert!(err.snippet.contains('@'));
        assert_eq!(err.span.line, 1);
    }

    #[test]
    fn tokenize_reports_unterminated_string_as_error() {
        // A lone `'` with no closing quote should fail to lex.
        let err = tokenize("'abc").expect_err("unterminated string must error");
        // In this stage logos maps the failed match to UnexpectedCharacter.
        // A later stage may refine this to UnterminatedString.
        assert!(matches!(
            err.kind,
            LexErrorKind::UnexpectedCharacter | LexErrorKind::UnterminatedString
        ));
    }

    #[test]
    fn lex_error_display_has_line_column_and_snippet() {
        let err = tokenize("\n  @").expect_err("expected a lex error");
        let msg = err.to_string();
        assert!(msg.contains("line 2"));
        assert!(msg.contains("column 3"));
        assert!(msg.contains("unexpected character"));
        assert!(msg.contains('@'));
    }

    #[test]
    fn lex_error_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let err = tokenize("@").expect_err("expected a lex error");
        assert_error(&err);
    }
}
