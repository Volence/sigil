//! FSTRING encoder + `assert` auto-message derivation (diagnostics construct,
//! Task 1). Pure functions turning format strings and assert operand-spellings
//! into the exact inline-message byte run that vladikcomper's MD Debugger v2.6
//! (`aeon/engine/debug/debugger.asm`) emits, so every downstream stage
//! (parser, desugar, acceptance vectors) rests on byte-for-byte ground truth.
//!
//! Ground truth for the byte constants and tables:
//!   * `aeon/engine/debug/debugger.asm` lines 53-89 (argument format flags:
//!     `hex`/`dec`/`bin`/`sym`/`symdisp`/`str` and the width equates
//!     `byte`/`word`/`long`) and lines 96-107 (console control flags:
//!     `endl`/`cr`/`pal0..3` plain, `setw`/`setoff`/`setpat`/`setx`
//!     parametrized). (The task plan cited "85-130"; the equates actually
//!     span 53-107 in this file — the constants below are copied from those
//!     lines, not the plan's approximate range.)
//!   * `__FSTRING_GenerateDecodedString` (debugger.asm lines 712-779) — the
//!     descriptor byte is `val(param) | width_bits` where width_bits is
//!     0/1/3 for `.b`/`.w`/`.l`, and the run ends with a `$00` terminator.
//!     The macro's own `if (val(.__param) < $80) !error` check (line 746) is
//!     enforced here as an `Err`.
//!   * The auto-message TEMPLATE (§4.4 of the design spec) and the exit-flag
//!     parity rule (§4.5) are pinned by the hand-written transliterations in
//!     `aeon/engine/objects/rings.emp` (~lines 96-112) and
//!     `aeon/engine/objects/core.emp` (Debug_AssertObjLoop, ~lines 292-337),
//!     which are the byte vectors reproduced in the tests below.
//!
//! This module reads no state and touches no `Evaluator`; it is a
//! self-contained encoder, unit-tested directly (matching `s4lz.rs`'s
//! split between arg-shape checking and a pure core).
//!
//! Task 1 of the diagnostics-construct build ships this encoder as the
//! ground-truth foundation; the parser/desugar/lowering that CONSUME its
//! `pub` API land in later tasks. Until then the items are exercised only by
//! the `#[cfg(test)]` vectors below, so the module-level `allow(dead_code)`
//! keeps the tree warning-free without weakening the public surface.
#![allow(dead_code)]

/// Argument display width for an FSTRING `%<.b|.w|.l ...>` operand and for
/// `assert.<w>`. The `width_bits` (0/1/3) are OR'd into the param base to
/// form the descriptor byte (debugger.asm lines 87-89, 750-756).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    B,
    W,
    L,
}

impl Width {
    /// `byte`=0, `word`=1, `long`=3 (debugger.asm lines 87-89).
    pub fn bits(self) -> u8 {
        match self {
            Width::B => 0,
            Width::W => 1,
            Width::L => 3,
        }
    }

    /// The AS `.b`/`.w`/`.l` spelling used in the auto-message template.
    fn suffix(self) -> &'static str {
        match self {
            Width::B => "b",
            Width::W => "w",
            Width::L => "l",
        }
    }
}

/// Plain (no-argument) console control tokens: `%<name>` → one byte.
/// Copied from debugger.asm lines 96-101.
const PLAIN_CONTROL_TOKENS: &[(&str, u8)] = &[
    ("endl", 0xE0), // "End of line": line break
    ("cr", 0xE6),   // "Carriage return": jump to beginning of line
    ("pal0", 0xE8), // use palette line #0
    ("pal1", 0xEA), // use palette line #1
    ("pal2", 0xEC), // use palette line #2
    ("pal3", 0xEE), // use palette line #3
];

/// Parametrized console control tokens: `%<name N>` → control byte + one
/// param byte. Copied from debugger.asm lines 104-107.
const PARAM_CONTROL_TOKENS: &[(&str, u8)] = &[
    ("setw", 0xF0),   // set line width
    ("setoff", 0xF4), // set tile offset (low byte of base pattern)
    ("setpat", 0xF8), // set tile pattern (high byte of base pattern)
    ("setx", 0xFA),   // set x-position
];

/// Argument format param base bytes. Copied from debugger.asm lines 53-58.
const PARAM_BASES: &[(&str, u8)] = &[
    ("hex", 0x80),     // display as hexadecimal
    ("dec", 0x90),     // display as decimal
    ("bin", 0xA0),     // display as binary
    ("sym", 0xB0),     // display as symbol (offset → symbol+displacement)
    ("symdisp", 0xC0), // symbol displacement alone
    ("str", 0xD0),     // display as string (offset → inserted string)
];

/// Additional param flags OR'd onto a base (debugger.asm lines 75-82):
/// `signed`=8 (hex/dec/bin), `split`=8 / `forced`=4 (sym), `weak`=8 (symdisp).
const PARAM_FLAGS: &[(&str, u8)] = &[("signed", 8), ("split", 8), ("forced", 4), ("weak", 8)];

/// One `%<.b|.w|.l operand [param]>` argument recorded in string order, so the
/// downstream push-code generator can emit the stack pushes (in reverse token
/// order, per `__FSTRING_GenerateArgumentsCode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FStringArg {
    pub width: Width,
    pub operand_spelling: String,
    pub param: String,
}

/// The encoded inline-message byte run for a format string, plus the ordered
/// argument list. `bytes` includes the trailing `$00` terminator but NOT the
/// exit-flag byte (that is offset-parity-dependent — see [`exit_flag_bytes`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedFString {
    pub bytes: Vec<u8>,
    pub args: Vec<FStringArg>,
}

/// Resolve a param spelling (e.g. `hex`, `signed`, `sym+split`, `dec+signed`)
/// to its byte value, mirroring `__FSTRING_GenerateDecodedString`'s handling:
/// empty → `hex`, bare `signed` → `hex+signed`, then AS `val()` over a
/// `+`-joined flag expression. Enforces the macro's own `< $80` check.
fn resolve_param(param: &str) -> Result<u8, String> {
    let param = param.trim();
    // Mirror debugger.asm lines 740-744: default hex, `signed` → hex+signed.
    let normalized = if param.is_empty() {
        "hex".to_string()
    } else if param == "signed" {
        "hex+signed".to_string()
    } else {
        param.to_string()
    };

    let mut value: u8 = 0;
    for term in normalized.split('+') {
        let term = term.trim();
        let bits = if let Some((_, v)) = PARAM_BASES.iter().find(|(n, _)| *n == term) {
            *v
        } else if let Some((_, v)) = PARAM_FLAGS.iter().find(|(n, _)| *n == term) {
            *v
        } else {
            return Err(format!(
                "unknown FSTRING param `{term}` (expected hex, dec, bin, sym, symdisp, str, \
                 or a flag: signed, split, forced, weak)"
            ));
        };
        value |= bits;
    }

    // The macro's own guard: `if (val(.__param) < $80) !error` (line 746).
    if value < 0x80 {
        return Err(format!(
            "illegal FSTRING param `{param}`: resolved byte ${value:02X} is < $80 \
             (expected hex, dec, bin, sym, str or their derivatives)"
        ));
    }
    Ok(value)
}

/// Encode a format string into its inline-message byte run.
///
/// Tokens:
///   * literal text → its ASCII bytes;
///   * `%<endl|cr|pal0..3>` → one control byte;
///   * `%<setw N | setoff N | setpat N | setx N>` → control byte + one param
///     byte (the numeric argument, low 8 bits);
///   * `%<.b|.w|.l operand [param]>` → one descriptor byte (`param_base |
///     width_bits`), with the argument recorded in [`EncodedFString::args`];
///     `param` defaults to `hex`.
///
/// A trailing `$00` terminator is always appended.
pub fn encode_fstring(s: &str) -> Result<EncodedFString, String> {
    let mut bytes = Vec::new();
    let mut args = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Token opener `%<` ... `>`.
        if chars[i] == '%' && i + 1 < chars.len() && chars[i + 1] == '<' {
            let start = i + 2;
            let Some(rel_end) = chars[start..].iter().position(|&c| c == '>') else {
                return Err(format!("unterminated `%<` token in FSTRING: `{s}`"));
            };
            let end = start + rel_end;
            let inner: String = chars[start..end].iter().collect();
            encode_token(&inner, &mut bytes, &mut args)?;
            i = end + 1;
            continue;
        }
        // Literal byte. FSTRING text is raw ASCII (matches the macro's
        // `dc.b substr(...)` verbatim copy).
        let c = chars[i];
        if !c.is_ascii() {
            return Err(format!("non-ASCII character `{c}` in FSTRING literal text"));
        }
        bytes.push(c as u8);
        i += 1;
    }
    bytes.push(0x00); // string terminator (debugger.asm line 777)
    Ok(EncodedFString { bytes, args })
}

/// Encode a single `%<...>` token's inner text.
fn encode_token(inner: &str, bytes: &mut Vec<u8>, args: &mut Vec<FStringArg>) -> Result<(), String> {
    let inner = inner.trim();

    // Argument token: `.b|.w|.l operand [param]`.
    if let Some(rest) = inner.strip_prefix('.') {
        let (width, rest) = if let Some(r) = rest.strip_prefix('b') {
            (Width::B, r)
        } else if let Some(r) = rest.strip_prefix('w') {
            (Width::W, r)
        } else if let Some(r) = rest.strip_prefix('l') {
            (Width::L, r)
        } else {
            return Err(format!("unknown FSTRING argument type `.{rest}` (expected .b/.w/.l)"));
        };
        // rest = " operand [param]" — first whitespace-run separates operand
        // from param (matching the macro's split on the first space after the
        // type, lines 667-678).
        let rest = rest.trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let operand = parts.next().unwrap_or("").trim().to_string();
        if operand.is_empty() {
            return Err(format!("FSTRING argument token `%<.{}...>` has no operand", width.suffix()));
        }
        let param_spelling = parts.next().unwrap_or("").trim().to_string();
        let param_byte = resolve_param(&param_spelling)?;
        // Descriptor byte = param base | width bits (lines 750-756).
        bytes.push(param_byte | width.bits());
        // Record the canonical param spelling (empty → hex) for downstream.
        let param = if param_spelling.is_empty() { "hex".to_string() } else { param_spelling };
        args.push(FStringArg { width, operand_spelling: operand, param });
        return Ok(());
    }

    // Plain control token.
    if let Some((_, b)) = PLAIN_CONTROL_TOKENS.iter().find(|(n, _)| *n == inner) {
        bytes.push(*b);
        return Ok(());
    }

    // Parametrized control token: `name N`.
    let mut parts = inner.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").trim();
    if let Some((_, b)) = PARAM_CONTROL_TOKENS.iter().find(|(n, _)| *n == name) {
        let arg = parts.next().map(str::trim).unwrap_or("");
        if arg.is_empty() {
            return Err(format!("FSTRING control token `%<{name}>` requires a numeric argument"));
        }
        let n = parse_num(arg)
            .ok_or_else(|| format!("FSTRING control token `%<{name} {arg}>`: `{arg}` is not a number"))?;
        bytes.push(*b);
        bytes.push((n & 0xFF) as u8);
        return Ok(());
    }

    Err(format!(
        "unknown FSTRING token `%<{inner}>` (expected endl, cr, pal0..3, setw/setoff/setpat/setx N, \
         or .b/.w/.l operand [param])"
    ))
}

/// Parse a small numeric literal in AS spellings a control param may use:
/// `$hex`, `0xhex`, `%bin`, or decimal.
fn parse_num(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('$') {
        u64::from_str_radix(h, 16).ok()
    } else if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()
    } else if let Some(b) = s.strip_prefix('%') {
        u64::from_str_radix(b, 2).ok()
    } else {
        s.parse().ok()
    }
}

/// Build the `assert` auto-message byte run (§4.4), from source operand
/// spellings. Byte-for-byte the AS macro's template (debugger.asm lines
/// 222-224):
///
/// ```text
/// "Assertion failed:" $E0 $EC "> assert.<w> " $E8 "<src>," $EC "<cond>"
///     [$E8 ",<dest>"] $E0 $EA "Got: " <descriptor> $00
/// ```
///
/// `cond` is lowercased; operand spellings are verbatim. In the tst form
/// (`dest == None`) the `$E8 ",<dest>"` segment is omitted. The run includes
/// the `%<.<w> src>` descriptor byte (`hex | width_bits`) and the `$00`
/// terminator, but NOT the exit-flag byte (see [`exit_flag_bytes`]).
pub fn assert_message(w: Width, src: &str, cond: &str, dest: Option<&str>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"Assertion failed:");
    out.push(0xE0); // %<endl>
    out.push(0xEC); // %<pal2>
    out.extend_from_slice(b"> assert.");
    out.extend_from_slice(w.suffix().as_bytes());
    out.push(b' ');
    out.push(0xE8); // %<pal0>
    out.extend_from_slice(src.as_bytes());
    out.push(b',');
    out.push(0xEC); // %<pal2>
    out.extend_from_slice(cond.to_ascii_lowercase().as_bytes());
    if let Some(dest) = dest {
        out.push(0xE8); // %<pal0>
        out.push(b',');
        out.extend_from_slice(dest.as_bytes());
    }
    out.push(0xE0); // %<endl>
    out.push(0xEA); // %<pal1>
    out.extend_from_slice(b"Got: ");
    // %<.<w> src> descriptor: hex ($80) | width_bits (line 750-756).
    out.push(0x80 | w.bits());
    out.push(0x00); // terminator
    out
}

/// The error-handler exit-flag byte(s) following the message run (§4.5).
///
/// `_eh_return` is `$20` (debugger.asm line 120). If the flag would land at an
/// ODD offset, the macro OR's `_eh_align_offset` (`$80`, line 122) onto it and
/// emits one `$00` pad so the following `jmp` is word-aligned (`!align 2`);
/// at an EVEN offset it emits the bare `$20`.
///
/// (rings: odd → `$A0, $00`; core: even → `$20`, no pad — both reproduced.)
pub fn exit_flag_bytes(odd_offset: bool) -> Vec<u8> {
    if odd_offset {
        vec![0x20 | 0x80, 0x00]
    } else {
        vec![0x20]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_tokens_encode() {
        assert_eq!(
            encode_fstring("A%<endl>B%<pal2>C").unwrap().bytes,
            vec![b'A', 0xE0, b'B', 0xEC, b'C', 0x00]
        );
    }

    #[test]
    fn rings_assert_message_vector() {
        let m = assert_message(Width::B, "d4", "eq", Some("#0"));
        let mut expect = Vec::new();
        expect.extend(b"Assertion failed:");
        expect.push(0xE0);
        expect.push(0xEC);
        expect.extend(b"> assert.b ");
        expect.push(0xE8);
        expect.extend(b"d4,");
        expect.push(0xEC);
        expect.extend(b"eq");
        expect.push(0xE8);
        expect.extend(b",#0");
        expect.push(0xE0);
        expect.push(0xEA);
        expect.extend(b"Got: ");
        expect.push(0x80); // %<.b src> descriptor: hex|width_bits(b=0)
        expect.push(0x00); // terminator
        assert_eq!(m, expect);
    }

    #[test]
    fn core_long_message_vector() {
        let m = assert_message(Width::L, "a0", "hs", Some("#Object_RAM"));
        assert_eq!(*m.last().unwrap(), 0x00);
        assert_eq!(m[m.len() - 2], 0x83); // hex($80) | long(3)
    }

    #[test]
    fn exit_flag_parity_both_ways() {
        // rings: flag at ODD offset -> $20|$80 + $00 pad; core: EVEN -> bare $20
        assert_eq!(exit_flag_bytes(/*odd_offset=*/ true), vec![0xA0, 0x00]);
        assert_eq!(exit_flag_bytes(/*odd_offset=*/ false), vec![0x20]);
    }

    #[test]
    fn tst_form_message_omits_dest() {
        let m = assert_message(Width::W, "d1", "eq", None);
        let s = m.windows(2).position(|w| w == [0xE8, b',']);
        assert!(s.is_none());
    }

    // --- additional coverage beyond the mandated Step-1 vectors ---

    #[test]
    fn rings_full_message_matches_transliteration() {
        // rings.emp line 109 dc.b run + descriptor + terminator (line 110).
        let m = assert_message(Width::B, "d4", "eq", Some("#0"));
        let mut expect = Vec::new();
        expect.extend(b"Assertion failed:");
        expect.extend([0xE0, 0xEC]);
        expect.extend(b"> assert.b ");
        expect.push(0xE8);
        expect.extend(b"d4,");
        expect.push(0xEC);
        expect.extend(b"eq");
        expect.push(0xE8);
        expect.extend(b",#0");
        expect.extend([0xE0, 0xEA]);
        expect.extend(b"Got: ");
        expect.extend([0x80, 0x00]);
        assert_eq!(m, expect);
    }

    #[test]
    fn core_word_message_matches_transliteration() {
        // core.emp line 333: assert.w d7, lo, #NUM_DYNAMIC ; descriptor $81.
        let m = assert_message(Width::W, "d7", "lo", Some("#NUM_DYNAMIC"));
        let mut expect = Vec::new();
        expect.extend(b"Assertion failed:");
        expect.extend([0xE0, 0xEC]);
        expect.extend(b"> assert.w ");
        expect.push(0xE8);
        expect.extend(b"d7,");
        expect.push(0xEC);
        expect.extend(b"lo");
        expect.push(0xE8);
        expect.extend(b",#NUM_DYNAMIC");
        expect.extend([0xE0, 0xEA]);
        expect.extend(b"Got: ");
        expect.extend([0x81, 0x00]); // hex|word
        assert_eq!(m, expect);
    }

    #[test]
    fn cond_is_lowercased() {
        let upper = assert_message(Width::L, "a0", "HS", Some("#Object_RAM"));
        let lower = assert_message(Width::L, "a0", "hs", Some("#Object_RAM"));
        assert_eq!(upper, lower);
    }

    #[test]
    fn fstring_records_arg_and_descriptor() {
        let e = encode_fstring("Got: %<.b d4>").unwrap();
        // "Got: " then descriptor $80 then terminator.
        assert_eq!(e.bytes, {
            let mut v = b"Got: ".to_vec();
            v.push(0x80);
            v.push(0x00);
            v
        });
        assert_eq!(e.args.len(), 1);
        assert_eq!(e.args[0], FStringArg { width: Width::B, operand_spelling: "d4".into(), param: "hex".into() });
    }

    #[test]
    fn fstring_arg_widths_descriptors() {
        assert_eq!(encode_fstring("%<.b d0>").unwrap().bytes, vec![0x80, 0x00]);
        assert_eq!(encode_fstring("%<.w d0>").unwrap().bytes, vec![0x81, 0x00]);
        assert_eq!(encode_fstring("%<.l a0>").unwrap().bytes, vec![0x83, 0x00]);
    }

    #[test]
    fn fstring_arg_param_variants() {
        // dec = $90, dec|width_bits; sym = $B0; hex+signed = $88.
        assert_eq!(encode_fstring("%<.w d0 dec>").unwrap().bytes, vec![0x91, 0x00]);
        assert_eq!(encode_fstring("%<.l a0 sym>").unwrap().bytes, vec![0xB3, 0x00]);
        assert_eq!(encode_fstring("%<.b d0 signed>").unwrap().bytes, vec![0x88, 0x00]);
        assert_eq!(encode_fstring("%<.l a0 sym+split>").unwrap().bytes, vec![0xBB, 0x00]);
    }

    #[test]
    fn ordered_args_multiple() {
        let e = encode_fstring("x=%<.w d0> y=%<.l a1 sym>").unwrap();
        assert_eq!(e.args.len(), 2);
        assert_eq!(e.args[0].operand_spelling, "d0");
        assert_eq!(e.args[1].operand_spelling, "a1");
        assert_eq!(e.args[1].param, "sym");
    }

    #[test]
    fn parametrized_control_token() {
        // setw ($F0) N ; N low byte follows.
        assert_eq!(encode_fstring("%<setw 40>").unwrap().bytes, vec![0xF0, 40, 0x00]);
        assert_eq!(encode_fstring("%<setx $10>").unwrap().bytes, vec![0xFA, 0x10, 0x00]);
    }

    #[test]
    fn all_plain_control_tokens() {
        assert_eq!(encode_fstring("%<endl>").unwrap().bytes, vec![0xE0, 0x00]);
        assert_eq!(encode_fstring("%<cr>").unwrap().bytes, vec![0xE6, 0x00]);
        assert_eq!(encode_fstring("%<pal0>").unwrap().bytes, vec![0xE8, 0x00]);
        assert_eq!(encode_fstring("%<pal1>").unwrap().bytes, vec![0xEA, 0x00]);
        assert_eq!(encode_fstring("%<pal2>").unwrap().bytes, vec![0xEC, 0x00]);
        assert_eq!(encode_fstring("%<pal3>").unwrap().bytes, vec![0xEE, 0x00]);
    }

    #[test]
    fn empty_fstring_is_just_terminator() {
        assert_eq!(encode_fstring("").unwrap().bytes, vec![0x00]);
    }

    #[test]
    fn unknown_token_errors() {
        assert!(encode_fstring("%<bogus>").is_err());
    }

    #[test]
    fn unterminated_token_errors() {
        assert!(encode_fstring("%<endl").is_err());
    }

    #[test]
    fn param_below_0x80_rejected() {
        // A bare flag (`signed`=8) alone would resolve < $80 without a base —
        // but `signed` is normalized to hex+signed, so probe a raw sub-$80
        // spelling via a flag-only expression that skips the normalization.
        assert!(resolve_param("forced").is_err());
        assert!(resolve_param("split").is_err());
    }

    #[test]
    fn signed_normalizes_to_hex_signed() {
        assert_eq!(resolve_param("signed").unwrap(), 0x88);
    }

    #[test]
    fn raise_error_style_message() {
        // path_swap.asm shape: "Bad path swap!%<endl>Got: %<.b d0>"
        let e = encode_fstring("Bad path swap!%<endl>Got: %<.b d0>").unwrap();
        let mut expect = b"Bad path swap!".to_vec();
        expect.push(0xE0);
        expect.extend(b"Got: ");
        expect.push(0x80);
        expect.push(0x00);
        assert_eq!(e.bytes, expect);
        assert_eq!(e.args.len(), 1);
    }
}
