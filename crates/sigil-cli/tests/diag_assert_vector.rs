//! CROSS-FRONT-END acceptance vectors for the `.emp` diagnostics construct
//! (`assert` / `raise_error`), diagnostics-construct build Task 4.
//!
//! Task 3 (`sigil-frontend-emp/tests/diag_desugar.rs`) proved the construct's
//! expansion equals a hand-written transliteration — but BOTH sides went through
//! the EMP front-end, so an EMP-vs-AS encoding divergence would be invisible.
//! THIS file closes that gap: the golden reference is assembled by the
//! INDEPENDENT `sigil_frontend_as` front-end running vladikcomper's REAL `assert`
//! / `RaiseError` macros from `aeon/engine/debug/debugger.asm` (the AS front-end
//! supports the full macro tower this uses — `macro`/`switch`/`case`/`while`/
//! `strstr`/`substr`/`val`/`lowstring`/`!error`/`!align`/`padding off`), and the
//! candidate is the `.emp` construct lowered with `DEBUG=1`. Equal linked bytes
//! prove the construct reproduces the debugger.asm macro's encoding EXACTLY,
//! independent of the EMP-side desugar the Task-3 twin shared.
//!
//! Because the reference runs the real macro, there is ZERO hand-transcription of
//! message/flag bytes here (the plan's PREFERRED path). The AS and EMP sides
//! agree on the two `MDDBG__*` handler entry points + the operand symbols
//! (`Object_RAM`, `NUM_DYNAMIC`) by pinning them to the SAME fixed addresses on
//! both sides (`equ` in the AS source; a stub `SymbolTable` for the EMP link).
//!
//! Plus §5 negative probes: the construct must be REJECTED (with a steering
//! message) for a memory `src`, an unknown cond, an unknown fstring token, a
//! `consoleprogram` second arg, and an fstring param byte < $80.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable, SymbolValue};
use sigil_span::Level;

// ---------------------------------------------------------------------------
// Shared symbol addresses — pinned IDENTICALLY on both sides so the two
// front-ends resolve the same operands to the same bytes.
// ---------------------------------------------------------------------------

const SYMS: &[(&str, i64)] = &[
    ("MDDBG__ErrorHandler", 0x0010_0000),
    ("MDDBG__ErrorHandler_PagesController", 0x0010_0100),
    ("Object_RAM", 0x00FF_B000),
    ("NUM_DYNAMIC", 0x60),
];

/// The stub symbol table the EMP side links against (the `MDDBG__*` handlers +
/// the operand symbols), matching the AS side's `equ`s byte-for-byte.
fn stubs() -> SymbolTable {
    let mut t = SymbolTable::new();
    for (name, addr) in SYMS {
        t.define(name, SymbolValue::Int(*addr));
    }
    t
}

/// Link a single-section module and return its bytes (mirrors
/// `table_plc_vector.rs`'s `linked_all`, but with the shared stub table so the
/// `MDDBG__*`/operand externs resolve).
fn linked(m: &Module, st: &SymbolTable) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&m.sections, st, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, st).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .expect("a linked section")
}

// ---------------------------------------------------------------------------
// AS reference — the REAL debugger.asm `assert` / `RaiseError` macros.
// ---------------------------------------------------------------------------

/// Assemble one diagnostic `site` (a raw `assert`/`RaiseError` line) through the
/// AS front-end running debugger.asm's ACTUAL macro tower, and return the
/// expansion bytes. `__DEBUG__` is defined so the `assert` macro's `ifdef`
/// emits; the `MDDBG__*` handlers + operand symbols are `equ`'d to the SAME
/// fixed addresses the EMP stub table uses; the config symbols debugger.asm's
/// header references (`MDDBG__Debugger_*`, `MDDBG__Str_OffsetLocation_24bit`)
/// are stubbed so the file assembles. The site sits in its own `phase 0`
/// section after the (byte-emitting-nothing) macro definitions, so the linked
/// bytes ARE the expansion.
fn as_reference(site: &str) -> Vec<u8> {
    let debugger =
        std::fs::read_to_string("/home/volence/sonic_hacks/aeon/engine/debug/debugger.asm")
            .expect("read debugger.asm");
    let asm = format!(
        "cpu 68000\n\
__DEBUG__: equ 1\n\
MDDBG__ErrorHandler: equ $100000\n\
MDDBG__ErrorHandler_PagesController: equ $100100\n\
MDDBG__Debugger_AddressRegisters: equ 0\n\
MDDBG__Debugger_Backtrace: equ 0\n\
MDDBG__Str_OffsetLocation_24bit: equ 0\n\
Object_RAM: equ $FFB000\n\
NUM_DYNAMIC: equ $60\n\
{debugger}\n\
phase 0\n\
Test:\n\
{site}\n"
    );
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let m = assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble `{site}`: {d:#?}"));
    linked(&m, &SymbolTable::new())
}

// ---------------------------------------------------------------------------
// EMP candidate — the construct lowered with DEBUG=1.
// ---------------------------------------------------------------------------

/// Lower a proc body containing the construct with the given `DEBUG` value (set
/// via a `-D`-style `defines` entry on `LowerOptions`, the same mechanism the
/// Task-3 desugar tests use), link it against the shared stub table, and return
/// its bytes. `assert` needs DEBUG=1 to emit; `raise_error` is unconditional
/// (so its vector passes DEBUG=0).
fn emp_candidate(body: &str, debug: i128) -> Vec<u8> {
    // Clobbers cover every register any vector's site touches; `preserves(sr)`
    // and a terminating `rts` keep the proc well-formed (no fallthrough warning)
    // without adding bytes to the measured expansion (the `rts` follows the last
    // labelled statement — we compare whole-section bytes, and `rts` is a shared
    // 2-byte tail present in neither reference, so each `body` ends WITHOUT rts
    // and we compare the section up to it; see below).
    let src = format!(
        "module m\n\
section s (cpu: m68000) {{\n\
    proc p () clobbers(d0, d1, d4, d7) preserves(sr) {{\n\
{body}\n\
    }}\n\
}}\n"
    );
    let (file, perrs) = parse_str(&src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse `{body}`: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), debug)],
        },
    );
    assert!(
        diags.iter().all(|d| d.level != Level::Error),
        "lower `{body}`: {:?}",
        diags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    linked(&m, &stubs())
}

/// The construct's expansion has no natural terminator, so the EMP proc bodies
/// carry no trailing `rts`; the whole linked section IS the expansion. Assert
/// AS == EMP with a hex dump on mismatch.
fn assert_vector(label: &str, site: &str, body: &str, debug: i128) {
    let a = as_reference(site);
    let e = emp_candidate(body, debug);
    let hex = |v: &[u8]| v.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
    assert_eq!(
        a, e,
        "\n[{label}] AS real-macro expansion must equal the EMP construct\n  AS  ({:3}): {}\n  EMP ({:3}): {}",
        a.len(),
        hex(&a),
        e.len(),
        hex(&e)
    );
    // Guard against a degenerate empty match (both front-ends silently emitting
    // nothing would trivially pass); the smallest real expansion is > 16 bytes.
    assert!(a.len() > 16, "[{label}] expansion is suspiciously short ({} bytes)", a.len());
}

// ---------------------------------------------------------------------------
// 4 positive vectors (DEBUG shape) — the construct vs the REAL macro.
// ---------------------------------------------------------------------------

/// `.b` no-dest (tst form): `assert.b d4, eq` → `tst.b d4` + branch + tail.
#[test]
fn assert_b_no_dest_tst_form() {
    assert_vector("b-no-dest (tst)", "        assert.b        d4, eq", "        assert.b d4, eq", 1);
}

/// `.w` with-dest immediate-symbol: `assert.w d7, lo, #NUM_DYNAMIC` — cmp form
/// with a symbol immediate (`cmp.w #NUM_DYNAMIC, d7`), even-parity `$A0,$00`.
#[test]
fn assert_w_with_dest_symbol_immediate() {
    assert_vector(
        "w-dest #symbol",
        "        assert.w        d7, lo, #NUM_DYNAMIC",
        "        assert.w d7, lo, #NUM_DYNAMIC",
        1,
    );
}

/// `.l` with-dest — the `$20`-no-pad parity case: `assert.l a0, hs, #Object_RAM`
/// has an ODD-length message, so the exit flag is the bare `$20` (no `$80`
/// align bit, no `$00` pad). This is the parity path the `.b`/`.w` vectors don't
/// exercise.
#[test]
fn assert_l_with_dest_odd_parity_no_pad() {
    assert_vector(
        "l-dest $20-no-pad",
        "        assert.l        a0, hs, #Object_RAM",
        "        assert.l a0, hs, #Object_RAM",
        1,
    );
}

/// `raise_error` with a `%<.b dN>` arg (path_swap.asm shape). Unconditional, so
/// lowered with DEBUG=0 — the reference is the real `RaiseError` macro.
#[test]
fn raise_error_with_byte_arg() {
    assert_vector(
        "raise_error %<.b d0>",
        "        RaiseError      \"Bad path swap!%<endl>Got: %<.b d0>\"",
        "        raise_error \"Bad path swap!%<endl>Got: %<.b d0>\"",
        0,
    );
}

// ---------------------------------------------------------------------------
// 5 negative probes (§5) — the construct must be REJECTED with a steering
// message. These run through the EMP front-end only (there is nothing to
// assemble on the AS side — the point is the construct's OWN validation).
// ---------------------------------------------------------------------------

/// Collect all parse + lower diagnostic messages (any level) for a proc body.
fn diag_messages(body: &str) -> String {
    let src = format!(
        "module m\n\
section s (cpu: m68000) {{\n\
    proc p () clobbers(d0, d1, d4) {{\n\
{body}\n\
    }}\n\
}}\n"
    );
    let (file, perrs) = parse_str(&src);
    let mut msgs: Vec<String> = perrs.iter().map(|d| d.message.clone()).collect();
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), 1)],
        },
    );
    msgs.extend(diags.iter().map(|d| d.message.clone()));
    msgs.join("\n---\n")
}

/// A memory `src` (`(a0)`) must be rejected — v1 requires a register, and the
/// message steers to "move ... to a register first" (§5).
#[test]
fn negative_memory_src_rejected() {
    let msg = diag_messages("        assert.b (a0), eq");
    assert!(
        msg.contains("register") && msg.to_lowercase().contains("move"),
        "memory src must steer to a register: {msg}"
    );
}

/// An unknown condition code must be rejected, listing the 16 valid codes (§5).
#[test]
fn negative_unknown_cond_rejected() {
    let msg = diag_messages("        assert.b d4, zz, #0");
    assert!(
        msg.to_lowercase().contains("condition") && msg.contains("zz") && msg.contains("eq"),
        "unknown cond must be rejected listing the codes: {msg}"
    );
}

/// An unknown fstring token in `raise_error` must be rejected, naming the token
/// and the valid set (§5).
#[test]
fn negative_unknown_fstring_token_rejected() {
    let msg = diag_messages("        raise_error \"bad %<bogus>\"");
    assert!(
        msg.contains("FSTRING") && msg.contains("bogus"),
        "unknown fstring token must be rejected naming it: {msg}"
    );
}

/// A `consoleprogram` second argument to `raise_error` is out of scope (§3/§5) —
/// a compile error with a steering diagnostic.
#[test]
fn negative_consoleprogram_second_arg_rejected() {
    let msg = diag_messages("        raise_error \"boom\", MyDebugger");
    assert!(
        msg.to_lowercase().contains("consoleprogram") || msg.contains("one string"),
        "consoleprogram second arg must be rejected: {msg}"
    );
}

/// An fstring param that resolves below `$80` (`forced`=$04 with no base) must be
/// rejected — the macro's own `if (val(.__param) < $80) !error` check (§5).
#[test]
fn negative_fstring_param_below_0x80_rejected() {
    let msg = diag_messages("        raise_error \"x %<.b d0 forced>\"");
    assert!(
        msg.contains("$80") || msg.to_lowercase().contains("illegal"),
        "param byte < $80 must be rejected: {msg}"
    );
}
