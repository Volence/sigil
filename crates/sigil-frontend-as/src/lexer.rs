//! lexer: CPU-context-aware tokeniser for one logical line.

// `lex_line` + helpers are consumed by the parser (next task); unused until then.

use crate::token::{Punct, Tok, Token};
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level, SourceId, Span};

/// Tokenise one logical line. `base` is the byte offset of `line`'s first char
/// within the whole source (so spans are absolute). Comments (`;`→EOL) are
/// stripped. Returns a diagnostic on a malformed token.
pub fn lex_line(line: &str, cpu: Cpu, source: SourceId, base: u32) -> Result<Vec<Token>, Diagnostic> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let span_at = |start: usize, end: usize| Span {
        source,
        start: base + start as u32,
        end: base + end as u32,
    };
    let err = |start: usize, end: usize, msg: &str| Diagnostic {
        level: Level::Error,
        message: msg.to_string(),
        primary: span_at(start, end),
    };

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => { i += 1; }
            b';' => break, // comment to EOL
            b'"' => {
                let start = i;
                i += 1;
                let s0 = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(err(start, i, "unterminated string literal"));
                }
                let s = std::str::from_utf8(&bytes[s0..i]).unwrap().to_string();
                i += 1; // closing quote
                out.push(Token { tok: Tok::Str(s), span: span_at(start, i) });
            }
            b'$' if cpu == Cpu::Z80 => {
                out.push(Token { tok: Tok::Dollar, span: span_at(i, i + 1) });
                i += 1;
            }
            b'$' => {
                // 68k hex: `$` then hex digits.
                let start = i;
                i += 1;
                let h0 = i;
                while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
                if i == h0 {
                    return Err(err(start, i, "`$` with no hex digits"));
                }
                let v = i64::from_str_radix(std::str::from_utf8(&bytes[h0..i]).unwrap(), 16)
                    .map_err(|_| err(start, i, "malformed hex literal"))?;
                out.push(Token { tok: Tok::Int(v), span: span_at(start, i) });
            }
            _ if c.is_ascii_digit() => {
                // Number: scan an alnum run; trailing `h`/`H` ⇒ hex, else decimal.
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let run = std::str::from_utf8(&bytes[start..i]).unwrap();
                // Float literal (`6.283185307179586`, only meaningful inside a
                // `sin(...)`/`int(...)` builtin argument — see `Tok::Float`):
                // a PLAIN decimal run (never a hex run, which already absorbed
                // any `h` suffix into `run` above) immediately followed by
                // `.` + a digit. A bare trailing `.` with no following digit
                // (or a hex run) is left alone as before.
                if run.bytes().all(|b| b.is_ascii_digit())
                    && i < bytes.len()
                    && bytes[i] == b'.'
                    && i + 1 < bytes.len()
                    && bytes[i + 1].is_ascii_digit()
                {
                    i += 1; // consume '.'
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    // Optional exponent: [eE][+-]?digits+ (only consumed if
                    // well-formed; otherwise left for the next token, matching
                    // how `parse::<f64>` would reject a dangling `e`).
                    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
                        let mut j = i + 1;
                        if j < bytes.len() && matches!(bytes[j], b'+' | b'-') {
                            j += 1;
                        }
                        let exp_digits_start = j;
                        while j < bytes.len() && bytes[j].is_ascii_digit() {
                            j += 1;
                        }
                        if j > exp_digits_start {
                            i = j;
                        }
                    }
                    let text = std::str::from_utf8(&bytes[start..i]).unwrap();
                    let v: f64 = text.parse().map_err(|_| err(start, i, "malformed float literal"))?;
                    out.push(Token { tok: Tok::Float(v), span: span_at(start, i) });
                    continue;
                }
                let v = if let Some(hexs) = run.strip_suffix(['h', 'H']) {
                    i64::from_str_radix(hexs, 16).map_err(|_| err(start, i, "malformed hex literal"))?
                } else if run.bytes().all(|b| b.is_ascii_digit()) {
                    run.parse::<i64>().map_err(|_| err(start, i, "malformed decimal literal"))?
                } else {
                    return Err(err(start, i, "malformed number (hex needs a trailing `h`)"));
                };
                out.push(Token { tok: Tok::Int(v), span: span_at(start, i) });
            }
            _ if is_ident_start(c) => {
                let start = i;
                while i < bytes.len() && is_ident_tail(bytes[i]) {
                    i += 1;
                }
                let s = std::str::from_utf8(&bytes[start..i]).unwrap().to_string();
                out.push(Token { tok: Tok::Ident(s), span: span_at(start, i) });
            }
            _ => {
                // Operators / delimiters (maximal munch for 2-char forms).
                let (p, len) = punct(&bytes[i..])
                    .ok_or_else(|| err(i, i + 1, "unexpected character"))?;
                out.push(Token { tok: Tok::Punct(p), span: span_at(i, i + len) });
                i += len;
            }
        }
    }
    Ok(out)
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'.'
}
fn is_ident_tail(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b'\''
}

/// Match a 1- or 2-byte operator at the head of `b`. 2-char forms first.
fn punct(b: &[u8]) -> Option<(Punct, usize)> {
    use Punct::*;
    let two = if b.len() >= 2 { Some((b[0], b[1])) } else { None };
    match two {
        Some((b'<', b'<')) => return Some((Shl, 2)),
        Some((b'>', b'>')) => return Some((Shr, 2)),
        Some((b'<', b'>')) => return Some((Ne, 2)),
        Some((b'<', b'=')) => return Some((Le, 2)),
        Some((b'>', b'=')) => return Some((Ge, 2)),
        Some((b'|', b'|')) => return Some((OrOr, 2)),
        Some((b'&', b'&')) => return Some((AndAnd, 2)),
        Some((b':', b'=')) => return Some((ColonEq, 2)),
        _ => {}
    }
    let one = match b[0] {
        b'+' => Plus, b'-' => Minus, b'*' => Star, b'/' => Slash,
        b'&' => Amp, b'|' => Pipe, b'=' => Eq, b'<' => Lt, b'>' => Gt,
        b'(' => LParen, b')' => RParen, b',' => Comma, b':' => Colon,
        b'#' => Hash, b'!' => Bang,
        _ => return None,
    };
    Some((one, 1))
}

#[cfg(test)]
mod tests {
    use super::lex_line;
    use crate::token::{Punct, Tok};
    use sigil_ir::backend::Cpu;
    use sigil_span::SourceId;

    fn kinds(src: &str, cpu: Cpu) -> Vec<Tok> {
        lex_line(src, cpu, SourceId(0), 0)
            .unwrap()
            .into_iter()
            .map(|t| t.tok)
            .collect()
    }

    #[test]
    fn z80_hex_decimal_and_dollar() {
        assert_eq!(kinds("0FFh", Cpu::Z80), vec![Tok::Int(0xFF)]);
        assert_eq!(kinds("08000h", Cpu::Z80), vec![Tok::Int(0x8000)]);
        assert_eq!(kinds("38h", Cpu::Z80), vec![Tok::Int(0x38)]);
        assert_eq!(kinds("255", Cpu::Z80), vec![Tok::Int(255)]);
        assert_eq!(kinds("$", Cpu::Z80), vec![Tok::Dollar]);
    }

    #[test]
    fn m68k_dollar_hex() {
        assert_eq!(kinds("$1234", Cpu::M68000), vec![Tok::Int(0x1234)]);
        assert_eq!(kinds("255", Cpu::M68000), vec![Tok::Int(255)]);
    }

    #[test]
    fn m68k_hash_immediate_marker() {
        assert_eq!(
            kinds("#$1234", Cpu::M68000),
            vec![Tok::Punct(Punct::Hash), Tok::Int(0x1234)]
        );
        assert_eq!(
            kinds("move.w #5,d0", Cpu::M68000),
            vec![
                Tok::Ident("move.w".into()),
                Tok::Punct(Punct::Hash),
                Tok::Int(5),
                Tok::Punct(Punct::Comma),
                Tok::Ident("d0".into()),
            ]
        );
    }

    #[test]
    fn identifiers_locals_dotted_and_shadow() {
        assert_eq!(kinds(".loop", Cpu::Z80), vec![Tok::Ident(".loop".into())]);
        assert_eq!(kinds("Seq.fetch", Cpu::Z80), vec![Tok::Ident("Seq.fetch".into())]);
        assert_eq!(kinds("af'", Cpu::Z80), vec![Tok::Ident("af'".into())]);
        // Leading letter (not digit) ⇒ identifier even if it looks hex-ish.
        assert_eq!(kinds("FFh", Cpu::Z80), vec![Tok::Ident("FFh".into())]);
    }

    #[test]
    fn operators_maximal_munch() {
        use Punct::*;
        assert_eq!(
            kinds("a >> 8 & 0FFh", Cpu::Z80),
            vec![Tok::Ident("a".into()), Tok::Punct(Shr), Tok::Int(8), Tok::Punct(Amp), Tok::Int(0xFF)]
        );
        assert_eq!(kinds("<> <= >= << >>", Cpu::Z80),
            vec![Tok::Punct(Ne), Tok::Punct(Le), Tok::Punct(Ge), Tok::Punct(Shl), Tok::Punct(Shr)]);
    }

    #[test]
    fn comment_stripped_and_string_and_indexed() {
        use Punct::*;
        assert_eq!(kinds("nop ; trailing", Cpu::Z80), vec![Tok::Ident("nop".into())]);
        assert_eq!(kinds("\"Z80\"", Cpu::Z80), vec![Tok::Str("Z80".into())]);
        assert_eq!(
            kinds("(ix+sc_flags)", Cpu::Z80),
            vec![Tok::Punct(LParen), Tok::Ident("ix".into()), Tok::Punct(Plus),
                 Tok::Ident("sc_flags".into()), Tok::Punct(RParen)]
        );
    }

    #[test]
    fn malformed_number_is_a_diagnostic_not_a_panic() {
        // Digit-led run containing A–F with no trailing `h` under z80 is an error.
        assert!(lex_line("1F", Cpu::Z80, SourceId(0), 0).is_err());
    }

    #[test]
    // The literal below is `deform_table_sine`'s exact source text
    // (`engine/parallax_macros.inc:223`), verbatim — not a rounded
    // approximation of `TAU` we should "fix" to the constant.
    #[allow(clippy::approx_constant)]
    fn float_literal_lexes_as_a_single_token() {
        // The `deform_table_sine` constant, and simpler decimal cases.
        assert_eq!(kinds("6.283185307179586", Cpu::M68000), vec![Tok::Float(6.283185307179586)]);
        assert_eq!(kinds("0.5", Cpu::M68000), vec![Tok::Float(0.5)]);
        // A bare integer (no `.digit` following) still lexes as `Int`, not `Float`.
        assert_eq!(kinds("6", Cpu::M68000), vec![Tok::Int(6)]);
        // Dotted identifiers (dot-leading, not digit-leading) are unaffected.
        assert_eq!(kinds("Seq.fetch", Cpu::M68000), vec![Tok::Ident("Seq.fetch".into())]);
        // A float inside a larger expression is still one token amid others.
        assert_eq!(
            kinds("6.283185307179586 * i / 64", Cpu::M68000),
            vec![
                Tok::Float(6.283185307179586),
                Tok::Punct(Punct::Star),
                Tok::Ident("i".into()),
                Tok::Punct(Punct::Slash),
                Tok::Int(64),
            ]
        );
    }
}
