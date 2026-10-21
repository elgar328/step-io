//! The value of a Part 21 STRING: decoding what files contain, and the one
//! form step-io writes.
//!
//! The lexer strips the quotes and undoes `''`; [`decode`] does the rest.
//! [`encode`] writes printable ASCII as itself and every other character as a
//! `\X2\` or `\X4\` run, so `decode(encode(s))` gives back `s` for any string.

use std::fmt::Write as _;

/// A decoded STRING, with the non-standard forms met on the way.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Decoded {
    pub text: String,
    /// An escape could not be decoded and was kept as written: a lone `\`,
    /// an unterminated or broken `\X2\` / `\X4\` run, a `\P?\` code page
    /// other than `A`, or an unknown directive.
    pub malformed: bool,
    /// A hex escape used lower-case digits, which Part 21 does not allow.
    pub lowercase_hex: bool,
}

/// Which ISO 8859 part `\S\` maps into, as chosen by `\P?\`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    /// `\PA\` (the default): ISO 8859-1, which matches U+0000..U+00FF.
    Latin1,
    /// `\PB\` .. `\PI\`, which step-io does not map.
    Other,
}

/// Decode a STRING's contents as the lexer hands them over (quotes stripped,
/// `''` already undone).
///
/// Line breaks in the text are dropped first: writers break long strings
/// across lines, sometimes inside an escape. An escape that cannot be decoded
/// is kept as written, so the value holds its backslash as a character.
pub(crate) fn decode(raw: &str) -> Decoded {
    let joined;
    let s = if raw.contains(['\r', '\n']) {
        joined = raw.replace(['\r', '\n'], "");
        joined.as_str()
    } else {
        raw
    };
    let mut out = Decoded {
        text: String::with_capacity(s.len()),
        malformed: false,
        lowercase_hex: false,
    };
    if !s.contains('\\') {
        out.text.push_str(s);
        return out;
    }
    let bytes = s.as_bytes();
    let mut page = Page::Latin1;
    let mut i = 0;
    while i < s.len() {
        let Some(offset) = s[i..].find('\\') else {
            out.text.push_str(&s[i..]);
            break;
        };
        out.text.push_str(&s[i..i + offset]);
        i += offset;
        let rest = &s[i..];
        if rest.starts_with("\\\\") {
            out.text.push('\\');
            i += 2;
        } else if rest.starts_with("\\X2\\") || rest.starts_with("\\X4\\") {
            i = extended(s, i, &mut out);
        } else if rest.starts_with("\\X\\") {
            match hex_value(bytes.get(i + 3..i + 5))
                .and_then(|(value, lower)| Some((char::from_u32(value)?, lower)))
            {
                Some((ch, lower)) => {
                    out.text.push(ch);
                    out.lowercase_hex |= lower;
                    i += 5;
                }
                None => i = keep_backslash(&mut out, i),
            }
        } else if rest.starts_with("\\S\\") {
            match bytes.get(i + 3) {
                Some(&c @ 0x20..=0x7E) if page == Page::Latin1 => {
                    out.text.push(char::from(c + 0x80));
                    i += 4;
                }
                Some(0x20..=0x7E) => {
                    out.text.push_str(&s[i..i + 4]);
                    out.malformed = true;
                    i += 4;
                }
                _ => i = keep_backslash(&mut out, i),
            }
        } else if let [b'\\', b'P', letter @ b'A'..=b'I', b'\\', ..] = rest.as_bytes() {
            if *letter == b'A' {
                page = Page::Latin1;
            } else {
                page = Page::Other;
                out.text.push_str(&s[i..i + 4]);
                out.malformed = true;
            }
            i += 4;
        } else {
            i = keep_backslash(&mut out, i);
        }
    }
    out
}

/// Keep the backslash at `i` as a character; scanning resumes after it.
fn keep_backslash(out: &mut Decoded, i: usize) -> usize {
    out.text.push('\\');
    out.malformed = true;
    i + 1
}

/// Decode the `\X2\` or `\X4\` run starting at `i`; return where scanning
/// resumes. A broken run is kept whole, as written.
fn extended(s: &str, i: usize, out: &mut Decoded) -> usize {
    let width = if s.as_bytes()[i + 2] == b'2' { 4 } else { 8 };
    let Some(len) = s[i + 4..].find("\\X0\\") else {
        return keep_backslash(out, i);
    };
    let end = i + 4 + len;
    let digits = &s.as_bytes()[i + 4..end];
    let mut lower = false;
    let mut values = Vec::with_capacity(digits.len() / width);
    let well_formed = !digits.is_empty()
        && digits.len() % width == 0
        && digits
            .chunks(width)
            .all(|chunk| match hex_value(Some(chunk)) {
                Some((value, l)) => {
                    values.push(value);
                    lower |= l;
                    true
                }
                None => false,
            });
    let decoded: Option<String> = if !well_formed {
        None
    } else if width == 4 {
        values
            .iter()
            .map(|&v| u16::try_from(v).ok())
            .collect::<Option<Vec<u16>>>()
            .and_then(|units| {
                char::decode_utf16(units)
                    .collect::<Result<String, _>>()
                    .ok()
            })
    } else {
        values.iter().map(|&v| char::from_u32(v)).collect()
    };
    if let Some(text) = decoded {
        out.text.push_str(&text);
        out.lowercase_hex |= lower;
    } else {
        out.text.push_str(&s[i..end + 4]);
        out.malformed = true;
    }
    end + 4
}

/// The value of a group of hex digits, and whether any was lower-case.
fn hex_value(digits: Option<&[u8]>) -> Option<(u32, bool)> {
    let digits = digits?;
    let mut value = 0u32;
    let mut lower = false;
    for &d in digits {
        let v = match d {
            b'0'..=b'9' => d - b'0',
            b'A'..=b'F' => d - b'A' + 10,
            b'a'..=b'f' => {
                lower = true;
                d - b'a' + 10
            }
            _ => return None,
        };
        value = value << 4 | u32::from(v);
    }
    Some((value, lower))
}

/// A STRING literal, quotes included. Printable ASCII is written as itself
/// (`'` doubled, `\` doubled); every other character goes into a
/// `\X2\…\X0\` run, or a `\X4\…\X0\` run beyond the BMP, so the file stays
/// plain ASCII.
pub(crate) fn encode(s: &str) -> String {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Run {
        Ascii,
        X2,
        X4,
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    let mut run = Run::Ascii;
    for ch in s.chars() {
        let code = u32::from(ch);
        let want = match ch {
            ' '..='~' => Run::Ascii,
            _ if code <= 0xFFFF => Run::X2,
            _ => Run::X4,
        };
        if want != run {
            if run != Run::Ascii {
                out.push_str("\\X0\\");
            }
            match want {
                Run::Ascii => {}
                Run::X2 => out.push_str("\\X2\\"),
                Run::X4 => out.push_str("\\X4\\"),
            }
            run = want;
        }
        match want {
            Run::Ascii => match ch {
                '\'' => out.push_str("''"),
                '\\' => out.push_str("\\\\"),
                _ => out.push(ch),
            },
            Run::X2 => {
                let _ = write!(out, "{code:04X}");
            }
            Run::X4 => {
                let _ = write!(out, "{code:08X}");
            }
        }
    }
    if run != Run::Ascii {
        out.push_str("\\X0\\");
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::{Decoded, decode, encode};

    fn ok(text: &str) -> Decoded {
        Decoded {
            text: text.to_owned(),
            malformed: false,
            lowercase_hex: false,
        }
    }

    fn malformed(text: &str) -> Decoded {
        Decoded {
            malformed: true,
            ..ok(text)
        }
    }

    fn lowercase(text: &str) -> Decoded {
        Decoded {
            lowercase_hex: true,
            ..ok(text)
        }
    }

    #[test]
    fn decodes_the_standard_forms() {
        let cases = [
            ("", ok("")),
            ("plain", ok("plain")),
            (r"a\\b", ok(r"a\b")),
            (r"\X2\D55CAE00\X0\", ok("한글")),
            (r"\X4\0001F600\X0\", ok("😀")),
            (r"\X2\D83DDE00\X0\", ok("😀")),
            (r"\X\E9t\X\E9", ok("été")),
            (r"\X\0Aend", ok("\nend")),
            (r"Volumenk\S\vrper", ok("Volumenkörper")),
            (r"\S\'", ok("§")),
            (r"\PA\\S\v", ok("ö")),
            // From a real file: an escaped backslash right after a run.
            (r"DIM\\w \\u\X2\2334\X0\\\n2", ok("DIM\\w \\u\u{2334}\\n2")),
        ];
        for (raw, want) in cases {
            assert_eq!(decode(raw), want, "raw {raw:?}");
        }
    }

    #[test]
    fn drops_line_breaks_even_inside_an_escape() {
        assert_eq!(
            decode("E:\\\\converted_stp\\\\1000\n28_0c3a8f1c_0.stp"),
            ok(r"E:\converted_stp\100028_0c3a8f1c_0.stp")
        );
        assert_eq!(decode("\\X2\\D5\r\n5C\\X0\\"), ok("한"));
    }

    #[test]
    fn flags_lower_case_hex() {
        assert_eq!(decode(r"\X2\c5d0\X0\"), lowercase("에"));
        assert_eq!(decode(r"\X\e9"), lowercase("é"));
    }

    #[test]
    fn keeps_what_it_cannot_decode() {
        let cases = [
            (r"NAUO8\Redondeo1", malformed(r"NAUO8\Redondeo1")),
            (r"trailing\", malformed(r"trailing\")),
            (
                r"pil d\X2\E6B1DFEB\X0\aplama",
                malformed(r"pil d\X2\E6B1DFEB\X0\aplama"),
            ),
            (r"\X2\00E\X0\", malformed(r"\X2\00E\X0\")),
            (r"\X2\00G9\X0\", malformed(r"\X2\00G9\X0\")),
            (r"\X4\00110000\X0\", malformed(r"\X4\00110000\X0\")),
            (r"\X2\00E9", malformed(r"\X2\00E9")),
            (r"\X\G1", malformed(r"\X\G1")),
            (r"\PB\\S\a", malformed(r"\PB\\S\a")),
            (r"\N\", malformed(r"\N\")),
        ];
        for (raw, want) in cases {
            assert_eq!(decode(raw), want, "raw {raw:?}");
        }
    }

    #[test]
    fn encodes_like_brep_to_step() {
        let cases = [
            ("", "''"),
            ("abc", "'abc'"),
            ("it's", "'it''s'"),
            (r"a\b", r"'a\\b'"),
            ("é", r"'\X2\00E9\X0\'"),
            ("한글", r"'\X2\D55CAE00\X0\'"),
            ("a한글b", r"'a\X2\D55CAE00\X0\b'"),
            ("😀", r"'\X4\0001F600\X0\'"),
            ("한😀a", r"'\X2\D55C\X0\\X4\0001F600\X0\a'"),
            ("line1\nline2", r"'line1\X2\000A\X0\line2'"),
            ("\u{7F}", r"'\X2\007F\X0\'"),
        ];
        for (input, want) in cases {
            assert_eq!(encode(input), want, "input {input:?}");
        }
    }

    /// What the lexer hands `decode` for an encoded literal.
    fn lexed(literal: &str) -> String {
        literal[1..literal.len() - 1].replace("''", "'")
    }

    #[test]
    fn decode_undoes_encode() {
        let pool: Vec<char> = "aZ09 '\\~\t\r\n\u{0}\u{7F}\u{80}éÿ한\u{FFFD}\u{FFFF}😀\u{10FFFF}"
            .chars()
            .collect();
        let mut state = 0x2545_F491_u64;
        for _ in 0..2000 {
            let mut s = String::new();
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = usize::try_from(state % 12).expect("small");
            for _ in 0..len {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let pick = state % u64::try_from(pool.len()).expect("small");
                s.push(pool[usize::try_from(pick).expect("small")]);
            }
            let encoded = encode(&s);
            assert!(encoded.is_ascii(), "{s:?} -> {encoded}");
            assert_eq!(decode(&lexed(&encoded)), ok(&s), "{s:?} -> {encoded}");
        }
    }
}
