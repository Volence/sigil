//! escape: AS's backslash escape grammar inside string and character literals.
//!
//! A `"..."` string is stored by the lexer in its SOURCE form, backslashes and
//! all: `Tok::Str` holds the text between the quotes verbatim. Its VALUE is that
//! text with every escape replaced by the character it denotes, and this module
//! computes it. A value that has to become a literal again (a string handed to a
//! macro as an argument) goes back through [`quote`], so a token only ever
//! carries the source form and no value is unescaped twice. A `'...'` character
//! constant is packed at lex time and takes [`unescape_bytes`] there.
//!
//! # The grammar, measured against asl 1.42 Beta Bld 212
//!
//! Every row is a reading from an exit-0 listing of
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, one construct per probe file; every
//! refusal is that build's non-zero answer. The probes and their listings are in
//! `docs/superpowers/notes/2026-09-11-as-string-escapes/`.
//!
//! ```text
//!   \\  \"  \'          the character itself: 5C 22 27
//!   \a \b \e \h \i      07 08 1B 27 22       (either case: \A is 07, \H is 27)
//!   \n \r \t            0A 0D 09             (either case)
//!   \xHH \XHH           hex, AT MOST two digits, zero digits is 00:
//!                       "\x414" is 41 34, "\xG" is 00 47, "\x" is 00
//!   \0ooo               octal, a 0 then AT MOST three digits:
//!                       "\012" is 0A, "\0123" is 53, "\01234" is 53 34
//!   \ddd                decimal, first digit 1-9, AT MOST three digits:
//!                       "\12" is 0C, "\123" is 7B, "\1234" is 7B 34
//!   \{expr}             string interpolation (the caller supplies the value)
//! ```
//!
//! Refused, each with asl's `error #2010: invalid escape sequence`: every other
//! letter (`\c \d \f \g \j \k \l \m \o \p \q \s \u \v \w \y \z`, both cases),
//! every other punctuation character, a space, and a backslash that ends the
//! literal. Refused with `error #1320: range overflow`: a numeric escape above
//! 255 (`\256`, `\999`, `\0400`) and an octal escape that holds an 8 or a 9
//! (`\08`, `\018`, `\09`). The digit run is read with 0-9 in both numeric forms,
//! which is why `\08` is a range error rather than `\0` followed by `8`.
//!
//! An escaped quote does not close the literal: `dc.b "a\";b"` is `61 22 3B 62`,
//! so the `;` inside it is a character and not a comment. [`literal_end`] is that
//! rule for every scanner that looks for the end of a literal.
//!
//! # The code page applies to the escaped character
//!
//! An escape yields a character, and that character then goes through the live
//! `charset` like any other: with `charset $41,$11` live, asl assembles `dc.b
//! "A\x41\65"` as `11 11 11`, `dc.w '\x41'` as `0011` and `move.w #"\x41",d0` as
//! `303C 0011`. A raw byte that bypassed the page would read `11 41 41`. The
//! one place a character is not translated is unchanged: the STRING target of a
//! two-operand `charset` stores its characters raw, escapes processed
//! (`charset $50,"\x41A"` then `dc.b "PQ"` is `41 41`).

use std::fmt;

/// Why a literal has no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EscapeError {
    /// A backslash that starts no escape, carrying the offending sequence (`\c`),
    /// or `\` alone when the backslash ends the literal. asl: `error #2010`.
    Invalid(String),
    /// A numeric escape above 255, or an octal escape holding an 8 or a 9,
    /// carrying the sequence as written. asl: `error #1320: range overflow`.
    Range(String),
    /// A `\{expr}` interpolation the caller gave no value, carrying `expr`.
    Interp(String),
}

impl fmt::Display for EscapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EscapeError::Invalid(s) if s == "\\" => {
                write!(f, "a `\\` at the end of a literal starts no escape sequence")
            }
            EscapeError::Invalid(s) => write!(f, "invalid escape sequence `{s}`"),
            EscapeError::Range(s) if s.starts_with("\\0") => write!(
                f,
                "escape sequence `{s}` is out of range: after `\\0` the digits are octal (0 to 7) and the value is at most 255"
            ),
            EscapeError::Range(s) => {
                write!(f, "escape sequence `{s}` is out of range: a character code is at most 255")
            }
            EscapeError::Interp(e) => write!(f, "string interpolation `\\{{{e}}}` has no value here"),
        }
    }
}

/// The value of one literal body, the text between its quotes: every escape
/// replaced by the character it denotes, each `\{expr}` replaced by what
/// `interp` returns for `expr`, `None` making it an [`EscapeError::Interp`].
///
/// A numeric or letter escape yields the character whose code is the escape's
/// value, so `c as u8` recovers the byte for every one of them. Text pasted by
/// `interp` is taken as it is and never scanned for escapes again.
pub(crate) fn unescape(
    raw: &str,
    interp: &mut dyn FnMut(&str) -> Option<String>,
) -> Result<String, EscapeError> {
    let mut out = String::with_capacity(raw.len());
    walk(raw, &mut |piece| {
        match piece {
            Piece::Text(t) => out.push_str(t),
            Piece::Byte(b) => out.push(char::from(b)),
            Piece::Interp(e) => match interp(e) {
                Some(v) => out.push_str(&v),
                None => return Err(EscapeError::Interp(e.to_string())),
            },
        }
        Ok(())
    })?;
    Ok(out)
}

/// [`unescape`] for a context with no way to evaluate an interpolation: every
/// `\{expr}` is an [`EscapeError::Interp`].
pub(crate) fn unescape_plain(raw: &str) -> Result<String, EscapeError> {
    unescape(raw, &mut |_| None)
}

/// [`unescape`] that leaves every `\{expr}` in place, verbatim, for a later
/// interpolation step to fold. For the string evaluator's nested literals
/// (`substr("\{n}...", ...)`), whose interpolation runs where the enclosing
/// value is bound.
pub(crate) fn unescape_keep_interp(raw: &str) -> Result<String, EscapeError> {
    unescape(raw, &mut |e| Some(format!("\\{{{e}}}")))
}

/// The value of a character constant's body as BYTES: source text contributes
/// its own bytes, an escape the one byte it denotes. A `\{expr}` is an
/// [`EscapeError::Interp`]; the lexer that packs character constants has no
/// evaluator.
pub(crate) fn unescape_bytes(raw: &str) -> Result<Vec<u8>, EscapeError> {
    let mut out = Vec::with_capacity(raw.len());
    walk(raw, &mut |piece| {
        match piece {
            Piece::Text(t) => out.extend_from_slice(t.as_bytes()),
            Piece::Byte(b) => out.push(b),
            Piece::Interp(e) => return Err(EscapeError::Interp(e.to_string())),
        }
        Ok(())
    })?;
    Ok(out)
}

/// The index of the quote that closes the literal opened at `bytes[open]`, or
/// `None` when the line ends first. A backslash consumes the byte after it, so
/// an escaped quote does not close the literal.
///
/// Inside a `"…"` string, a `\{expr}` interpolation is scanned as ONE unit,
/// and the literals its expression holds are skipped whole: asl reads
/// `dc.b "\{strlen("}")}",$EE` as one string and writes `31 EE` (probe
/// `v_interp_brace_in_quote`), so the quote that opens `"}"` neither closes the
/// outer literal nor lets its `}` end the interpolation. An interpolation with
/// no closing brace is scanned as the two characters `\{`, and [`escape_at`]
/// refuses it where the string is used.
pub(crate) fn literal_end(bytes: &[u8], open: usize) -> Option<usize> {
    let quote = bytes[open];
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if quote == b'"' && bytes.get(i + 1) == Some(&b'{') => {
                i = interp_close(bytes, i + 2).map_or(i + 2, |close| close + 1);
            }
            b'\\' => i += 2,
            b if b == quote => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// The index of the `}` that ends a `\{expr}` interpolation whose expression
/// begins at `bytes[start]`, or `None` when nothing ends it. A string or
/// character literal inside the expression is skipped whole ([`literal_end`]),
/// so a `}` or a quote inside it belongs to that literal.
pub(crate) fn interp_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'}' => return Some(i),
            b'"' | b'\'' => i = literal_end(bytes, i)? + 1,
            _ => i += 1,
        }
    }
    None
}

/// The source form of a string value: the body a `"..."` literal needs so that
/// [`unescape`] gives `value` back. A backslash and a double quote are the two
/// characters that need an escape; every other character stands for itself.
pub(crate) fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    for c in value.chars() {
        if matches!(c, '\\' | '"') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// One step of a literal body: a run of source text, the byte an escape
/// denotes, or the expression text of a `\{expr}`.
enum Piece<'a> {
    Text(&'a str),
    Byte(u8),
    Interp(&'a str),
}

/// Split `raw` into [`Piece`]s, left to right, handing each to `sink`.
fn walk(
    raw: &str,
    sink: &mut dyn FnMut(Piece<'_>) -> Result<(), EscapeError>,
) -> Result<(), EscapeError> {
    let mut rest = raw;
    while let Some(pos) = rest.find('\\') {
        if pos > 0 {
            sink(Piece::Text(&rest[..pos]))?;
        }
        let seq = &rest[pos..];
        let (piece, len) = escape_at(seq)?;
        sink(piece)?;
        rest = &seq[len..];
    }
    if !rest.is_empty() {
        sink(Piece::Text(rest))?;
    }
    Ok(())
}

/// The escape at the head of `seq` (which starts with its backslash), and how
/// many bytes of `seq` it spans.
fn escape_at(seq: &str) -> Result<(Piece<'_>, usize), EscapeError> {
    let b = seq.as_bytes();
    let Some(&c) = b.get(1) else {
        return Err(EscapeError::Invalid("\\".to_string()));
    };
    Ok(match c {
        b'\\' | b'"' | b'\'' => (Piece::Byte(c), 2),
        b'{' => {
            let Some(close) = interp_close(b, 2) else {
                return Err(EscapeError::Invalid("\\{".to_string()));
            };
            (Piece::Interp(&seq[2..close]), close + 1)
        }
        b'x' | b'X' => {
            let end = 2 + run(&b[2..], 2, u8::is_ascii_hexdigit);
            (Piece::Byte(radix_value(&seq[2..end], 16) as u8), end)
        }
        b'0' => {
            let end = 2 + run(&b[2..], 3, u8::is_ascii_digit);
            let text = &seq[1..end];
            let v = radix_value(text, 8);
            if text.bytes().any(|d| d > b'7') || v > 0xFF {
                return Err(EscapeError::Range(format!("\\{text}")));
            }
            (Piece::Byte(v as u8), end)
        }
        b'1'..=b'9' => {
            let end = 2 + run(&b[2..], 2, u8::is_ascii_digit);
            let text = &seq[1..end];
            let v = radix_value(text, 10);
            if v > 0xFF {
                return Err(EscapeError::Range(format!("\\{text}")));
            }
            (Piece::Byte(v as u8), end)
        }
        _ => {
            let v = match c.to_ascii_lowercase() {
                b'a' => 0x07,
                b'b' => 0x08,
                b'e' => 0x1B,
                b'h' => 0x27,
                b'i' => 0x22,
                b'n' => 0x0A,
                b'r' => 0x0D,
                b't' => 0x09,
                _ => {
                    let ch = seq[1..].chars().next().unwrap_or(char::from(c));
                    return Err(EscapeError::Invalid(format!("\\{ch}")));
                }
            };
            (Piece::Byte(v), 2)
        }
    })
}

/// The length of the run of bytes at the head of `b` that satisfy `pred`,
/// capped at `max`.
fn run(b: &[u8], max: usize, pred: fn(&u8) -> bool) -> usize {
    b.iter().take(max).take_while(|x| pred(x)).count()
}

/// `text` read in `radix`, the empty text being 0. A digit that is not valid in
/// the radix counts as 0 here; the octal caller refuses those first. Three
/// digits cannot overflow a `u32`.
fn radix_value(text: &str, radix: u32) -> u32 {
    text.chars().fold(0, |v, d| v * radix + d.to_digit(radix).unwrap_or(0))
}
