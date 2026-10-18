use std::borrow::Cow;
use std::io::Read;

use logos::Logos;

use super::entity::Error;

/// Position information for a lexed token.
///
/// `start`/`end` are byte offsets into the input. `line` is 1-indexed by
/// `\n` occurrences. `column` is 1-indexed and counts characters, so a
/// multibyte character advances it by one (in an entity read as ISO 8859-1,
/// every byte is a character; see [`parse`](crate::parser::parse)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u64,
    pub end: u64,
    pub line: u32,
    pub column: u32,
}

/// A lexed token carrying its kind and position.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

/// Lexical error categories.
///
/// `UnexpectedCharacter` (a token that does not match any rule) and
/// `InvalidNumber` (a numeric/ref literal that is out of range or non-finite —
/// classified from the offending snippet) are produced by the lexer. The
/// remaining variants are declared for forward compatibility.
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
            Self::InvalidNumber => "numeric literal out of range",
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

/// Bytes asked of the input per read.
const CHUNK: usize = 64 * 1024;

/// Where the record splitter is: between tokens, or inside a string,
/// comment or anchor (the only tokens that may hold a `;`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Split {
    Plain,
    Str,
    Comment,
    Anchor,
}

/// Streaming Part 21 lexer that tracks line/column positions and wraps
/// logos's byte-level lexer.
///
/// The input is read a chunk at a time and cut into records, each running up
/// to and including a `;` outside strings, comments and anchors: one entity
/// with the blanks and comments before it. Tokens never cross a record's
/// end, so each is lexed from the record alone and only the record being
/// lexed (with the chunk after it) is held. A record reads as UTF-8 if its
/// bytes are valid UTF-8, and as ISO 8859-1 otherwise.
///
/// `Lexer` owns its own single-token lookahead buffer (`peeked`) rather than
/// relying on the standard `Peekable` adapter, so the parser can borrow the
/// peeked token.
pub(crate) struct Lexer<R> {
    input: R,
    buf: Vec<u8>,
    /// Input offset of `buf[0]`.
    base: u64,
    /// Next byte to lex.
    pos: usize,
    /// End of the record being lexed.
    end: usize,
    /// How far the splitter has looked for the next record's end, and what
    /// it is inside of there.
    scan: usize,
    split: Split,
    /// Watching for `<` (anchors) too, while rescanning a record that has
    /// one; and how far the record has been checked for one.
    careful: bool,
    checked: usize,
    /// The input has no more bytes.
    eof: bool,
    /// The record being lexed reads as UTF-8 (else ISO 8859-1).
    utf8: bool,
    /// The record being lexed when it is UTF-8, as text: token text is cut
    /// from it without checking the bytes again.
    text: String,
    line: u32,
    column: u32,
    peeked: Option<Result<Token, Error>>,
}

impl<R: Read> Lexer<R> {
    pub(crate) fn new(input: R) -> Self {
        Self {
            input,
            buf: Vec::new(),
            base: 0,
            pos: 0,
            end: 0,
            scan: 0,
            split: Split::Plain,
            careful: false,
            checked: 0,
            eof: false,
            utf8: true,
            text: String::new(),
            line: 1,
            column: 1,
            peeked: None,
        }
    }

    /// Return a reference to the next token without consuming it. Repeated
    /// calls return the same token until [`Lexer::next`] is called.
    pub(crate) fn peek(&mut self) -> Option<&Result<Token, Error>> {
        if self.peeked.is_none() {
            self.peeked = self.next_from_inner();
        }
        self.peeked.as_ref()
    }

    /// Read more of the input after the unread bytes, dropping the lexed
    /// ones. `false` at the end of the input.
    fn fill(&mut self) -> Result<bool, Error> {
        if self.eof {
            return Ok(false);
        }
        if self.pos > 0 {
            self.buf.drain(..self.pos);
            self.base += self.pos as u64;
            self.scan -= self.pos;
            self.end -= self.pos;
            self.checked = self.checked.saturating_sub(self.pos);
            self.pos = 0;
        }
        let n = self.buf.len();
        self.buf.resize(n + CHUNK, 0);
        loop {
            match self.input.read(&mut self.buf[n..]) {
                Ok(got) => {
                    self.buf.truncate(n + got);
                    self.eof = got == 0;
                    return Ok(got > 0);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => {
                    // Nothing more is lexed after a failed read.
                    self.buf.clear();
                    (self.end, self.scan, self.checked) = (0, 0, 0);
                    self.eof = true;
                    return Err(Error::Io {
                        kind: e.kind(),
                        message: e.to_string(),
                    });
                }
            }
        }
    }

    /// The end of the next record, if the bytes read so far hold it: just
    /// past the next `;` outside strings, comments and anchors. Stops short
    /// of a byte whose meaning hangs on the one after it while that one is
    /// still unread.
    fn find_cut(&mut self) -> Option<usize> {
        let mut found = self.scan_for_cut();
        // An anchor may hold a `;` or a `'`. Anchors are rare, so a record is
        // first scanned without them, and again watching for them as soon as
        // a `<` shows in what was scanned.
        if !self.careful {
            let upto = found.unwrap_or(self.scan);
            let from = self.checked.max(self.pos);
            let anchor = memchr::memchr(b'<', &self.buf[from..upto]).is_some();
            self.checked = upto;
            if anchor {
                self.careful = true;
                self.scan = self.pos;
                self.split = Split::Plain;
                found = self.scan_for_cut();
            }
        }
        if found.is_some() {
            self.careful = false;
        }
        found
    }

    /// [`find_cut`](Self::find_cut)'s scan, from where the last one stopped.
    fn scan_for_cut(&mut self) -> Option<usize> {
        let b = &self.buf;
        let n = b.len();
        let more = !self.eof;
        let mut i = self.scan;
        while i < n {
            match self.split {
                Split::Plain => {
                    let hit = memchr::memchr3(b';', b'\'', b'/', &b[i..]);
                    if self.careful {
                        let run = hit.unwrap_or(n - i);
                        if let Some(k) = memchr::memchr(b'<', &b[i..i + run]) {
                            i += k + 1;
                            self.split = Split::Anchor;
                            continue;
                        }
                    }
                    let Some(k) = hit else {
                        i = n;
                        break;
                    };
                    i += k;
                    match (b[i], b.get(i + 1)) {
                        (b';', _) => {
                            self.scan = i + 1;
                            return Some(i + 1);
                        }
                        (b'\'', _) => {
                            i += 1;
                            self.split = Split::Str;
                        }
                        (_, Some(b'*')) => {
                            i += 2;
                            self.split = Split::Comment;
                        }
                        (_, None) if more => break,
                        _ => i += 1,
                    }
                }
                // A quote ends the string unless another follows it.
                Split::Str => match memchr::memchr(b'\'', &b[i..]) {
                    None => i = n,
                    Some(k) => {
                        i += k;
                        match b.get(i + 1) {
                            Some(b'\'') => i += 2,
                            None if more => break,
                            _ => {
                                i += 1;
                                self.split = Split::Plain;
                            }
                        }
                    }
                },
                Split::Comment => match memchr::memchr(b'*', &b[i..]) {
                    None => i = n,
                    Some(k) => {
                        i += k;
                        match b.get(i + 1) {
                            Some(b'/') => {
                                i += 2;
                                self.split = Split::Plain;
                            }
                            None if more => break,
                            _ => i += 1,
                        }
                    }
                },
                Split::Anchor => match memchr::memchr(b'>', &b[i..]) {
                    None => i = n,
                    Some(k) => {
                        i += k + 1;
                        self.split = Split::Plain;
                    }
                },
            }
        }
        self.scan = i;
        None
    }

    /// Make the next record the one being lexed; `false` at the end of the
    /// input.
    fn next_record(&mut self) -> Result<bool, Error> {
        loop {
            if let Some(cut) = self.find_cut() {
                self.end = cut;
                break;
            }
            if !self.fill()? {
                self.end = self.buf.len();
                self.scan = self.end;
                break;
            }
        }
        self.text.clear();
        match std::str::from_utf8(&self.buf[self.pos..self.end]) {
            Ok(t) => {
                self.text.push_str(t);
                self.utf8 = true;
            }
            Err(_) => self.utf8 = false,
        }
        Ok(self.end > self.pos)
    }

    /// Walk through `buf[start..end]` updating `line`/`column`: a column
    /// per character, which in UTF-8 is a byte that does not continue one.
    fn advance_over(&mut self, start: usize, end: usize) {
        let latin1 = !self.utf8;
        for &b in &self.buf[start..end] {
            if b == b'\n' {
                self.line = self.line.saturating_add(1);
                self.column = 1;
            } else if latin1 || b & 0xC0 != 0x80 {
                self.column = self.column.saturating_add(1);
            }
        }
    }

    /// The record start, for cutting token text from `text`.
    fn record_start(&self) -> usize {
        self.end - self.text.len()
    }

    /// The text of `buf[range]`: UTF-8 in a UTF-8 record, decoded from
    /// ISO 8859-1 otherwise.
    fn text(&self, range: std::ops::Range<usize>) -> Cow<'_, str> {
        if self.utf8 {
            let at = self.record_start();
            Cow::Borrowed(
                self.text
                    .get(range.start - at..range.end - at)
                    .unwrap_or_default(),
            )
        } else {
            Cow::Owned(self.buf[range].iter().map(|&b| char::from(b)).collect())
        }
    }

    /// The text of `buf[range]`, which the token rules keep to ASCII.
    fn ascii(&self, range: std::ops::Range<usize>) -> &str {
        if self.utf8 {
            let at = self.record_start();
            self.text
                .get(range.start - at..range.end - at)
                .unwrap_or_default()
        } else {
            std::str::from_utf8(&self.buf[range]).unwrap_or_default()
        }
    }

    /// The token a matched rule stands for, with its value; `None` for a
    /// number out of range or not finite.
    fn token_kind(&self, raw: Raw, r: std::ops::Range<usize>) -> Option<TokenKind> {
        let inner = r.start + 1..r.end - 1;
        Some(match raw {
            Raw::IsoStart => TokenKind::IsoStart,
            Raw::IsoEnd => TokenKind::IsoEnd,
            Raw::Header => TokenKind::Header,
            Raw::Data => TokenKind::Data,
            Raw::EndSec => TokenKind::EndSec,
            Raw::LParen => TokenKind::LParen,
            Raw::RParen => TokenKind::RParen,
            Raw::Comma => TokenKind::Comma,
            Raw::Semicolon => TokenKind::Semicolon,
            Raw::Equals => TokenKind::Equals,
            Raw::Asterisk => TokenKind::Asterisk,
            Raw::Dollar => TokenKind::Dollar,
            // Reject non-finite (e.g. `1.E999` -> inf) as an InvalidNumber
            // error rather than letting inf into the model.
            Raw::Real => TokenKind::Real(
                self.ascii(r)
                    .parse::<f64>()
                    .ok()
                    .filter(|f| f.is_finite())?,
            ),
            Raw::Integer => TokenKind::Integer(self.ascii(r).parse().ok()?),
            Raw::String => TokenKind::String(self.text(inner).replace("''", "'")),
            Raw::EntityRef => TokenKind::EntityRef(self.ascii(r.start + 1..r.end).parse().ok()?),
            Raw::Enum => TokenKind::Enum(self.ascii(inner).to_owned()),
            Raw::Binary => TokenKind::Binary(self.ascii(inner).to_owned()),
            Raw::Keyword => TokenKind::Keyword(self.ascii(r).to_owned()),
            Raw::AnchorRef => TokenKind::AnchorRef(self.text(r).into_owned()),
        })
    }

    /// Lex the next token, bypassing the peek buffer.
    fn next_from_inner(&mut self) -> Option<Result<Token, Error>> {
        loop {
            if self.pos < self.end {
                let rec = &self.buf[self.pos..self.end];
                let mut inner = Raw::lexer(rec);
                if let Some(raw) = inner.next() {
                    let mut r = inner.span();
                    if raw.is_err() && self.utf8 {
                        // An unmatched character spans all of its bytes.
                        r.end += rec[r.end..]
                            .iter()
                            .take_while(|&&b| b & 0xC0 == 0x80)
                            .count();
                    }
                    return Some(self.token(raw, self.pos + r.start..self.pos + r.end));
                }
                // Only blanks and comments are left in the record.
                self.advance_over(self.pos, self.end);
                self.pos = self.end;
            }
            match self.next_record() {
                Ok(true) => {}
                Ok(false) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }

    /// The token (or lexical error) a rule match at `buf[range]` makes, after
    /// the blanks and comments before it.
    fn token(
        &mut self,
        raw: Result<Raw, ()>,
        range: std::ops::Range<usize>,
    ) -> Result<Token, Error> {
        self.advance_over(self.pos, range.start);
        // Position recorded on the span is the token's *start*.
        let (line, column) = (self.line, self.column);
        // Walk through the token body so that the next token begins at the
        // correct line/column (string literals may span lines).
        self.advance_over(range.start, range.end);
        self.pos = range.end;
        let span = Span {
            start: self.base + range.start as u64,
            end: self.base + range.end as u64,
            line,
            column,
        };
        // A number out of range / non-finite is an InvalidNumber; a character
        // no rule matches, an UnexpectedCharacter.
        let kind = match raw.ok().map(|raw| self.token_kind(raw, range.clone())) {
            Some(Some(kind)) => return Ok(Token { kind, span }),
            Some(None) => LexErrorKind::InvalidNumber,
            None => LexErrorKind::UnexpectedCharacter,
        };
        let snippet = truncate_to_chars(&self.text(range), 40);
        Err(Error::Lex(LexError {
            kind,
            span,
            snippet,
        }))
    }
}

impl<R: Read> Iterator for Lexer<R> {
    type Item = Result<Token, Error>;

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
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    IsoStart,
    IsoEnd,
    Header,
    Data,
    EndSec,
    LParen,
    RParen,
    Comma,
    Semicolon,
    Equals,
    /// Derived attribute marker.
    Asterisk,
    /// Unset / optional attribute marker.
    Dollar,
    /// Real number — must contain a `.`; captures optional sign and exponent.
    Real(f64),
    /// Integer — no decimal point; may carry a sign.
    Integer(i64),
    /// String literal enclosed in single quotes. Outer quotes are stripped
    /// and the `''` escape is decoded to a single `'`. The backslash escapes
    /// (`\X\HH`, `\X2\…\X0\`, `\X4\…\X0\`, …) and line breaks stay as
    /// written; the parser decodes them into [`Attribute::String`](crate::parser::Attribute::String).
    String(String),
    /// Entity reference (`#N`). The numeric identifier is stored as `u64`.
    EntityRef(u64),
    /// Enumeration literal such as `.T.` or `.MILLI.`. The inner name is
    /// stored without the surrounding dots.
    Enum(String),
    /// Hex-encoded binary literal, stored without the surrounding quotes.
    /// The leading hex digit (0–3) encodes the number of unused bits.
    Binary(String),
    /// Identifier / entity type name.
    Keyword(String),
    /// P21 edition 3 anchor reference (`<name>` or `<url#name>`). The
    /// whole bracketed form is captured as a single token; the parser
    /// currently discards these (ed.3 sections are skip-only — see
    /// [`crate::parser::ParseWarning::Ed3SectionDiscarded`]).
    AnchorRef(String),
}

/// The rules the source's bytes are matched against, one per [`TokenKind`];
/// the [`Lexer`] turns a match into the token with its value.
#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(utf8 = false)]
#[logos(skip r"[ \t\r\n]+")]
#[logos(skip r"/\*((?-u:[^*])|\*+(?-u:[^*/]))*\*+/")]
enum Raw {
    // --- Section keywords (priority 10 protects them from `Keyword`) ---
    //
    // Part 21 is case insensitive for keywords, so every section marker is
    // matched with an ASCII case-insensitive regex via the `(?i-u)` flags.
    // The hyphen in `ISO-10303-21` / `END-ISO-10303-21` keeps them outside
    // the `Keyword` regex, which matches `[A-Za-z_][A-Za-z0-9_]*` only.
    #[regex(r"(?i-u)ISO-10303-21", priority = 10)]
    IsoStart,
    #[regex(r"(?i-u)END-ISO-10303-21", priority = 10)]
    IsoEnd,
    #[regex(r"(?i-u)HEADER", priority = 10)]
    Header,
    #[regex(r"(?i-u)DATA", priority = 10)]
    Data,
    #[regex(r"(?i-u)ENDSEC", priority = 10)]
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
    #[token("*")]
    Asterisk,
    #[token("$")]
    Dollar,
    #[regex(r"[+-]?[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?")]
    Real,
    #[regex(r"[+-]?[0-9]+")]
    Integer,
    // Any byte but a quote, or a doubled quote.
    #[regex(r"'((?-u:[^'])|'')*'")]
    String,
    #[regex(r"#[0-9]+")]
    EntityRef,
    #[regex(r"\.[A-Za-z_][A-Za-z0-9_]*\.")]
    Enum,
    #[regex(r#""[0-3][0-9A-Fa-f]*""#)]
    Binary,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Keyword,
    #[regex(r"<(?-u:[^>])+>")]
    AnchorRef,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lex `src` given as text.
    fn lexer(src: &str) -> Lexer<&[u8]> {
        Lexer::new(src.as_bytes())
    }

    /// Every token of `src`, or its first lexical error.
    fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
        lexer(src)
            .map(|t| {
                t.map_err(|e| match e {
                    Error::Lex(e) => e,
                    other => panic!("{other:?}"),
                })
            })
            .collect()
    }

    fn first_token(src: &str) -> TokenKind {
        lexer(src)
            .next()
            .expect("expected at least one token")
            .expect("expected Ok token")
            .kind
    }

    fn kinds(src: &str) -> impl Iterator<Item = Result<TokenKind, Error>> + '_ {
        lexer(src).map(|t| t.map(|t| t.kind))
    }

    #[test]
    fn lex_punctuation() {
        let mut lex = kinds("(),;=*$");
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
        let mut lex = kinds("1.23");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Real(1.23))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_integer_when_no_dot() {
        // A bare "1" has no decimal point, so it is Integer, not Real.
        let mut lex = kinds("1");
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
    fn lex_anchor_ref_simple() {
        assert_eq!(
            first_token("<TestAnchor>"),
            TokenKind::AnchorRef("<TestAnchor>".into())
        );
    }

    #[test]
    fn lex_anchor_ref_with_url() {
        assert_eq!(
            first_token("<testAnchorAndData.stp#TestAnchor>"),
            TokenKind::AnchorRef("<testAnchorAndData.stp#TestAnchor>".into())
        );
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
        let mut lex = kinds("HEADER  ;\n\tDATA");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Header)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Semicolon)));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Data)));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_comment_skipped_between_tokens() {
        let mut lex = kinds("1 /* ignored */ 2");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(1))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(2))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_multiline_comment_skipped() {
        let mut lex = kinds("1/* line1\nline2 */2");
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(1))));
        assert_eq!(lex.next(), Some(Ok(TokenKind::Integer(2))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn lex_minimal_entity_line() {
        // A realistic Part 21 line: `#1=LINE('',#2,#3);`
        let mut lex = kinds("#1=LINE('',#2,#3);");
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
        lexer(src)
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
    fn spans_cover_the_source_bytes() {
        // No blanks between the tokens, so their spans tile the source.
        let source = "#1=LINE('',#2,#3);";
        let tokens = collect(source);
        let mut at = 0;
        for tok in &tokens {
            assert_eq!(tok.span.start, at);
            at = tok.span.end;
        }
        assert_eq!(at, source.len() as u64);
    }

    // --- peek() ---

    #[test]
    fn peek_returns_same_token_twice() {
        let mut lex = lexer("HEADER ; DATA");
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
        let mut lex = lexer("HEADER ; DATA");
        let peeked = lex.peek().cloned();
        let next = lex.next();
        assert_eq!(peeked, next);
    }

    #[test]
    fn peek_does_not_consume_token() {
        let mut lex = lexer("HEADER ; DATA");
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
        let mut lex = lexer("HEADER");
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

    fn collect_bytes(src: &[u8]) -> Vec<Token> {
        Lexer::new(src)
            .collect::<Result<Vec<_>, _>>()
            .expect("lexes")
    }

    #[test]
    fn latin1_source_decodes_strings_and_counts_bytes() {
        // 0xC0 and 0xE9 alone are not UTF-8, so the whole source is Latin-1.
        let tokens = collect_bytes(b"'\xC0b' /* \xE9 */ <a\xE9> x");
        assert_eq!(tokens[0].kind, TokenKind::String("\u{C0}b".into()));
        assert_eq!(tokens[1].kind, TokenKind::AnchorRef("<a\u{E9}>".into()));
        // Offsets count the bytes given; columns count one per byte.
        assert_eq!((tokens[1].span.start, tokens[1].span.column), (13, 14));
        assert_eq!((tokens[2].span.start, tokens[2].span.column), (18, 19));
    }

    #[test]
    fn utf8_source_counts_columns_in_characters() {
        let tokens = collect_bytes("'é' x".as_bytes());
        assert_eq!(tokens[0].kind, TokenKind::String("é".into()));
        assert_eq!((tokens[1].span.start, tokens[1].span.column), (5, 5));
    }

    #[test]
    fn unexpected_multibyte_character_is_one_error() {
        let mut lexer = lexer("1 é 2");
        assert!(lexer.next().is_some_and(|t| t.is_ok()));
        let Error::Lex(err) = lexer.next().unwrap().expect_err("é is not a token") else {
            panic!("not a lexical error");
        };
        assert_eq!((err.span.start, err.span.end, err.span.column), (2, 4, 3));
        assert_eq!(err.snippet, "é");
        let two = lexer.next().unwrap().expect("lexing goes on after it");
        assert_eq!((two.kind, two.span.column), (TokenKind::Integer(2), 5));
    }

    /// Hands out at most `step` bytes per read, and every other read is
    /// interrupted.
    struct Trickle<'a> {
        src: &'a [u8],
        step: usize,
        interrupt: bool,
    }

    impl Read for Trickle<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            self.interrupt = !self.interrupt;
            if self.interrupt {
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            let n = self.step.min(out.len()).min(self.src.len());
            out[..n].copy_from_slice(&self.src[..n]);
            self.src = &self.src[n..];
            Ok(n)
        }
    }

    /// The spans the lexer gives, error or not.
    fn spans<R: Read>(lexer: Lexer<R>) -> Vec<(u64, u64, u32, u32)> {
        lexer
            .map(|t| {
                let span = match t {
                    Ok(t) => t.span,
                    Err(Error::Lex(e)) => e.span,
                    Err(e) => panic!("{e:?}"),
                };
                (span.start, span.end, span.line, span.column)
            })
            .collect()
    }

    /// Semicolons inside strings (also after a doubled quote), comments and
    /// anchors do not end a record, and unclosed ones lex as they would in
    /// one piece; so do inputs handed over a few bytes at a time.
    #[test]
    fn records_lex_as_the_whole_input_would() {
        let sources: [&[u8]; 7] = [
            b"HEADER;FILE_DESCRIPTION(('a;b','it''s;'),'2;1');\n/* c;o*m/ment; **/#1=X(<a;b>,'x''',.T.,\n1.5E+3,\"0F\",$,*,'\xC3\xA9;');#2=Y('');ENDSEC;",
            b"#1=X('abc;);#2=Y(1);",
            b"#1=X(1);/* open; comment",
            b"#1=X(<open;anchor);#2=Y(1);",
            b"#1=X('ab'';c');x/y;*/;",
            b"#1=X(<a'b;c>);#2=Y('z');",
            b"END-ISO-10303-21;",
        ];
        for src in sources {
            // The rules matched over the whole input in one go.
            let mut whole = Raw::lexer(src);
            let mut want = Vec::new();
            while whole.next().is_some() {
                want.push((whole.span().start as u64, whole.span().end as u64));
            }
            let got = spans(Lexer::new(src));
            let got_ranges: Vec<_> = got.iter().map(|&(s, e, _, _)| (s, e)).collect();
            assert_eq!(got_ranges, want, "{:?}", String::from_utf8_lossy(src));
            for step in 1..=7 {
                let trickle = Trickle {
                    src,
                    step,
                    interrupt: false,
                };
                assert_eq!(spans(Lexer::new(trickle)), got, "step {step}");
            }
        }
    }

    /// Each record reads as UTF-8 when it is valid UTF-8 and as ISO 8859-1
    /// otherwise, whatever the rest of the input is.
    #[test]
    fn each_record_picks_its_encoding() {
        let tokens = collect_bytes(b"#1=X('\xE9');\n#2=X('\xC3\xA9');");
        let strings: Vec<_> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(strings, ["\u{E9}", "\u{E9}"]);
    }

    /// A failing input stops the lexer with `Error::Io`.
    #[test]
    fn input_failure_is_an_io_error() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk gone"))
            }
        }
        let io_error = Some(Err(Error::Io {
            kind: std::io::ErrorKind::Other,
            message: "disk gone".into(),
        }));
        let mut lexer = Lexer::new(Broken);
        assert_eq!(lexer.next(), io_error);
        assert_eq!(lexer.next(), None);
        // What was read before the failure lexes; what is left of the record
        // it broke off in does not.
        let mut lexer = Lexer::new(b"#1=X(1);#2=Y(".chain(Broken));
        let kinds: Vec<_> = lexer.by_ref().take(7).map(|t| t.map(|t| t.kind)).collect();
        assert_eq!(kinds.last(), Some(&Ok(TokenKind::Semicolon)));
        assert_eq!(lexer.next(), io_error);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn section_markers_ignore_ascii_case_only() {
        assert_eq!(first_token("endsec"), TokenKind::EndSec);
        assert_eq!(first_token("Iso-10303-21"), TokenKind::IsoStart);
        // `ſ` folds to `s` in Unicode, not in Part 21.
        assert_ne!(first_token("ENDſEC"), TokenKind::EndSec);
    }
}
