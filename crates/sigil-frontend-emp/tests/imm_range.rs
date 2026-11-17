//! A1 (step 0 soundness): a sized 68k immediate, a bit number and a typed
//! scalar `const` that do not fit are REFUSED with a message, never truncated.
//!
//! Before this gate the `.emp` front end narrowed every immediate to its field
//! silently: `move.w #$12345, d0` assembled to `303C 2345`, `btst #40, d0` to
//! `0800 0028` (the AS front end refuses the former with the same
//! `operand N out of range lo..=hi` text asserted here). The bounds are the AS
//! front end's `m68k_imm_bounds` (signed OR bit-pattern unsigned spelling), so a
//! boundary spelling and a complemented byte mask still assemble, and the
//! link-time `ImmLink` path (a RAM label in a `.w` immediate) is untouched.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

/// Lower and link `src`: the linked bytes, or every ERROR message (no location).
fn emp(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<String> =
        diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect();
    if !errs.is_empty() {
        return Err(errs);
    }
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

fn one(inst: &str) -> String {
    format!("module m in m\npub proc f() {{\n        {inst}\n        rts\n}}\n")
}

/// The error messages one instruction provokes; panics if it assembles.
fn err(inst: &str) -> Vec<String> {
    match emp(&one(inst)) {
        Ok(b) => panic!("`{inst}` assembled to {b:02X?}; expected a refusal"),
        Err(e) => e,
    }
}

/// The bytes of one instruction (the trailing `rts` stripped).
fn ok(inst: &str) -> Vec<u8> {
    let mut b = emp(&one(inst)).unwrap_or_else(|e| panic!("`{inst}` refused: {e:?}"));
    assert_eq!(b.split_off(b.len() - 2), [0x4E, 0x75], "trailing rts");
    b
}

/// The error messages a whole module provokes; panics if it lowers clean.
fn err_mod(item: &str) -> Vec<String> {
    match emp(&format!("module m in m\n{item}\n")) {
        Ok(b) => panic!("`{item}` lowered clean ({b:02X?}); expected a refusal"),
        Err(e) => e,
    }
}

#[test]
fn word_immediate_past_range_is_refused() {
    assert_eq!(err("move.w  #$12345, d0"), ["operand 74565 out of range -32768..=65535"]);
}

#[test]
fn byte_immediate_past_range_is_refused() {
    assert_eq!(err("move.b  #300, d1"), ["operand 300 out of range -128..=255"]);
    assert_eq!(err("andi.b  #$1FF, d0"), ["operand 511 out of range -128..=255"]);
    assert_eq!(err("cmpi.b  #300, d0"), ["operand 300 out of range -128..=255"]);
    assert_eq!(err("move.b  #-129, d2"), ["operand -129 out of range -128..=255"]);
    assert_eq!(err("andi.b  #~$80, d0"), ["operand -129 out of range -128..=255"]);
    assert_eq!(err("andi.b  #$1FF, ccr"), ["operand 511 out of range -128..=255"]);
}

#[test]
fn sr_and_memory_destinations_are_checked() {
    assert_eq!(err("move.w  #$12345, sr"), ["operand 74565 out of range -32768..=65535"]);
    assert_eq!(err("cmpi.w  #$12345, (a0)"), ["operand 74565 out of range -32768..=65535"]);
    assert_eq!(err("addi.w  #-32769, d0"), ["operand -32769 out of range -32768..=65535"]);
}

#[test]
fn long_message_matches_the_as_front_end() {
    assert_eq!(
        err("move.l  #$123456789, d0"),
        ["operand 4886718345 out of range -2147483648..=4294967295"]
    );
}

#[test]
fn boundary_spellings_assemble() {
    assert_eq!(ok("move.b  #-128, d0"), [0x10, 0x3C, 0x00, 0x80]);
    assert_eq!(ok("move.b  #255, d0"), [0x10, 0x3C, 0x00, 0xFF]);
    assert_eq!(ok("move.w  #-32768, d0"), [0x30, 0x3C, 0x80, 0x00]);
    assert_eq!(ok("move.w  #$FFFF, d0"), [0x30, 0x3C, 0xFF, 0xFF]);
    assert_eq!(ok("andi.b  #~$7F, d0"), [0x02, 0x00, 0x00, 0x80]);
}

#[test]
fn bit_numbers_are_checked() {
    assert_eq!(err("btst    #40, d0"), ["[lower.bit-number] bit number 40 out of range 0..=31"]);
    assert_eq!(err("bset    #9, (a0)"), ["[lower.bit-number] bit number 9 out of range 0..=7"]);
    assert_eq!(ok("btst    #31, d0"), [0x08, 0x00, 0x00, 0x1F]);
    assert_eq!(ok("bset    #7, (a0)"), [0x08, 0xD0, 0x00, 0x07]);
}

#[test]
fn typed_const_out_of_range_is_refused() {
    assert_eq!(
        err_mod("const LIMIT: u8 = 300"),
        ["[const.out-of-range] 300 does not fit u8 (0..=255), in 'LIMIT'"]
    );
    assert_eq!(
        err_mod("const NEG: u8 = -1"),
        ["[const.out-of-range] -1 does not fit u8 (0..=255), in 'NEG'"]
    );
    assert_eq!(
        err_mod("const SMALL: i8 = 200"),
        ["[const.out-of-range] 200 does not fit i8 (-128..=127), in 'SMALL'"]
    );
}

/// A `vars` label at `$FFFF9EDE` in a `.w` immediate is a LINK-TIME immediate
/// (`ImmLink` → `ImmWord16Be`, which admits the sign-extended high half). A1's
/// check must not reach it (globals Review Focus 1).
const RAM_FIXTURE: &str = "module m in m\n\
     region r @ $FFFF9EDE .. $FFFFFF00\n\
     vars r {\n\
         RamX: u16,\n\
     }\n\
     pub proc f() {\n\
             move.w  #RamX, d0\n\
             rts\n\
     }\n";

#[test]
fn link_time_word_immediate_still_accepts_a_ram_address() {
    let b = emp(RAM_FIXTURE).unwrap_or_else(|e| panic!("RAM fixture refused: {e:?}"));
    assert_eq!(b, [0x30, 0x3C, 0x9E, 0xDE, 0x4E, 0x75]);
}
