//! Parser/lexer robustness: adversarial input is a structured, graceful
//! `ParseError` (never a panic, abort, or silent corruption). Covers the
//! nesting-depth guard, structured error preservation, out-of-range / non-finite
//! numbers, and synthetic-id overflow. Inline adversarial byte probes, not
//! valid-CAD fixtures.

use step_io::{LexError, LexErrorKind, ParseError, read};

const HEADER: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION(('probe'),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn doc(body: &str) -> String {
    format!("{HEADER}{body}{FOOTER}")
}

/// Deeply nested parameters → graceful `NestingTooDeep`, NOT a stack-overflow
/// abort. 100k `(` would overflow the stack without the depth guard; the guard
/// errors at depth 257, so the test process survives and returns `Err`.
#[test]
fn deep_nesting_is_graceful_not_stack_overflow() {
    let body = format!("#1=A({}{});\n", "(".repeat(100_000), ")".repeat(100_000));
    let err = read(doc(&body).as_bytes()).unwrap_err();
    assert!(
        matches!(err, ParseError::NestingTooDeep { .. }),
        "expected NestingTooDeep, got {err:?}"
    );
}

/// A normal-depth aggregate (a few levels) still parses fine — the guard is far
/// above any real nesting.
#[test]
fn normal_nesting_still_parses() {
    let body = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n";
    let (_m, report) = read(doc(body).as_bytes()).expect("normal depth parses");
    assert_eq!(report.validated, 1);
}

/// Errors are structured (not flattened to a String): a caller can match the
/// concrete `ParseError` variant.
#[test]
fn duplicate_id_is_structured() {
    let body = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n#1=DIRECTION('',(1.,0.,0.));\n";
    let err = read(doc(body).as_bytes()).unwrap_err();
    assert!(
        matches!(err, ParseError::DuplicateEntityId { id: 1, .. }),
        "expected DuplicateEntityId #1, got {err:?}"
    );
}

/// An integer past i64 range → `LexErrorKind::InvalidNumber` (precise kind, not a
/// generic "unexpected character").
#[test]
fn out_of_range_integer_is_invalid_number() {
    let body = "#1=A(123456789012345678901234567890);\n";
    let err = read(doc(body).as_bytes()).unwrap_err();
    assert!(
        matches!(
            err,
            ParseError::Lex(LexError {
                kind: LexErrorKind::InvalidNumber,
                ..
            })
        ),
        "expected Lex(InvalidNumber), got {err:?}"
    );
}

/// A non-finite real (`1.0E999` → inf) is rejected as `InvalidNumber` rather than
/// silently entering the model as `inf`.
#[test]
fn non_finite_real_is_invalid_number() {
    let body = "#1=A(1.0E999);\n";
    let err = read(doc(body).as_bytes()).unwrap_err();
    assert!(
        matches!(
            err,
            ParseError::Lex(LexError {
                kind: LexErrorKind::InvalidNumber,
                ..
            })
        ),
        "expected Lex(InvalidNumber), got {err:?}"
    );
}

/// An entity id at `u64::MAX` must not overflow synthetic-id allocation
/// (`max + 1`): debug builds would panic, release would wrap to 0 and overwrite
/// an existing entity. The read succeeds, both entities survive, accounting holds.
/// (`cargo test` is a debug build, so a clean run proves no arithmetic-overflow
/// panic.)
#[test]
fn max_u64_id_does_not_overflow_or_corrupt() {
    let body = "#18446744073709551615=CARTESIAN_POINT('',(0.,0.,0.));\n\
#1=DIRECTION('',(1.,0.,0.));\n";
    let (_m, report) = read(doc(body).as_bytes()).expect("parses without panic");
    assert_eq!(
        report.validated + report.dropped.len(),
        report.n_in + report.n_synth,
        "accounting invariant"
    );
    assert_eq!(report.validated, 2, "both entities kept (no overwrite)");
    assert_eq!(report.dropped.len(), 0);
}
