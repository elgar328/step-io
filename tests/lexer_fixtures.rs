//! Integration tests that run the lexer against every committed STEP fixture.
//!
//! These tests verify that the lexer can tokenize real AP203/AP214/AP242
//! files without errors and that well-known structural invariants
//! (section markers, balanced parentheses) are preserved in the token stream.

use step_io::{Lexer, Token, TokenKind, tokenize};

const FIXTURES: &[(&str, &str)] = &[
    ("box_ap203", include_str!("fixtures/box_ap203.step")),
    ("box_ap214_cd", include_str!("fixtures/box_ap214_cd.step")),
    ("box_ap214_dis", include_str!("fixtures/box_ap214_dis.step")),
    ("box_ap214_is", include_str!("fixtures/box_ap214_is.step")),
    ("box_ap242_dis", include_str!("fixtures/box_ap242_dis.step")),
];

fn tokenize_fixture(name: &str, source: &str) -> Vec<Token> {
    match tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => panic!("fixture {name} failed to tokenize: {err}"),
    }
}

#[test]
fn every_fixture_tokenizes_without_error() {
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        assert!(!tokens.is_empty(), "fixture {name} produced no tokens");
    }
}

#[test]
fn every_fixture_starts_with_iso_header_marker() {
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        assert_eq!(
            tokens[0].kind,
            TokenKind::IsoStart,
            "fixture {name} did not start with ISO-10303-21"
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Semicolon,
            "fixture {name} missing semicolon after ISO-10303-21"
        );
    }
}

#[test]
fn every_fixture_ends_with_iso_end_marker() {
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        let n = tokens.len();
        assert!(n >= 2, "fixture {name} too short");
        assert_eq!(
            tokens[n - 2].kind,
            TokenKind::IsoEnd,
            "fixture {name} did not end with END-ISO-10303-21"
        );
        assert_eq!(
            tokens[n - 1].kind,
            TokenKind::Semicolon,
            "fixture {name} missing final semicolon"
        );
    }
}

#[test]
fn every_fixture_contains_header_data_endsec() {
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        let mut has_header = false;
        let mut has_data = false;
        let mut endsec_count = 0;
        for tok in &tokens {
            match tok.kind {
                TokenKind::Header => has_header = true,
                TokenKind::Data => has_data = true,
                TokenKind::EndSec => endsec_count += 1,
                _ => {}
            }
        }
        assert!(has_header, "fixture {name} missing HEADER");
        assert!(has_data, "fixture {name} missing DATA");
        // One ENDSEC after HEADER, one after DATA.
        assert_eq!(endsec_count, 2, "fixture {name} expected 2 ENDSEC");
    }
}

#[test]
fn every_fixture_uses_streaming_lexer_identically() {
    // The streaming Lexer and the `tokenize` convenience must agree.
    for (name, source) in FIXTURES {
        let via_tokenize = tokenize_fixture(name, source);
        let via_lexer: Vec<Token> = Lexer::new(source)
            .collect::<Result<_, _>>()
            .unwrap_or_else(|e| panic!("fixture {name} streaming lex failed: {e}"));
        assert_eq!(
            via_tokenize.len(),
            via_lexer.len(),
            "fixture {name} token count mismatch"
        );
        for (a, b) in via_tokenize.iter().zip(via_lexer.iter()) {
            assert_eq!(a.kind, b.kind, "fixture {name} kind mismatch");
            assert_eq!(a.span, b.span, "fixture {name} span mismatch");
        }
    }
}

#[test]
fn every_fixture_has_balanced_parentheses() {
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        let mut depth: i64 = 0;
        for tok in &tokens {
            match tok.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    assert!(
                        depth >= 0,
                        "fixture {name}: unexpected ) at line {} col {}",
                        tok.span.line,
                        tok.span.column,
                    );
                }
                _ => {}
            }
        }
        assert_eq!(
            depth, 0,
            "fixture {name}: parentheses not balanced (final depth {depth})"
        );
    }
}

#[test]
fn every_token_span_roundtrips_through_source_slice() {
    // `span.start..span.end` must always be a valid char-boundary slice
    // that matches whatever the lexer saw. This catches any byte-offset
    // drift in the line/column tracker.
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        for (i, tok) in tokens.iter().enumerate() {
            assert!(
                source.is_char_boundary(tok.span.start),
                "fixture {name} token {i}: span.start not on char boundary"
            );
            assert!(
                source.is_char_boundary(tok.span.end),
                "fixture {name} token {i}: span.end not on char boundary"
            );
            let slice = &source[tok.span.start..tok.span.end];
            assert_eq!(
                tok.span.slice(source),
                slice,
                "fixture {name} token {i}: Span::slice mismatch"
            );
            assert!(!slice.is_empty(), "fixture {name} token {i}: empty span");
        }
    }
}

#[test]
fn every_token_span_monotonically_advances() {
    // Byte offsets and (line, column) pairs must be strictly non-decreasing
    // across the token stream. Regressions would indicate the Lexer wrapper
    // is rewinding state incorrectly.
    for (name, source) in FIXTURES {
        let tokens = tokenize_fixture(name, source);
        let mut prev_end = 0usize;
        let mut prev_line_col = (0u32, 0u32);
        for tok in &tokens {
            assert!(
                tok.span.start >= prev_end,
                "fixture {name}: token starts before previous ends"
            );
            let this = (tok.span.line, tok.span.column);
            assert!(
                this >= prev_line_col,
                "fixture {name}: line/column regressed from {prev_line_col:?} to {this:?}"
            );
            prev_end = tok.span.end;
            prev_line_col = this;
        }
    }
}

#[test]
fn box_ap214_is_entity_165_complex_entity_snapshot() {
    // Snapshot of the complex entity at `#165` in box_ap214_is.step.
    //
    // The textual form in the fixture is:
    //
    //     #165 = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)
    //     GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#169)) GLOBAL_UNIT_ASSIGNED_CONTEXT
    //     ((#166,#167,#168)) REPRESENTATION_CONTEXT('Context #1',
    //       '3D Context with UNIT and UNCERTAINTY') );
    //
    // This test locks the token sequence produced for that statement so that
    // any future regression in the lexer — especially around how complex
    // entities, nested parentheses, and string literals with embedded
    // whitespace are handled — is detected immediately.

    let (_, source) = FIXTURES
        .iter()
        .find(|(name, _)| *name == "box_ap214_is")
        .expect("box_ap214_is fixture must be present");
    let tokens = tokenize_fixture("box_ap214_is", source);

    // Find the `#165 = ...;` definition statement. The reference `#165`
    // appears earlier in the file inside another entity's attribute list,
    // so we need to match the *definition* by requiring an `=` immediately
    // after the entity ref.
    let start = tokens
        .windows(2)
        .position(|w| w[0].kind == TokenKind::EntityRef(165) && w[1].kind == TokenKind::Equals)
        .expect("entity #165 definition should exist in box_ap214_is");
    let end = tokens[start..]
        .iter()
        .position(|t| t.kind == TokenKind::Semicolon)
        .map(|offset| start + offset + 1)
        .expect("statement should end with a semicolon");
    let slice: Vec<TokenKind> = tokens[start..end].iter().map(|t| t.kind.clone()).collect();

    let expected: Vec<TokenKind> = vec![
        TokenKind::EntityRef(165),
        TokenKind::Equals,
        TokenKind::LParen, // opens the complex entity wrapper
        TokenKind::Keyword("GEOMETRIC_REPRESENTATION_CONTEXT".into()),
        TokenKind::LParen,
        TokenKind::Integer(3),
        TokenKind::RParen,
        TokenKind::Keyword("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT".into()),
        TokenKind::LParen,
        TokenKind::LParen,
        TokenKind::EntityRef(169),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::Keyword("GLOBAL_UNIT_ASSIGNED_CONTEXT".into()),
        TokenKind::LParen,
        TokenKind::LParen,
        TokenKind::EntityRef(166),
        TokenKind::Comma,
        TokenKind::EntityRef(167),
        TokenKind::Comma,
        TokenKind::EntityRef(168),
        TokenKind::RParen,
        TokenKind::RParen,
        TokenKind::Keyword("REPRESENTATION_CONTEXT".into()),
        TokenKind::LParen,
        TokenKind::String("Context #1".into()),
        TokenKind::Comma,
        TokenKind::String("3D Context with UNIT and UNCERTAINTY".into()),
        TokenKind::RParen,
        TokenKind::RParen, // closes the complex entity wrapper
        TokenKind::Semicolon,
    ];

    assert_eq!(slice, expected, "#165 complex entity token stream drifted");
}
