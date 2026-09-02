//! Synthetic witnesses for the seven 68000 condition-branch spellings that had
//! NONE — `bge bgt ble blt bmi bls bcc` — on the `.emp` lowering path.
//!
//! # Why this file exists
//!
//! A sweep of the 271 synthetic test texts in this workspace found these seven
//! spellings with **zero** synthetic witnesses, against eight positive controls
//! spanning 2–41 files (`beq` 24, `bra` 41, `bne` 21, `bsr` 18, `bcs` 5, `bpl` 5,
//! `bhi` 3, `blo` 2), so the empty result is an absence and not a broken query.
//! They reached sigil at all only because the Aeon source writes them (36 of 193
//! `.emp` files; `bmi` alone in 22), which means their sole witness was the
//! external corpus and the whole-ROM byte gate riding on it. These tests give
//! each spelling a witness that owns no dependency on any external tree.
//!
//! **SEVEN IS A FLOOR, NOT A TOTAL.** The sweep hand-inspected every NONE row but
//! did NOT hand-inspect the ~120 rows that scored above zero, and at least one of
//! those (`bcc`) turned out to be a comment-only hit that scored as covered. More
//! zero-witness constructs may still be hiding in the scored rows. Nothing here
//! entitles anyone to say the condition-branch differential is closed.
//!
//! # THE BAR: every expected byte below is DERIVED FROM THE ENCODER TABLE
//!
//! No expectation in this file was read off an Aeon ROM, listing or golden. An
//! expectation lifted from the corpus would pass forever while witnessing nothing
//! of its own — which is precisely the dependency these tests exist to break. The
//! derivation has exactly three in-repo inputs:
//!
//! 1. **The condition nibble.** `sigil-isa/src/m68k.rs` `enum Cond` — *"discriminant
//!    is the 4-bit cc field (bits 11–8)"*:
//!    `T=0x0 F=0x1 Hi=0x2 Ls=0x3 Cc=0x4 Cs=0x5 Ne=0x6 Eq=0x7`
//!    `Vc=0x8 Vs=0x9 Pl=0xA Mi=0xB Ge=0xC Lt=0xD Gt=0xE Le=0xF`.
//!    [`cc_nibble`] below re-reads that enum rather than restating it, so the
//!    derivation is executed at test time, not transcribed.
//! 2. **The composition.** `sigil-isa/src/m68k.rs` `encode_branch` — base word
//!    `0110 cccc dddddddd` = `0x6000 | (cc << 8)`; under `Size::S` the signed
//!    8-bit displacement occupies the **low byte**; under `Size::W` the low byte
//!    is `0x00` and a 16-bit displacement **word follows**. [`short_branch`] and
//!    [`word_branch`] below are that sentence as code.
//! 3. **The spelling→condition map under test.** `sigil-frontend-emp/src/lower/code.rs`
//!    `m68k_cond` — the copy this front end uses and the copy nothing unit-tested.
//!    It is the *subject*, so it is never consulted to build an expectation: the
//!    expectation is built from (1) and (2), and the test asks whether lowering the
//!    spelling reproduces it.
//!
//! The displacement itself comes from the `Disp` operand contract in the same
//! file: *"the already-resolved displacement … measured from `instruction_address
//! + 2`"*. Every fixture uses one fixed shape so that number is arithmetic and not
//! a lookup:
//!
//! ```text
//! proc p() {
//! .tgt:                     ; offset 0
//!     nop                   ; 4E 71          — 2 bytes, offsets 0..2
//!     b<cc>.s .tgt          ; branch opcode at offset 2
//!     rts                   ; 4E 75
//! }
//! ```
//!
//! The branch sits at offset 2, so it measures from 2 + 2 = 4, and `.tgt` is at 0:
//! **disp = 0 − 4 = −4**, i.e. `0xFC` as a signed byte and `0xFF 0xFC` as a signed
//! word. Hence, for a condition with nibble `cc`:
//!
//! - `.s` proc image: `4E 71 | (0x60|cc) FC | 4E 75`
//! - `.w` proc image: `4E 71 | (0x60|cc) 00 FF FC | 4E 75`
//!
//! Worked once by hand for `bmi` (`Mi` = `0xB`): `0x60|0xB` = `0x6B`, so the short
//! form is `6B FC` and the word form is `6B 00 FF FC`. Every other spelling is the
//! same arithmetic with its own nibble.

use sigil_backend_m68k::m68k::Cond;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::Level;

// ---------------------------------------------------------------------------
// The derivation, as executable code (input 1 and input 2 above).
// ---------------------------------------------------------------------------

/// The high byte of the branch family's base word: `0x6000 >> 8` from
/// `encode_branch`'s `let base = 0x6000 | (cc << 8);`.
const BRANCH_BASE_HI: u8 = 0x60;

/// `nop` / `rts` — the two fixed-opcode fillers that frame every fixture. Present
/// so the surrounding bytes of each assertion are themselves derived and not
/// magic: `nop` = `4E71`, `rts` = `4E75` (68000 fixed opcodes).
const NOP: [u8; 2] = [0x4E, 0x71];
const RTS: [u8; 2] = [0x4E, 0x75];

/// The 4-bit cc field for a condition, read from the ISA enum's discriminant —
/// the enum whose doc comment *is* the specification of that field. Anything
/// wider than a nibble is a contradiction of that doc comment, so assert it.
fn cc_nibble(c: Cond) -> u8 {
    let cc = c.cc();
    assert!(cc <= 0xF, "{c:?} discriminant {cc:#x} is not a 4-bit cc field");
    cc as u8
}

/// `encode_branch`, `Size::S` arm: base word `0x6000 | (cc << 8)` with the signed
/// 8-bit displacement in the low byte.
fn short_branch(c: Cond, disp: i8) -> [u8; 2] {
    [BRANCH_BASE_HI | cc_nibble(c), disp as u8]
}

/// `encode_branch`, `Size::W` arm: the same base word with a **zero** low byte,
/// followed by the signed 16-bit displacement word (big-endian).
fn word_branch(c: Cond, disp: i16) -> [u8; 4] {
    let d = disp.to_be_bytes();
    [BRANCH_BASE_HI | cc_nibble(c), 0x00, d[0], d[1]]
}

/// The displacement every fixture's branch resolves to: the branch opcode is at
/// offset 2, the `Disp` operand is measured from `instruction_address + 2` = 4,
/// and the target label `.tgt` is at offset 0.
const FIXTURE_DISP: i32 = 0 - (2 + 2);

// ---------------------------------------------------------------------------
// Lowering harness (mirrors tests/asm_splice.rs).
// ---------------------------------------------------------------------------

fn lower(src: &str) -> sigil_ir::Module {
    let (file, perrs) = parse_str(src);
    assert!(
        perrs.iter().all(|d| d.level != Level::Error),
        "unexpected parse diagnostics: {perrs:?}"
    );
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<&str> = ldiags
        .iter()
        .filter(|d| d.level == Level::Error)
        .map(|d| d.message.as_str())
        .collect();
    assert!(errs.is_empty(), "unexpected lower errors: {errs:?}");
    module
}

fn section<'a>(module: &'a sigil_ir::Module, name: &str) -> &'a Section {
    module
        .sections
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section `{name}`"))
}

fn proc_bytes(module: &sigil_ir::Module, name: &str, len: usize) -> Vec<u8> {
    let s = section(module, "text");
    let off = s
        .labels
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no label `{name}`"))
        .offset as usize;
    let resolved =
        sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let bytes = &linked.section("text").expect("linked text").bytes;
    bytes[off..off + len].to_vec()
}

/// Build the one fixture shape for a spelling, at a given width suffix.
fn fixture_src(spelling: &str, width: &str) -> String {
    format!(
        "module m\n\
         proc p() {{\n\
         .tgt:\n\
             nop\n\
             {spelling}{width}  .tgt\n\
             rts\n\
         }}\n"
    )
}

/// Assert BOTH widths of one spelling against the derived expectation.
///
/// `spelling` is the `.emp` mnemonic under test; `cond` is the condition the
/// encoder table says it must select. The expectation is assembled here from
/// [`short_branch`] / [`word_branch`] — never from a corpus image.
fn assert_spelling(spelling: &str, cond: Cond) {
    let disp8 = i8::try_from(FIXTURE_DISP).expect("fixture disp fits a byte");
    let disp16 = i16::try_from(FIXTURE_DISP).expect("fixture disp fits a word");

    let mut want_s = Vec::new();
    want_s.extend_from_slice(&NOP);
    want_s.extend_from_slice(&short_branch(cond, disp8));
    want_s.extend_from_slice(&RTS);
    let got_s = proc_bytes(&lower(&fixture_src(spelling, ".s")), "p", want_s.len());
    assert_eq!(
        got_s, want_s,
        "`{spelling}.s` must encode {cond:?} (cc {:#x}) with disp {FIXTURE_DISP}",
        cc_nibble(cond)
    );

    let mut want_w = Vec::new();
    want_w.extend_from_slice(&NOP);
    want_w.extend_from_slice(&word_branch(cond, disp16));
    want_w.extend_from_slice(&RTS);
    let got_w = proc_bytes(&lower(&fixture_src(spelling, ".w")), "p", want_w.len());
    assert_eq!(
        got_w, want_w,
        "`{spelling}.w` must encode {cond:?} (cc {:#x}) with disp {FIXTURE_DISP}",
        cc_nibble(cond)
    );
}

// ---------------------------------------------------------------------------
// The derivation's own guard.
//
// Every fixture below is only as good as the arithmetic in `short_branch` /
// `word_branch`. This test pins that arithmetic against a fully hand-written
// expectation for one condition, so a mistake in the shared helper cannot make
// all seven fixtures agree with each other and with nothing else.
// ---------------------------------------------------------------------------

#[test]
fn the_derivation_helpers_match_a_hand_expansion() {
    // Mi = 0xB per the `Cond` doc comment; 0x60 | 0xB = 0x6B; disp -4 = 0xFC.
    assert_eq!(cc_nibble(Cond::Mi), 0xB);
    assert_eq!(short_branch(Cond::Mi, -4), [0x6B, 0xFC]);
    assert_eq!(word_branch(Cond::Mi, -4), [0x6B, 0x00, 0xFF, 0xFC]);
    // A second condition at the other end of the nibble, so a helper that ignored
    // `cc` entirely would still be caught: Ls = 0x3; 0x60 | 0x3 = 0x63.
    assert_eq!(cc_nibble(Cond::Ls), 0x3);
    assert_eq!(short_branch(Cond::Ls, -4), [0x63, 0xFC]);
    // And the fixture's displacement is the arithmetic the module header states.
    assert_eq!(FIXTURE_DISP, -4);
}

// ---------------------------------------------------------------------------
// THE SEVEN. One test per zero-witness spelling.
// ---------------------------------------------------------------------------

/// `bge` — Cond::Ge, cc `0xC`. Short form `6C FC`, word form `6C 00 FF FC`.
#[test]
fn bge_encodes_condition_ge() {
    assert_spelling("bge", Cond::Ge);
}

/// `bgt` — Cond::Gt, cc `0xE`. Short form `6E FC`, word form `6E 00 FF FC`.
#[test]
fn bgt_encodes_condition_gt() {
    assert_spelling("bgt", Cond::Gt);
}

/// `ble` — Cond::Le, cc `0xF`. Short form `6F FC`, word form `6F 00 FF FC`.
#[test]
fn ble_encodes_condition_le() {
    assert_spelling("ble", Cond::Le);
}

/// `blt` — Cond::Lt, cc `0xD`. Short form `6D FC`, word form `6D 00 FF FC`.
#[test]
fn blt_encodes_condition_lt() {
    assert_spelling("blt", Cond::Lt);
}

/// `bmi` — Cond::Mi, cc `0xB`. Short form `6B FC`, word form `6B 00 FF FC`.
/// The most-written of the seven in the external corpus (22 of 193 files), so
/// the one whose silent regression would have been widest.
#[test]
fn bmi_encodes_condition_mi() {
    assert_spelling("bmi", Cond::Mi);
}

/// `bls` — Cond::Ls, cc `0x3`. Short form `63 FC`, word form `63 00 FF FC`.
#[test]
fn bls_encodes_condition_ls() {
    assert_spelling("bls", Cond::Ls);
}

/// `bcc` — Cond::Cc, cc `0x4`. Short form `64 FC`, word form `64 00 FF FC`.
/// Its one hit in the differential sweep was a **comment**
/// (`sigil-frontend-as/src/eval.rs`), which is how a scored-nonzero row can still
/// be a zero-witness row — the reason the header calls seven a floor.
#[test]
fn bcc_encodes_condition_cc() {
    assert_spelling("bcc", Cond::Cc);
}

// ---------------------------------------------------------------------------
// Adjacent guards: the seven must be DISTINCT, and must not collide with the
// unconditional forms. A `m68k_cond` that mapped every spelling to one condition
// would satisfy each test above only if that condition were right seven times —
// but a table that dropped an arm and fell through to a neighbour would not be
// caught by any single fixture, so pin the whole set's shape too.
// ---------------------------------------------------------------------------

/// The seven nibbles are seven distinct values, none of them `0x0` (`bra`) or
/// `0x1` (`bsr`) — `encode_branch` gives those two the cc field, so a condition
/// wrongly mapped there would silently become an unconditional branch or a
/// subroutine call.
#[test]
fn the_seven_conditions_are_distinct_and_not_bra_or_bsr() {
    let seven = [Cond::Ge, Cond::Gt, Cond::Le, Cond::Lt, Cond::Mi, Cond::Ls, Cond::Cc];
    let mut nibbles: Vec<u8> = seven.iter().copied().map(cc_nibble).collect();
    nibbles.sort_unstable();
    let before = nibbles.len();
    nibbles.dedup();
    assert_eq!(nibbles.len(), before, "two of the seven share a cc field: {nibbles:?}");
    for c in seven {
        let cc = cc_nibble(c);
        assert_ne!(cc, 0x0, "{c:?} collides with the `bra` cc field");
        assert_ne!(cc, 0x1, "{c:?} collides with the `bsr` cc field");
    }
}

/// End-to-end distinctness on the lowering path: all seven spellings lowered
/// through the SAME fixture shape must produce seven different opcode bytes. This
/// is the assertion that a fall-through in `m68k_cond` (one arm deleted, the next
/// arm answering for it) cannot survive.
#[test]
fn the_seven_spellings_lower_to_seven_different_opcodes() {
    let seven = ["bge", "bgt", "ble", "blt", "bmi", "bls", "bcc"];
    let mut seen: Vec<(u8, &str)> = Vec::new();
    for s in seven {
        let bytes = proc_bytes(&lower(&fixture_src(s, ".s")), "p", 6);
        let opcode_hi = bytes[2];
        if let Some((_, other)) = seen.iter().find(|(b, _)| *b == opcode_hi) {
            panic!("`{s}` and `{other}` both lower to opcode byte {opcode_hi:#04x}");
        }
        seen.push((opcode_hi, s));
    }
}
