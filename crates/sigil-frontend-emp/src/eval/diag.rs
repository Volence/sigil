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
//! `pub` API land in later tasks. Until then those public items are exercised
//! only by the `#[cfg(test)]` vectors below, so each carries a per-item
//! `#[allow(dead_code)]`. The attribute is deliberately NOT module-scoped:
//! private helpers (`encode_token`/`resolve_param`/`parse_num`) stay
//! dead-code-checked, so a genuinely-orphaned helper still warns once the
//! encoder is wired in.

/// Argument display width for an FSTRING `%<.b|.w|.l ...>` operand and for
/// `assert.<w>`. The `width_bits` (0/1/3) are OR'd into the param base to
/// form the descriptor byte (debugger.asm lines 87-89, 750-756).
#[allow(dead_code)] // consumed by the parser/desugar in later diag tasks
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
#[allow(dead_code)] // consumed by the push-code generator in later diag tasks
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FStringArg {
    pub width: Width,
    pub operand_spelling: String,
    /// The canonical param spelling (empty input normalizes to `"hex"`).
    /// This is the SPELLING for diagnostics/round-tripping; a consumer that
    /// needs the descriptor byte re-resolves it through the param table
    /// (same path `encode_fstring` uses), it is not stored pre-resolved here.
    pub param: String,
}

/// The encoded inline-message byte run for a format string, plus the ordered
/// argument list. `bytes` includes the trailing `$00` terminator but NOT the
/// exit-flag byte (that is offset-parity-dependent — see [`exit_flag_bytes`]).
#[allow(dead_code)] // returned to the lowering stage in later diag tasks
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
#[allow(dead_code)] // driven by `raise_error` lowering in later diag tasks
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
        // The control param is a single byte — refuse to silently truncate
        // (house rule: loud over silent). Names the token and the value.
        if n > 0xFF {
            return Err(format!(
                "FSTRING control token `%<{name} {arg}>`: value {n} does not fit one byte (0..=255)"
            ));
        }
        bytes.push(*b);
        bytes.push(n as u8);
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
#[allow(dead_code)] // driven by `assert` lowering in later diag tasks
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
/// `_eh_return` is `$20` (debugger.asm line 120). The macro's own align rule
/// (`.__align_flag: set ((((*)&1)!1)*_eh_align_offset)`, debugger.asm line 264):
/// `(*)` is the flag byte's OWN address and AS's logical-`!` negates its low
/// bit, so the `_eh_align_offset` bit (`$80`, line 122) is OR'd in — and one
/// `$00` pad emitted so the following `jmp` is word-aligned (`!align 2`) —
/// exactly when the flag byte sits at an EVEN address; at an ODD address it
/// emits the bare `$20`.
///
/// `pad` = "the flag byte lands at an even offset → emit the aligned form".
/// (rings: message len 50, flag at even offset → `$A0, $00`; core `.l`: flag at
/// odd offset → bare `$20`, no pad — both reproduced by the byte-equality tests.)
#[allow(dead_code)] // emitted by the lowering stage in later diag tasks
pub fn exit_flag_bytes(pad: bool) -> Vec<u8> {
    if pad {
        vec![0x20 | 0x80, 0x00]
    } else {
        vec![0x20]
    }
}

// ---------------------------------------------------------------------------
// Task 3 — the desugar (§4.2 / §4.3): turn an `assert`/`raise_error` AST into
// the exact twin-parity `Vec<AsmStmt>` the enclosing proc body then lowers via
// `lower_asm_stmt` (the SAME path a comptime-`if`'s chosen branch takes). The
// synthesized statements are built directly as AST (the `lower/script.rs`
// pattern), not re-parsed from text, so no operand-spelling round-trip can
// diverge from the parser's shapes.
//
// Hygiene: `.skip` / `.raise` are minted as fully-unique LITERAL symbols
// (`$diag{n}$skip`) — `$`-wrapped like the hygiene module's own hidden locals,
// so they are unspellable from source and two asserts in one proc never
// collide. The symbol is used verbatim in BOTH the `Label` def and the branch/
// `pea` reference; the enclosing body's `LabelScope` leaves an unknown name
// untouched (passthrough), so def and reference land on the same string and the
// intra-expansion branch resolves. They are NEVER routed through scope
// mangling (which would rewrite one side and not the other).

use crate::ast::{self, AsmStmt, Expr, InstrLine, Operand, TextOrSplice};
use sigil_span::Span;

/// One byte of assembled inline data as a `dc.b` element operand.
///
/// STEP-8 decision (one-Imm-per-byte, NOT mixed string+ints): `lower_dc`
/// evaluates each `dc` operand independently to a `Value::Int` (any width) or a
/// `Value::Str` (dc.b only). The message/flag bytes arrive here as a flat
/// `Vec<u8>` from the encoder, so the uniform, unambiguous representation is one
/// `Operand::Plain { Expr::Int(byte) }` per byte — it needs no string-literal
/// round-trip, and a byte value never risks re-lexing as a control token. A
/// mixed `dc.b "Assertion failed:", $E0, …` form would require reconstructing
/// string spans the encoder has already flattened; per-byte ints are strictly
/// simpler and byte-identical.
fn dc_byte_op(b: u8, span: Span) -> Operand {
    Operand::Plain { expr: Expr::Int(b as i64, span), size: None, span }
}

/// A single-segment path operand (`sr`, `$diag0$skip`, …) as a plain operand.
fn path_op(seg: &str, span: Span) -> Operand {
    Operand::Plain {
        expr: Expr::Path(ast::Path { segments: vec![seg.to_string()], span }),
        size: None,
        span,
    }
}

/// `(sp)` — the bare stack-pointer register indirect, shared base for the
/// pre-decrement / post-increment / displacement stack forms below.
fn sp_ind(span: Span) -> Operand {
    Operand::Ind {
        parts: vec![(Expr::Path(ast::Path { segments: vec!["sp".into()], span }), None)],
        size: None,
        span,
    }
}

/// `-(sp)` — pre-decrement on the stack pointer.
fn predec_sp(span: Span) -> Operand {
    Operand::PreDec(Box::new(sp_ind(span)))
}

/// `(sp)+` — post-increment on the stack pointer (the CCR restore's source).
fn postinc_sp(span: Span) -> Operand {
    Operand::PostInc(Box::new(sp_ind(span)))
}

/// `1(sp)` — displacement-indirect on the stack pointer (the `.b` arg slot).
fn disp1_sp(span: Span) -> Operand {
    Operand::DispInd {
        disp: Expr::Int(1, span),
        inner: Box::new(sp_ind(span)),
        disp_spliced: false,
        field_size_override: None,
        span,
    }
}

/// `label(pc)` — a PC-relative operand naming a (minted, unique) label symbol.
fn pcrel(label: &str, span: Span) -> Operand {
    Operand::DispInd {
        disp: Expr::Path(ast::Path { segments: vec![label.to_string()], span }),
        inner: Box::new(Operand::Ind {
            parts: vec![(Expr::Path(ast::Path { segments: vec!["pc".into()], span }), None)],
            size: None,
            span,
        }),
        disp_spliced: false,
        field_size_override: None,
        span,
    }
}

/// `(Sym).l` — an explicit-`.l` absolute indirect naming a link symbol (the
/// two `MDDBG__*` handler entry points). Matches the transliterations'
/// `jsr (MDDBG__ErrorHandler).l` / `jmp (…PagesController).l` byte-for-byte.
fn abs_l(sym: &str, span: Span) -> Operand {
    Operand::Ind {
        parts: vec![(Expr::Path(ast::Path { segments: vec![sym.to_string()], span }), None)],
        size: Some(TextOrSplice::Text("l".into())),
        span,
    }
}

/// A synthesized instruction line.
fn instr(mnemonic: &str, size: Option<&str>, operands: Vec<Operand>, span: Span) -> AsmStmt {
    AsmStmt::Instr(InstrLine {
        mnemonic: vec![TextOrSplice::Text(mnemonic.into())],
        size: size.map(|s| TextOrSplice::Text(s.into())),
        operands,
        span,
        dispatch_bound: None,
    })
}

/// A synthesized `dc.b` line over `bytes` (one int cell per byte, §step-8).
fn dc_b(bytes: &[u8], span: Span) -> AsmStmt {
    AsmStmt::Instr(InstrLine {
        mnemonic: vec![TextOrSplice::Text("dc".into())],
        size: Some(TextOrSplice::Text("b".into())),
        operands: bytes.iter().map(|&b| dc_byte_op(b, span)).collect(),
        span,
        dispatch_bound: None,
    })
}

/// A non-`export` label definition with a minted, unique symbol name.
fn label(name: &str, span: Span) -> AsmStmt {
    AsmStmt::Label { name: name.to_string(), export: false, span }
}

/// The error-handler entry points (this engine's debugger config, §7.5).
const HANDLER: &str = "MDDBG__ErrorHandler";
const PAGES: &str = "MDDBG__ErrorHandler_PagesController";

/// The RRAISE tail (spec §4.2 steps 4-10 / §4.3): `pea self(pc)`, SR push, the
/// argument pushes, `jsr (handler).l`, the inline message + exit-flag data, and
/// `jmp (pages).l`. Shared by `assert` (auto-message) and `raise_error` (user
/// fstring). `raise_label` is the minted self-address label; `message` is the
/// full encoded run INCLUDING its `$00` terminator (from `assert_message` or
/// `encode_fstring`); `arg_pushes` are the already-ordered stack pushes.
///
/// The parity insight (§4.5): every 68k instruction preceding the data is
/// word-sized (even), so the exit-flag's offset parity from the expansion start
/// equals `message.len() % 2`. We compute `odd_offset` from that and let
/// `exit_flag_bytes` add the `$80` align bit + `$00` pad when the flag would
/// land odd. (Asserted in `debug_assert!` below + a unit test, so a future
/// odd-length synthesized instruction trips the identity.)
fn raise_tail(raise_label: &str, message: &[u8], arg_pushes: Vec<AsmStmt>, span: Span) -> Vec<AsmStmt> {
    let mut out = Vec::new();
    // 4. pea self(pc) — the handler reads the message at the jsr return addr.
    out.push(label(raise_label, span));
    out.push(instr("pea", None, vec![pcrel(raise_label, span)], span));
    // 5. move.w sr, -(sp) — SR for the handler display.
    out.push(instr("move", Some("w"), vec![path_op("sr", span), predec_sp(span)], span));
    // 6. argument pushes (already built + ordered by the caller).
    out.extend(arg_pushes);
    // 7. jsr (handler).l
    out.push(instr("jsr", None, vec![abs_l(HANDLER, span)], span));
    // 8. inline auto/user message (incl. its own $00 terminator).
    out.push(dc_b(message, span));
    // 9. exit-flag byte(s), align-padded per the debugger.asm parity rule (§4.5).
    //
    // The macro's rule (debugger.asm line 264):
    //   `.__align_flag: set ((((*)&1)!1)*_eh_align_offset)`
    // `(*)` is the flag byte's OWN address; `(*)&1` is 1 at an odd address, and
    // AS's logical-`!` negates it — so the align bit ($80) is OR'd in (and a
    // `$00` pad emitted) exactly when the flag byte sits at an EVEN address, so
    // the handler skips the byte to reach the word-aligned `jmp`.
    //
    // The deterministic insight: every statement emitted BEFORE the message
    // (steps 1-7 for assert, 4-7 for raise_error) is a 68k instruction, and
    // every 68k instruction is word-sized (even bytes). So the message run
    // STARTS at an even offset, and the flag byte's offset parity equals
    // `message.len() % 2`: an EVEN-length message → flag at an even offset →
    // pad; an odd-length message → flag at an odd offset → bare `$20`.
    // (rings: message len 50, even → `$A0,$00`; core `.l`: odd → `$20`. Proven
    // against a real lowering by the byte-equality tests; a future odd-length
    // synthesized instruction before the message would shift this and trip them.)
    let flag_at_even_offset = message.len().is_multiple_of(2);
    // `exit_flag_bytes`'s `pad` param IS "emit the aligned/padded form" — pass
    // the even-offset case (the align bit + `$00` pad).
    out.push(dc_b(&exit_flag_bytes(flag_at_even_offset), span));
    // 10. jmp (pages).l
    out.push(instr("jmp", None, vec![abs_l(PAGES, span)], span));
    out
}

/// The `.b`/`.w`/`.l` argument-push statements for pushing `src` for the
/// handler to read: `.b` → `subq.w #2,sp` + `move.b src,1(sp)` (2-byte slot,
/// low byte written); `.w`/`.l` → `move.<w> src,-(sp)`. Shared by BOTH call
/// contexts — `assert`'s single auto-message arg (§4.2 step 6) and each
/// `raise_error` FSTRING `%<...>` token's arg (§4.3, pushed in reverse token
/// order by the caller). The operand class is validated upstream (a register
/// for assert; register-or-immediate for raise_error, via
/// [`fstring_arg_operand`]).
#[allow(dead_code)] // driven by the diag arms in eval/asm.rs
pub fn arg_push(w: Width, src: Operand, span: Span) -> Vec<AsmStmt> {
    match w {
        Width::B => vec![
            instr("subq", Some("w"), vec![Operand::Imm(Expr::Int(2, span)), path_op("sp", span)], span),
            instr("move", Some("b"), vec![src, disp1_sp(span)], span),
        ],
        Width::W => vec![instr("move", Some("w"), vec![src, predec_sp(span)], span)],
        Width::L => vec![instr("move", Some("l"), vec![src, predec_sp(span)], span)],
    }
}

/// The compare mnemonic for the `assert` cmp form — always `cmp` (§4.2 #2),
/// paralleling the [`HANDLER`]/[`PAGES`] handler-entry consts above.
const CMP: &str = "cmp";

/// The parsed pieces of one `assert` site, carried together into the desugar.
/// Mirrors the AST's own pairing (`AsmStmt::Assert` keeps `dest` as an
/// `Option<(Operand, spelling)>`), so the operand and its verbatim source
/// spelling never drift apart. `dest = Some` is the cmp form (`cmp.<w> dest,
/// src`), `None` the tst form (`tst.<w> src`).
#[allow(dead_code)] // constructed by the `Assert` arm in eval/asm.rs
#[derive(Clone, Debug)]
pub struct AssertParts {
    /// The operation width (`.b`/`.w`/`.l`).
    pub width: Width,
    /// The compared/tested source operand (a register, §5).
    pub src: Operand,
    /// `src`'s verbatim source spelling, for the auto-message.
    pub src_spelling: String,
    /// The condition code (one of the 16 Bcc codes, lowercase).
    pub cond: String,
    /// The compare destination + its verbatim spelling (`cmp` form); `None` is
    /// the `tst` form.
    pub dest: Option<(Operand, String)>,
}

/// Build the full `assert` DEBUG-shape expansion (§4.2, 11 steps IN ORDER).
///
/// `n` is a fresh instantiation id (from the evaluator's counter) that makes the
/// `.skip`/`.raise` symbols unique — two asserts in one proc get distinct
/// `$diag{n}$…` labels. `p` carries the ALREADY-PARSED operands (cloned from the
/// AST) so their exact addressing shape rides through unchanged; `p.src` is also
/// pushed for the handler to display. The message bytes come from
/// [`assert_message`] over the source spellings.
#[allow(dead_code)] // driven by the `Assert` arm in eval/asm.rs
pub fn build_assert_expansion(n: u32, p: &AssertParts, span: Span) -> Vec<AsmStmt> {
    let skip = format!("$diag{n}$skip");
    let raise = format!("$diag{n}$raise");
    let wsfx = p.width.suffix();
    let mut out = Vec::new();

    // 1. move.w sr, -(sp) — CCR save.
    out.push(instr("move", Some("w"), vec![path_op("sr", span), predec_sp(span)], span));

    // 2. cmp.<w> dest, src  (cmp form)  |  tst.<w> src  (tst form).
    match &p.dest {
        Some((dest, _)) => {
            out.push(instr(CMP, Some(wsfx), vec![dest.clone(), p.src.clone()], span));
        }
        None => {
            out.push(instr("tst", Some(wsfx), vec![p.src.clone()], span));
        }
    }

    // 3. b<cond>.w .skip — PINNED .w (generator-owned structural width, §4.2 #3).
    out.push(instr(&format!("b{}", p.cond), Some("w"), vec![path_op(&skip, span)], span));

    // 4-10. the RaiseError tail (auto-message).
    let dest_spelling = p.dest.as_ref().map(|(_, s)| s.as_str());
    let message = assert_message(p.width, &p.src_spelling, &p.cond, dest_spelling);
    let arg_pushes = arg_push(p.width, p.src.clone(), span);
    out.extend(raise_tail(&raise, &message, arg_pushes, span));

    // 11. .skip: then move.w (sp)+, sr — CCR restore.
    out.push(label(&skip, span));
    out.push(instr("move", Some("w"), vec![postinc_sp(span), path_op("sr", span)], span));
    out
}

/// Build the `raise_error` expansion (§4.3): steps 4-10 only — NO DEBUG gate,
/// NO cmp/branch/CCR-compare wrapper. `n` mints the unique `.raise` self-label;
/// `message` is the encoded user fstring (incl. `$00`); `arg_pushes` are the
/// per-token pushes the caller already built in REVERSE token order
/// (matching `__FSTRING_GenerateArgumentsCode`).
#[allow(dead_code)] // driven by the `RaiseError` arm in eval/asm.rs
pub fn build_raise_error_expansion(
    n: u32,
    message: &[u8],
    arg_pushes: Vec<AsmStmt>,
    span: Span,
) -> Vec<AsmStmt> {
    let raise = format!("$diag{n}$raise");
    raise_tail(&raise, message, arg_pushes, span)
}


/// Build the operand for an FSTRING argument spelling (`d0`, `#$8000`, …),
/// enforcing the §5 register-or-immediate limit. A leading `#` → an immediate
/// whose expr is parsed from the remainder; a register name → a plain register
/// operand; anything else → `None` (the caller emits the steering error).
#[allow(dead_code)] // driven by the `RaiseError` arm in eval/asm.rs
pub fn fstring_arg_operand(spelling: &str, span: Span) -> Option<Operand> {
    let s = spelling.trim();
    if let Some(imm) = s.strip_prefix('#') {
        // A bare label / small numeric immediate. Parse the common forms the
        // FSTRING census uses: decimal/`$hex`/`0xhex`/`%bin`, else a bare
        // symbol path. (Full expression immediates are a recorded extension;
        // the corpus's raise_error args are registers or `#literal`.)
        let expr = parse_imm_expr(imm.trim(), span)?;
        return Some(Operand::Imm(expr));
    }
    // A register name (dn/an, sp alias) — the common raise_error arg.
    if is_register_name(s) {
        return Some(path_op(s, span));
    }
    None
}

/// Parse a simple immediate remainder (after `#`) into an [`Expr`]: a numeric
/// literal or a bare symbol. Returns `None` for anything with operators/dots
/// (unbuilt for v1 raise_error args — ledger the demand).
fn parse_imm_expr(s: &str, span: Span) -> Option<Expr> {
    if let Some(n) = parse_num(s) {
        return i64::try_from(n).ok().map(|v| Expr::Int(v, span));
    }
    // A bare symbol name (single identifier segment, no operators/dots).
    if !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.chars().next().unwrap().is_ascii_digit()
    {
        return Some(Expr::Path(ast::Path { segments: vec![s.to_string()], span }));
    }
    None
}

/// Whether `s` names a data/address register (`d0`..`d7`, `a0`..`a7`, `sp`).
fn is_register_name(s: &str) -> bool {
    matches!(
        s,
        "d0" | "d1" | "d2" | "d3" | "d4" | "d5" | "d6" | "d7"
            | "a0" | "a1" | "a2" | "a3" | "a4" | "a5" | "a6" | "a7" | "sp"
    )
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
        // pad=true (flag at EVEN offset) -> $20|$80 + $00 pad (rings shape);
        // pad=false (flag at ODD offset) -> bare $20 (core `.l` shape).
        assert_eq!(exit_flag_bytes(/*pad=*/ true), vec![0xA0, 0x00]);
        assert_eq!(exit_flag_bytes(/*pad=*/ false), vec![0x20]);
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
    fn control_param_out_of_range_errors() {
        // Loud over silent: >255 must error, not truncate to the low byte.
        let e = encode_fstring("%<setw 400>");
        assert!(e.is_err());
        let msg = e.unwrap_err();
        assert!(msg.contains("setw") && msg.contains("400"), "names token+value: {msg}");
        // Boundary: 255 is the largest one-byte value and must succeed.
        assert_eq!(encode_fstring("%<setw 255>").unwrap().bytes, vec![0xF0, 0xFF, 0x00]);
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
    fn encode_fstring_ends_at_terminator_no_exit_flag() {
        // The run must end at the $00 string terminator with NO exit-flag byte
        // ($20/$A0) after it — the exit flag is offset-parity-dependent and is
        // appended by the lowering stage, not the encoder. This pins the seam
        // later tasks concatenate across (assert_message is locked separately
        // by core_long_message_vector).
        for s in ["", "plain text", "Got: %<.l a0 sym>", "a%<endl>b%<setw 20>c"] {
            let bytes = encode_fstring(s).unwrap().bytes;
            // Exactly one $00 and it is the final byte: nothing (no exit flag)
            // follows the terminator.
            assert_eq!(*bytes.last().unwrap(), 0x00, "must end at terminator: {s:?}");
            assert_eq!(
                bytes.iter().filter(|&&b| b == 0x00).count(),
                1,
                "exactly one terminator, none embedded: {s:?}"
            );
        }
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

    // --- Task 3: the desugar builder (structural / parity unit vectors) -------

    /// A throwaway span for structurally-built AST (byte tests use real spans).
    fn sp() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    fn reg_op(name: &str) -> Operand {
        path_op(name, sp())
    }

    /// Build [`AssertParts`] for a cmp-form (`dest = Some`) or tst-form (`None`)
    /// assert over register `src`, for the structural unit tests.
    fn parts(w: Width, src: &str, cond: &str, dest: Option<&str>) -> AssertParts {
        AssertParts {
            width: w,
            src: reg_op(src),
            src_spelling: src.to_string(),
            cond: cond.to_string(),
            dest: dest.map(|d| (Operand::Imm(Expr::Int(0, sp())), d.to_string())),
        }
    }

    /// The parity IDENTITY: `exit_flag_bytes` is fed `message.len() % 2 == 0`,
    /// and — because every synthesized statement before the message is a
    /// word-sized 68k instruction — that equals "the flag byte lands at an even
    /// offset". This test pins the derivation directly: an EVEN-length message
    /// yields the padded `$A0,$00` flag, an ODD-length message the bare `$20`.
    /// A future non-even synthesized instruction before the data would break the
    /// premise and the byte-equality integration tests would trip.
    #[test]
    fn flag_parity_follows_message_length() {
        // rings message (len 50, even) → padded.
        let even = assert_message(Width::B, "d4", "eq", Some("#0"));
        assert!(even.len().is_multiple_of(2));
        assert_eq!(exit_flag_bytes(even.len().is_multiple_of(2)), vec![0xA0, 0x00]);
        // core `.l` message (odd) → bare $20.
        let odd = assert_message(Width::L, "a0", "hs", Some("#Object_RAM"));
        assert!(!odd.len().is_multiple_of(2));
        assert_eq!(exit_flag_bytes(odd.len().is_multiple_of(2)), vec![0x20]);
    }

    /// The cmp-form assert desugar has the §4.2 statement shape in order: SR
    /// save, `cmp`, pinned-`.w` branch, the raise label + tail, the skip label +
    /// restore. (Byte-exactness is proven by the integration tests; this pins
    /// the STRUCTURE — mnemonics, the pinned branch width, hygienic label names.)
    #[test]
    fn assert_expansion_structure_cmp_form() {
        let stmts = build_assert_expansion(7, &parts(Width::B, "d4", "eq", Some("#0")), sp());
        // 1: move.w sr,-(sp)
        assert!(matches!(&stmts[0], AsmStmt::Instr(i) if i.mnemonic == vec![TextOrSplice::Text("move".into())]));
        // 2: cmp.b (cmp form, not tst)
        let AsmStmt::Instr(cmp) = &stmts[1] else { panic!() };
        assert_eq!(cmp.mnemonic, vec![TextOrSplice::Text("cmp".into())]);
        assert_eq!(cmp.size, Some(TextOrSplice::Text("b".into())));
        // 3: beq.w — PINNED .w branch.
        let AsmStmt::Instr(br) = &stmts[2] else { panic!() };
        assert_eq!(br.mnemonic, vec![TextOrSplice::Text("beq".into())]);
        assert_eq!(br.size, Some(TextOrSplice::Text("w".into())), "branch width is pinned .w (§4.2 #3)");
        // The hygienic labels carry the unique instantiation id 7.
        let labels: Vec<&str> = stmts
            .iter()
            .filter_map(|s| if let AsmStmt::Label { name, .. } = s { Some(name.as_str()) } else { None })
            .collect();
        assert!(labels.contains(&"$diag7$raise"), "raise label is fresh: {labels:?}");
        assert!(labels.contains(&"$diag7$skip"), "skip label is fresh: {labels:?}");
    }

    /// The tst form emits `tst.<w>` (one operand), NOT `cmp` — and no dest.
    #[test]
    fn assert_expansion_structure_tst_form() {
        let stmts = build_assert_expansion(0, &parts(Width::W, "d1", "eq", None), sp());
        let AsmStmt::Instr(op) = &stmts[1] else { panic!() };
        assert_eq!(op.mnemonic, vec![TextOrSplice::Text("tst".into())]);
        assert_eq!(op.size, Some(TextOrSplice::Text("w".into())));
        assert_eq!(op.operands.len(), 1, "tst form tests src alone");
    }

    /// Two expansions with distinct ids never share a label symbol (hygiene).
    #[test]
    fn fresh_labels_per_instantiation() {
        let a = build_assert_expansion(1, &parts(Width::B, "d4", "eq", Some("#0")), sp());
        let b = build_assert_expansion(2, &parts(Width::B, "d4", "eq", Some("#0")), sp());
        let names = |v: &[AsmStmt]| -> Vec<String> {
            v.iter().filter_map(|s| if let AsmStmt::Label { name, .. } = s { Some(name.clone()) } else { None }).collect()
        };
        for na in names(&a) {
            assert!(!names(&b).contains(&na), "label `{na}` collides across expansions");
        }
    }

    /// `raise_error` desugar is the bare tail: it starts at the `.raise` label +
    /// `pea` — NO SR-save/`cmp`/branch prefix and NO `.skip` restore.
    #[test]
    fn raise_error_expansion_is_bare_tail() {
        let enc = encode_fstring("boom%<endl>Got: %<.b d0>").unwrap();
        let stmts = build_raise_error_expansion(3, &enc.bytes, vec![], sp());
        // First stmt is the raise label; second is `pea self(pc)`.
        assert!(matches!(&stmts[0], AsmStmt::Label { name, .. } if name == "$diag3$raise"));
        let AsmStmt::Instr(pea) = &stmts[1] else { panic!() };
        assert_eq!(pea.mnemonic, vec![TextOrSplice::Text("pea".into())]);
        // No `cmp`/`tst`/`b<cc>` anywhere (no compare wrapper).
        for s in &stmts {
            if let AsmStmt::Instr(i) = s {
                let m = match &i.mnemonic[0] { TextOrSplice::Text(t) => t.as_str(), _ => "" };
                assert!(!m.starts_with('b') || m == "boom", "no branch in raise_error tail: {m}");
                assert_ne!(m, "cmp");
                assert_ne!(m, "tst");
            }
        }
    }

    /// The FSTRING-arg operand builder enforces §5 (register or immediate only).
    #[test]
    fn fstring_arg_operand_reg_and_imm_only() {
        assert!(fstring_arg_operand("d0", sp()).is_some());
        assert!(fstring_arg_operand("#$8000", sp()).is_some());
        assert!(fstring_arg_operand("#Object_RAM", sp()).is_some());
        // A memory/EA operand is refused (the caller emits the steering error).
        assert!(fstring_arg_operand("(a0)", sp()).is_none());
        assert!(fstring_arg_operand("4(a0,d0.w)", sp()).is_none());
    }
}
