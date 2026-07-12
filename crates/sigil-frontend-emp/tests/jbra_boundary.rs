//! Regression: an unsized `jbra` whose forward displacement EXCEEDS the `.s`
//! range (+127) must relax to `.w` (4 bytes), never stay `.s` (2 bytes).
//! Surfaced by the tranche-11 A1 camera-bias fold, which grew a `jbra
//! .next_object` from ~120 to ~130 bytes forward; sigil kept it `.s` (a 130-byte
//! `bra.s` cannot reach — an invalid encoding / broken ROM). asl correctly
//! width-selected `.w`.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Lower `src`, resolve+link the `text` section, return its final bytes.
fn text_bytes(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower: {ldiags:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section("text").expect("text section").bytes.clone()
}

/// Build a proc: `jbra .far`, then `nops` nops (2 bytes each), then `.far: rts`.
fn jbra_over(nops: usize) -> String {
    let body: String = std::iter::repeat("        nop\n").take(nops).collect();
    format!("module m\nproc p() {{\n        jbra .far\n{body}    .far:\n        rts\n}}\n")
}

/// 64 nops = 128 bytes between the `jbra` and `.far`: with a 2-byte `bra.s`,
/// `.far` sits 128 bytes past the branch's PC+2 — disp 128 > 127, so `.s` is
/// out of range and the branch MUST widen to `bra.w` (opcode `60 00 hh ll`).
#[test]
fn jbra_just_over_short_range_widens_to_word() {
    let bytes = text_bytes(&jbra_over(64));
    // bra.w = `60 00` + 16-bit disp; bra.s = `60 dd` (dd != 0). So byte 1 == 0
    // iff the branch widened to `.w`.
    assert_eq!(
        bytes[0], 0x60,
        "the branch opcode high byte must be a bra (0x60)"
    );
    assert_eq!(
        bytes[1], 0x00,
        "a jbra 128 bytes forward MUST be bra.w (60 00 ..), not bra.s — got byte1={:#04x} (bra.s with disp {}, unreachable)",
        bytes[1], bytes[1]
    );
    // And the whole branch is 4 bytes → total = 4 + 64*2 + 2 = 134.
    assert_eq!(bytes.len(), 134, "bra.w (4) + 64 nops (128) + rts (2)");
}

/// Control: 62 nops = 124 bytes → disp 124 ≤ 127, so `.s` (2 bytes) is in
/// range and stays `.s` (byte 1 = the non-zero displacement).
#[test]
fn jbra_within_short_range_stays_short() {
    let bytes = text_bytes(&jbra_over(62));
    assert_eq!(bytes[0], 0x60, "bra opcode");
    assert_ne!(
        bytes[1], 0x00,
        "a jbra 124 bytes forward should stay bra.s (60 dd, dd != 0)"
    );
    // bra.s (2) + 62 nops (124) + rts (2) = 128.
    assert_eq!(bytes.len(), 128, "bra.s (2) + 62 nops (124) + rts (2)");
}
