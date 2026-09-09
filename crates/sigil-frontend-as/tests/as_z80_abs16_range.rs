//! A Z80 `jp`/`call`/`ld rr` whose 16-bit operand is a SYMBOL is range-checked
//! exactly like one whose operand is a literal, instead of being cast to `u16`.
//!
//! ## What was silently wrong
//!
//! The literal paths already refused (`jp 10000h` is `operand 65536 out of
//! range 0..=65535`), but a bare symbol took a different arm:
//!
//! ```text
//!     Big equ 10000h                  Big1 equ 12345h
//!     jp Big        ->  C3 00 00      call Big1     ->  CD 45 23
//!     Neg equ -1                      ld hl,Big1    ->  21 45 23
//!     jp Neg        ->  C3 FF FF
//! ```
//!
//! exit 0, no diagnostic: `Imm16(v as u16)` on the resolved side, and a
//! `BankPtr16Le` fixup (the linker's masking address kind, `value as u16`) on
//! the unresolved side, where the `.emp` twin `lower_z80_abs16_sym` and the AS
//! `dw` deferral both emit the range-checked `Value16Le`.
//!
//! ## The reference
//!
//! asl (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked on every run) refuses every one of `jp Big`, `call Big1`,
//! `ld hl,Big1` and `jp Neg` with `error #1320: range overflow`, exit 2, and
//! assembles `jp 0FFFFh` / `call 0FFFFh` / `ld hl,0FFFFh` / `ld hl,-32768` to
//! `C3 FF FF` / `CD FF FF` / `21 FF FF` / `21 00 80`, exit 0.

use sigil_frontend_as::{assemble_root_located, Options};
use sigil_ir::{Module, SymbolTable, SymbolValue};

fn assemble(body: &str) -> Result<Module, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => Ok(m),
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        // `file(line):col`: the line is what sits BETWEEN the
                        // parens. Reading to the end of the string instead used
                        // to work only because nothing followed the `)`, and it
                        // returned 0 (a line no source has) the moment the
                        // column arrived.
                        l.rsplit_once('(')
                            .and_then(|(_, rest)| rest.split_once(')'))
                            .and_then(|(n, _)| n.parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

fn link_with(m: &Module, stubs: &SymbolTable) -> Result<Vec<u8>, Vec<String>> {
    let resolved = sigil_link::resolve_layout(&m.sections, stubs, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, stubs).map_err(|ds| ds.into_iter().map(|d| d.message).collect::<Vec<_>>())?;
    Ok(m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default())
}

fn bytes(body: &str) -> Vec<u8> {
    link_with(&assemble(body).expect("front end accepts"), &SymbolTable::new()).expect("link accepts")
}

fn assert_refused(body: &str, line: u32, needles: &[&str]) {
    match assemble(body) {
        Ok(m) => {
            let b = link_with(&m, &SymbolTable::new()).unwrap_or_default();
            panic!("assembled to {b:02X?} instead of refusing the out-of-range symbol");
        }
        Err(diags) => {
            let hit = diags.iter().find(|(l, msg)| *l == line && needles.iter().all(|n| msg.contains(n)));
            assert!(hit.is_some(), "no diagnostic at line {line} naming {needles:?}; got {diags:?}");
        }
    }
}

const Z80: &str = "\tcpu z80\n";

#[test]
fn jp_and_call_to_a_symbol_above_ffff_are_refused_like_a_literal() {
    assert_refused(&format!("{Z80}Big equ 10000h\n\tjp Big\n"), 3, &["65536", "out of range 0..=65535"]);
    assert_refused(&format!("{Z80}Big1 equ 12345h\n\tcall Big1\n"), 3, &["74565", "out of range 0..=65535"]);
    assert_refused(&format!("{Z80}Big1 equ 12345h\n\tjp nz,Big1\n"), 3, &["74565", "out of range 0..=65535"]);
}

#[test]
fn jp_to_a_negative_symbol_is_refused_where_asl_refuses_it() {
    assert_refused(&format!("{Z80}Neg equ -1\n\tjp Neg\n"), 3, &["-1", "out of range 0..=65535"]);
}

#[test]
fn ld_rr_from_a_symbol_takes_the_word_window() {
    assert_refused(&format!("{Z80}Big1 equ 12345h\n\tld hl,Big1\n"), 3, &["74565", "out of range -32768..=65535"]);
    assert_refused(&format!("{Z80}Low equ -32769\n\tld hl,Low\n"), 3, &["-32769", "out of range -32768..=65535"]);
    // The signed floor asl accepts for a word operand.
    assert_eq!(bytes(&format!("{Z80}Neg equ -32768\n\tld hl,Neg\n")), vec![0x21, 0x00, 0x80]);
}

#[test]
fn symbolic_controls_assemble_to_asl_bytes() {
    assert_eq!(bytes(&format!("{Z80}Top equ 0FFFFh\n\tjp Top\n\tcall Top\n\tld hl,Top\n")), vec![0xC3, 0xFF, 0xFF, 0xCD, 0xFF, 0xFF, 0x21, 0xFF, 0xFF]);
    assert_eq!(bytes(&format!("{Z80}\tjp Fwd\n\tcall Fwd\nFwd:\tnop\n")), vec![0xC3, 0x06, 0x00, 0xCD, 0x06, 0x00, 0x00]);
}

/// The unresolved side: a symbol the front end never sees resolves at link,
/// where the fixup is the range-checked value kind the `.emp` twin emits.
#[test]
fn a_link_time_symbol_above_ffff_is_refused_by_the_linker() {
    let m = assemble(&format!("{Z80}\tjp Extern\n\tld hl,Extern\n")).expect("defers");
    let mut big = SymbolTable::new();
    big.define("Extern", SymbolValue::Int(0x1_0000));
    let err = link_with(&m, &big).expect_err("$10000 does not fit a Z80 address");
    assert!(err.iter().any(|d| d.contains("does not fit a 16-bit cell")), "got {err:?}");
    let mut top = SymbolTable::new();
    top.define("Extern", SymbolValue::Int(0xFFFF));
    assert_eq!(link_with(&m, &top).expect("$FFFF fits"), vec![0xC3, 0xFF, 0xFF, 0x21, 0xFF, 0xFF]);
}
