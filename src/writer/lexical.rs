//! Low-level Part 21 lexical helpers.
//!
//! These routines turn individual scalar values into the exact text shape the
//! Part 21 grammar expects. They are the leaves of `serialize_attr`.

use super::WriteError;

/// Render a finite `f64` as a Part 21 real literal.
///
/// Returns `Err(InvalidFloat)` for `NaN` or `±Infinity`; Part 21 admits only
/// finite reals. Output shape follows the OCCT convention so that round-tripped
/// files stay diff-friendly against existing fixtures:
///
/// - `0.0` and `-0.0` → `"0."`
/// - `|f| < 1e-4` or `|f| >= 1e15` → exponential `1.E-07` (uppercase `E`,
///   at least two digits in the exponent)
/// - otherwise → decimal form with a trailing `.` when missing (`1.`, `1.5`)
pub(super) fn format_real(f: f64) -> Result<String, WriteError> {
    if !f.is_finite() {
        return Err(WriteError::InvalidFloat {
            value: f,
            context: "real attribute",
        });
    }
    if f == 0.0 {
        return Ok("0.".into());
    }
    let abs = f.abs();
    if (1e-4..1e15).contains(&abs) {
        Ok(format_decimal(f))
    } else {
        Ok(format_exponential(f))
    }
}

fn format_decimal(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') { s } else { format!("{s}.") }
}

fn format_exponential(f: f64) -> String {
    // Rust's `{:E}` yields something like "1E-7" or "1.5E-7". We want the
    // mantissa to always contain a decimal point and the exponent to be at
    // least two digits with an explicit sign.
    let raw = format!("{f:E}");
    let (mantissa, exponent) = raw.split_once('E').expect("{:E} always emits an 'E'");
    let mantissa = if mantissa.contains('.') {
        mantissa.to_string()
    } else {
        format!("{mantissa}.")
    };
    let (sign, digits) = match exponent.as_bytes().first() {
        Some(b'-') => ("-", &exponent[1..]),
        Some(b'+') => ("+", &exponent[1..]),
        _ => ("+", exponent),
    };
    let padded = if digits.len() < 2 {
        format!("0{digits}")
    } else {
        digits.to_string()
    };
    format!("{mantissa}E{sign}{padded}")
}

/// Wrap a string in single quotes and escape inner single quotes by doubling.
/// Non-ASCII wide-char escapes (`\X2\...\X0\`) are out of scope for W-A.
pub(super) fn format_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push('\'');
        }
        out.push(ch);
    }
    out.push('\'');
    out
}

/// Render an enumeration value as `.VALUE.`.
pub(super) fn format_enum(name: &str) -> String {
    format!(".{name}.")
}

/// Render an entity reference as `#N`.
pub(super) fn format_ref(id: u64) -> String {
    format!("#{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(f: f64) -> String {
        format_real(f).expect("finite")
    }

    #[test]
    fn zero_and_negative_zero_render_as_dot() {
        assert_eq!(real(0.0), "0.");
        assert_eq!(real(-0.0), "0.");
    }

    #[test]
    fn integral_reals_gain_trailing_dot() {
        assert_eq!(real(1.0), "1.");
        assert_eq!(real(-1.0), "-1.");
        assert_eq!(real(100.0), "100.");
        assert_eq!(real(-64.0), "-64.");
    }

    #[test]
    fn fractional_reals_round_trip_display() {
        assert_eq!(real(1.5), "1.5");
        assert_eq!(real(-2.75), "-2.75");
        assert_eq!(real(0.270_598_022_562), "0.270598022562");
    }

    #[test]
    fn very_small_reals_use_exponential() {
        assert_eq!(real(1e-7), "1.E-07");
        assert_eq!(real(-1e-7), "-1.E-07");
        assert_eq!(real(6.597_363_721_994e-8), "6.597363721994E-08");
    }

    #[test]
    fn very_large_reals_use_exponential() {
        assert_eq!(real(1e15), "1.E+15");
        assert_eq!(real(-2.5e20), "-2.5E+20");
    }

    #[test]
    fn boundary_at_1e_minus_4_stays_decimal() {
        // 1e-4 is exactly at the boundary — abs < 1e-4 switches; equal stays decimal.
        assert_eq!(real(1e-4), "0.0001");
    }

    #[test]
    fn non_finite_reals_error() {
        assert!(matches!(
            format_real(f64::NAN),
            Err(WriteError::InvalidFloat { .. })
        ));
        assert!(matches!(
            format_real(f64::INFINITY),
            Err(WriteError::InvalidFloat { .. })
        ));
        assert!(matches!(
            format_real(f64::NEG_INFINITY),
            Err(WriteError::InvalidFloat { .. })
        ));
    }

    #[test]
    fn strings_wrap_and_escape_single_quotes() {
        assert_eq!(format_string(""), "''");
        assert_eq!(format_string("abc"), "'abc'");
        assert_eq!(format_string("a'b"), "'a''b'");
        assert_eq!(format_string("''"), "''''''");
    }

    #[test]
    fn lexer_format_string_round_trip_with_escaped_quote() {
        // The lexer decodes `''` to `'`, and `format_string` re-escapes
        // `'` to `''`. The pair must round-trip a Part 21 string literal
        // back to its source form for strings whose only special character
        // is the single quote.
        use crate::parser::lexer::{TokenKind, tokenize};

        let source = "'Philip''s Head Screw'";
        let tokens = tokenize(source).expect("lex");
        let Some(TokenKind::String(decoded)) = tokens.first().map(|t| &t.kind) else {
            panic!("expected leading String token");
        };
        assert_eq!(decoded, "Philip's Head Screw");
        assert_eq!(format_string(decoded), source);
    }

    #[test]
    fn enum_wraps_in_dots() {
        assert_eq!(format_enum("T"), ".T.");
        assert_eq!(format_enum("MILLI"), ".MILLI.");
    }

    #[test]
    fn entity_refs_prefix_hash() {
        assert_eq!(format_ref(1), "#1");
        assert_eq!(format_ref(12345), "#12345");
    }
}
