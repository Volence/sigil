//! The 21 Z80 mnemonics the `.emp` front end had no name for, driven end to end
//! and checked against the OTHER front end's answer for the same instruction.
//!
//! Every expectation here is DERIVED, never typed: each snippet is assembled
//! twice — once through `sigil-frontend-as` (whose own coverage suite pins these
//! forms against the reference asl) and once through `.emp` — and the two byte
//! strings are required to agree. Typing the opcodes instead would be the weak
//! form of this test for exactly this instruction group: the ED block ops are a
//! 4x4 grid at `ED A0 | family | direction << 3 | repeat << 4`, so a table that
//! transposed its two axes, or ran either off by one, still lands on a legal
//! instruction from the same grid and reads as plausible.
//!
//! Cross-frontend agreement alone is still not enough, because one shared wrong
//! answer would satisfy it. Two further properties are required:
//!
//!   * PAIRWISE DISTINCTNESS across the grid — a table that collapsed two
//!     neighbours onto one opcode passes every individual comparison that
//!     happens to look at the surviving one;
//!   * REGISTER-CODE coverage on the `(c)` port forms — `in r,(c)` places the
//!     register in bits 5..3, and `a` is code 7 while `b` is code 0, so a family
//!     exercised only on `b` leaves the register shift entirely unexercised.
//!
//! The `.emp` surface spellings are the ones every Z80 reference uses: `in a,(c)`
//! and `out (c),a` for the C-addressed port, `in a,(127)` / `out (127),a` for the
//! direct port. The immediate is a BARE numeral inside the parentheses, which is
//! `.emp`'s own absolute-address spelling (`ld a, ($4000)`), not the 68k `#`
//! immediate form.
//!
//! Every snippet below is ONE string driving BOTH front ends, so the two cannot
//! be compared on subtly different instructions. That is why the numerals are
//! DECIMAL: `$` is the program counter in the AS source language, so a `$7f`
//! there is not the hex literal `.emp` reads it as, and a shared string has to
//! avoid the token whose meaning differs.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Fragment, Module, SymbolTable};
use sigil_span::Level;

// ---- the two front ends, each reduced to `snippet -> bytes` ----------------

/// Assemble one Z80 snippet through the AS front end: parse -> lower -> link ->
/// flatten. This is the DERIVATION source for every expectation below.
fn as_bytes(snippet: &str) -> Vec<u8> {
    let src = format!("        cpu z80\n        phase 0\n        {snippet}\n");
    let module = sigil_frontend_as::assemble(&src, &sigil_frontend_as::Options::default())
        .unwrap_or_else(|d| panic!("AS assemble `{snippet}` failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link `{snippet}` failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00).unwrap()
}

/// Parse + lower `src` for the 68k default (a Z80 section opts in explicitly).
fn lower(src: &str) -> (Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "emp parse: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    )
}

/// The linked bytes of a named section.
fn section_bytes(module: &Module, name: &str) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let _ = Fragment::Data as usize as u8; // keep the import honest across refactors
    linked.section(name).expect("linked section").bytes.clone()
}

/// The emp source for one Z80 body line: a `proc` in a `(cpu: z80, vma: $0)`
/// section, terminated by `ret` so no `[proc.undeclared-fallthrough]` fires.
fn emp_src(snippet: &str) -> String {
    format!(
        "module m\nsection s (cpu: z80, vma: $0) {{\n  proc P() {{\n    {snippet}\n    ret\n  }}\n}}\n"
    )
}

/// Assemble one Z80 snippet through the `.emp` front end, returning the
/// snippet's OWN bytes (the trailing `ret`, which the wrapper adds, is stripped
/// after being asserted present — a snippet that silently emitted nothing would
/// otherwise compare equal to nothing).
fn emp_bytes(snippet: &str) -> Vec<u8> {
    let (module, diags) = lower(&emp_src(snippet));
    assert!(diags.is_empty(), "emp lower `{snippet}`: {diags:?}");
    let mut b = section_bytes(&module, "s");
    assert_eq!(
        b.pop(),
        Some(0xC9),
        "`{snippet}`: the wrapper's trailing `ret` (C9) is missing, so these bytes are not \
         the snippet's own"
    );
    assert!(!b.is_empty(), "`{snippet}` emitted no bytes of its own");
    b
}

/// The emp lowering diagnostics for a snippet, WITHOUT asserting them empty.
fn emp_diags(snippet: &str) -> Vec<sigil_span::Diagnostic> {
    let (file, perrs) = parse_str(&emp_src(snippet));
    let parse_errs: Vec<_> = perrs.into_iter().filter(|d| d.level == Level::Error).collect();
    if !parse_errs.is_empty() {
        return parse_errs;
    }
    lower(&emp_src(snippet)).1
}

// ---- the population --------------------------------------------------------

/// The 21 names `.emp` could not spell, each with every operand form the ISA
/// encodes for it. The list is the FULL difference between the two front ends'
/// mnemonic tables, so a name dropped from here is a name that stops being
/// checked — which is why `the_population_is_all_21_names` re-derives the count.
const CASES: &[(&str, &str)] = &[
    // The LD block family.
    ("ldi", "ldi"),
    ("ldd", "ldd"),
    ("lddr", "lddr"),
    // The CP block family.
    ("cpi", "cpi"),
    ("cpd", "cpd"),
    ("cpir", "cpir"),
    ("cpdr", "cpdr"),
    // The IN block family.
    ("ini", "ini"),
    ("ind", "ind"),
    ("inir", "inir"),
    ("indr", "indr"),
    // The OUT block family.
    ("outi", "outi"),
    ("outd", "outd"),
    ("otir", "otir"),
    ("otdr", "otdr"),
    // The port I/O group, both addressing modes and both ends of the register
    // code range (`a` = 7, `b` = 0).
    ("in", "in a, (c)"),
    ("in", "in b, (c)"),
    ("in", "in a, (127)"),
    ("out", "out (c), a"),
    ("out", "out (c), b"),
    ("out", "out (127), a"),
    // The two interrupt returns and the two BCD nibble rotates.
    ("reti", "reti"),
    ("retn", "retn"),
    ("rrd", "rrd"),
    ("rld", "rld"),
];

// ---- the gates -------------------------------------------------------------

/// GUARD: the population is the whole 21-name difference, and it is actually
/// walked. An empty or shrunken table makes every comparison below pass by
/// examining almost nothing, which is the failure mode a table-driven test has
/// that 21 hand-written cases do not.
#[test]
fn the_population_is_all_21_names() {
    let mut names: Vec<&str> = CASES.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    names.dedup();
    let expected = [
        "cpd", "cpdr", "cpi", "cpir", "in", "ind", "indr", "ini", "inir", "ldd", "lddr", "ldi",
        "otdr", "otir", "out", "outd", "outi", "reti", "retn", "rld", "rrd",
    ];
    assert_eq!(names, expected, "the case table no longer covers exactly the 21 added names");
    assert_eq!(names.len(), 21);
    assert!(CASES.len() >= 25, "the case table lost its multi-form port rows");
}

/// THE GATE: `.emp` and the AS front end must produce the SAME bytes for the
/// same instruction, for every form. The expectation is the AS front end's own
/// answer, computed in this test run — nothing is copied from a neighbouring
/// pin, and nothing is typed from memory.
#[test]
fn emp_agrees_with_the_as_frontend_on_every_form() {
    let mut wrong = Vec::new();
    let mut checked = 0usize;
    for (name, snippet) in CASES {
        let want = as_bytes(snippet);
        assert!(!want.is_empty(), "AS produced no bytes for `{snippet}`; the derivation is dead");
        let got = emp_bytes(snippet);
        checked += 1;
        if got != want {
            wrong.push(format!("`{snippet}` ({name}): AS says {want:02X?}, emp says {got:02X?}"));
        }
    }
    assert_eq!(checked, CASES.len(), "not every case was examined");
    assert!(
        wrong.is_empty(),
        "{} of {checked} form(s) disagree between the front ends:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// The sixteen-member ED block grid must be PAIRWISE DISTINCT under `.emp`.
/// Cross-frontend agreement cannot catch a collapse that both tables share, and
/// this grid is the one where a collapse is invisible to a spot check: every
/// member is a legal instruction one bit away from three others.
#[test]
fn the_ed_block_grid_is_pairwise_distinct() {
    let grid = [
        "ldi", "ldd", "ldir", "lddr", "cpi", "cpd", "cpir", "cpdr", "ini", "ind", "inir", "indr",
        "outi", "outd", "otir", "otdr",
    ];
    let encoded: Vec<(&str, Vec<u8>)> = grid.iter().map(|m| (*m, emp_bytes(m))).collect();
    assert_eq!(encoded.len(), 16, "the grid lost a member");
    for (i, (a, ba)) in encoded.iter().enumerate() {
        assert_eq!(ba.len(), 2, "`{a}` is not a 2-byte ED form: {ba:02X?}");
        assert_eq!(ba[0], 0xED, "`{a}` is not ED-prefixed: {ba:02X?}");
        for (b, bb) in encoded.iter().skip(i + 1) {
            assert_ne!(ba, bb, "`{a}` and `{b}` encode identically ({ba:02X?})");
        }
    }
}

/// The `(c)` port operand must be the C-ADDRESSED port, not a direct port whose
/// address happened to fold from a symbol named `c`. `in a,(c)` is a 2-byte ED
/// form; `in a,(127)` is a 2-byte unprefixed form — confusing them is the single
/// most plausible way to get this operand wrong, and the two differ in length
/// nowhere, only in the prefix byte.
#[test]
fn the_c_port_and_the_direct_port_are_different_instructions() {
    let ind_c = emp_bytes("in a, (c)");
    let direct = emp_bytes("in a, (127)");
    assert_eq!(ind_c[0], 0xED, "`in a,(c)` must be ED-prefixed, got {ind_c:02X?}");
    assert_ne!(direct[0], 0xED, "`in a,(127)` must NOT be ED-prefixed, got {direct:02X?}");
    assert_ne!(ind_c, direct);
    // And the register really rides the opcode: `a` (code 7) and `b` (code 0)
    // must differ, in both directions.
    assert_ne!(emp_bytes("in a, (c)"), emp_bytes("in b, (c)"));
    assert_ne!(emp_bytes("out (c), a"), emp_bytes("out (c), b"));
}

/// SCOPE PROPERTY: a name that is now RECOGNIZED but whose operand shape the Z80
/// model cannot reach must produce the bounded `[lower.z80-unsupported]`
/// diagnostic — never a parse error (which would read as "no such instruction")
/// and never bytes. `in b,(127)` is the case: only `in a,(n)` has an encoding,
/// so the register form of the direct port is recognized-but-unencodable.
#[test]
fn a_recognized_but_unencodable_form_is_bounded_scope_not_a_parse_error() {
    let diags = emp_diags("in b, (127)");
    assert!(!diags.is_empty(), "`in b,(127)` lowered clean; it has no encoding");
    assert!(
        diags.iter().any(|d| d.message.contains("[lower.z80-unsupported]")),
        "expected `[lower.z80-unsupported]`, got: {diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("unknown mnemonic")
            || d.message.contains("not a recognized")),
        "the name must be RECOGNIZED; only its operand shape is unsupported: {diags:?}"
    );
}

/// `(c)` is read as the C-addressed port ONLY under `in`/`out`, mirroring the AS
/// front end's own `matches!(m, In | Out)` condition. Under any other mnemonic a
/// parenthesised `c` keeps whatever meaning it had before this change, which is
/// what makes the new spelling byte-neutral for every program that already
/// assembles: `ld a,(c)` with a comptime `c` is still the absolute-address form.
#[test]
fn the_c_port_spelling_is_confined_to_in_and_out() {
    let src = "module m\n\
               section s (cpu: z80, vma: $0) {\n\
                 const c = 16384\n\
                 proc P() {\n\
                   ld a, (c)\n\
                   ret\n\
                 }\n\
               }\n";
    let (module, diags) = lower(src);
    assert!(diags.is_empty(), "`ld a,(c)` with a comptime `c`: {diags:?}");
    // `ld a,(nn)` is 3A nn nn — the ABSOLUTE form, unchanged by the new `(c)`
    // spelling. Derived from the AS front end rather than typed.
    let want = as_bytes("ld a,(16384)");
    let mut got = section_bytes(&module, "s");
    assert_eq!(got.pop(), Some(0xC9));
    assert_eq!(got, want, "a non-`in`/`out` `(c)` must still be the absolute-address form");
}
