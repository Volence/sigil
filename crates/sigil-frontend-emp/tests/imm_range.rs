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
    let module = lowered(src)?;
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

/// Lower `src` (no link): the module, or every ERROR message.
fn lowered(src: &str) -> Result<sigil_ir::Module, Vec<String>> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            roots: None,
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
    Ok(module)
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

/// The i32-wrap band (fix round 1): a value in `$80000000..=$FFFFFFFF` used to
/// narrow to a negative `i32` before the check and pass as its low bits. The
/// message names the value the author wrote, not the wrapped one.
#[test]
fn wrapped_32_bit_values_in_byte_and_word_slots_are_refused() {
    assert_eq!(
        err("move.w  #4294967295, d0"),
        ["operand 4294967295 out of range -32768..=65535"]
    );
    assert_eq!(err("move.w  #$FFFF8000, d0"), ["operand 4294934528 out of range -32768..=65535"]);
    assert_eq!(err("move.w  #$FFFF7FFF, d0"), ["operand 4294934527 out of range -32768..=65535"]);
    assert_eq!(err("move.b  #$FFFFFFFF, d0"), ["operand 4294967295 out of range -128..=255"]);
    assert_eq!(
        err("btst    #$FFFFFFFF, d0"),
        ["[lower.bit-number] bit number 4294967295 out of range 0..=31"]
    );
    assert_eq!(
        emp("module m in m\nconst W: u32 = $FFFF9EDE\npub proc f() {\n        move.w  #W, d0\n        rts\n}\n")
            .unwrap_err(),
        ["operand 4294942430 out of range -32768..=65535"]
    );
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
    assert_eq!(err("btst    #9, f(pc)"), ["[lower.bit-number] bit number 9 out of range 0..=7"]);
    assert_eq!(err("btst    #-1, d0"), ["[lower.bit-number] bit number -1 out of range 0..=31"]);
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

/// A const typed by a NEWTYPE over a primitive carries the primitive's range, as a bare
/// primitive does (G0b Task 9). Before, `newtype SfxId = u8` let `const X: SfxId = -1`
/// bind -1 silently (and 300 as well): demo's `SFXID_REV_LOOP = -1` was the first
/// negative `SfxId`, and its "feature off" meaning rode on a value its own type cannot
/// hold. In range it binds, so every shipped `SfxId` (sonic4's `$AB`) is unchanged.
#[test]
fn newtype_scalar_const_is_range_checked() {
    assert_eq!(
        err_mod("newtype SfxId = u8\nconst NEG: SfxId = -1"),
        ["[const.out-of-range] -1 does not fit SfxId (u8, 0..=255), in 'NEG'"]
    );
    assert_eq!(
        err_mod("newtype SfxId = u8\nconst BIG: SfxId = 300"),
        ["[const.out-of-range] 300 does not fit SfxId (u8, 0..=255), in 'BIG'"]
    );
    assert_eq!(
        err_mod("newtype Inner = i8\nnewtype Outer = Inner\nconst X: Outer = 200"),
        ["[const.out-of-range] 200 does not fit Outer (i8, -128..=127), in 'X'"]
    );
    emp("module m in m\nnewtype SfxId = u8\nconst OK: SfxId = $AB\nconst LO: SfxId = 0\n")
        .unwrap_or_else(|e| panic!("an in-range newtype const was refused: {e:?}"));
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
    // It IS the link-time path: the lowered fragment carries an `ImmWord16Be`
    // fixup at the imm field (offset 2), not a folded comptime value (which
    // would be in `.w` range only by the i32 wrap, and is now refused).
    let module = lowered(RAM_FIXTURE).unwrap_or_else(|e| panic!("RAM fixture refused: {e:?}"));
    let fixups: Vec<_> = module
        .sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .filter_map(|f| match f {
            sigil_ir::Fragment::Data(df) => Some(df.fixups.iter()),
            _ => None,
        })
        .flatten()
        .map(|fx| (fx.kind, fx.offset))
        .collect();
    assert_eq!(fixups, [(sigil_ir::FixupKind::ImmWord16Be, 2)]);
    let b = emp(RAM_FIXTURE).unwrap_or_else(|e| panic!("RAM fixture refused: {e:?}"));
    assert_eq!(b, [0x30, 0x3C, 0x9E, 0xDE, 0x4E, 0x75]);
}

/// Lower and link `src` (no expect on the link): the bytes, or every lowering
/// or LINK error message.
fn emp_link(src: &str) -> Result<Vec<u8>, Vec<String>> {
    let module = lowered(src)?;
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .map_err(|e| e.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    Ok(linked.sections.iter().flat_map(|s| s.bytes.clone()).collect())
}

/// Final review I1: a LINK-TIME `.w` immediate whose value has an all-ones
/// high half but bit 15 clear does not fit. `Lo` at `$FFFF0000` and `Hi` at
/// `$FFFF9C40` (`$9C40` = 40000), so `Lo - Hi` = −40000, which as a u32 is
/// `$FFFF63C0`. The old linker test (`(value as u32) >> 16` is 0 or `$FFFF`)
/// accepted it and wrote `303C 63C0`, i.e. `move.w #25536, d0`.
const BAND_FIXTURE: &str = "module m in m\n\
     region lo @ $FFFF0000 .. $FFFF0010\n\
     vars lo {\n\
         Lo: u16,\n\
     }\n\
     region hi @ $FFFF9C40 .. $FFFF9C50\n\
     vars hi {\n\
         Hi: u16,\n\
     }\n\
     pub proc f() {\n\
             move.w  #(Lo - Hi), d0\n\
             rts\n\
     }\n";

#[test]
fn link_time_word_immediate_refuses_a_negative_past_the_signed_word() {
    let errs = emp_link(BAND_FIXTURE).expect_err("Lo - Hi = -40000 must not assemble");
    assert!(
        errs.iter().any(|m| m.contains("does not fit a 16-bit immediate")),
        "{errs:?}"
    );
}

/// The same band reached by a RAM label below `$FFFF8000`: its word would
/// sign-extend to `$0000xxxx`, not to the label.
#[test]
fn link_time_word_immediate_refuses_a_ram_label_below_ffff8000() {
    let src = RAM_FIXTURE.replace("$FFFF9EDE", "$FFFF0100");
    let errs = emp_link(&src).expect_err("a $FFFF0100 label has no .w immediate spelling");
    assert!(
        errs.iter().any(|m| m.contains("does not fit a 16-bit immediate")),
        "{errs:?}"
    );
    // The legal side of the edge is untouched (globals Review Focus 1).
    assert_eq!(emp_link(RAM_FIXTURE).unwrap(), [0x30, 0x3C, 0x9E, 0xDE, 0x4E, 0x75]);
}

/// A6 finding (the front-end differential, Task 9): an immediate to `ccr` is a
/// BYTE field whatever the spelled size. Before this, `.emp` range-checked
/// `andi.w #65535, ccr` against the `.w` bounds and the encoder emitted the
/// byte form, `023C 00FF` (the value truncated); `.s` used the long bounds, so
/// `ori.s #256, ccr` assembled `003C 0000`. The AS front end refuses every
/// non-`.b` size, as asl does (`error #1130: invalid operand
/// size`, `frontend-as/src/eval.rs`, `andi_w_ccr`).
#[test]
fn ccr_immediates_are_byte_only() {
    const MSG: &str =
        "an immediate to `ccr` is byte only, as asl requires: write it with `.b`";
    assert_eq!(err("andi.w  #65535, ccr"), [MSG]);
    assert_eq!(err("andi.w  #1, ccr"), [MSG]);
    assert_eq!(err("andi.l  #-1, ccr"), [MSG]);
    assert_eq!(err("ori.w   #-32768, ccr"), [MSG]);
    assert_eq!(err("ori.s   #256, ccr"), [MSG]);
    assert_eq!(err("andi.s  #-129, ccr"), [MSG]);
    assert_eq!(ok("andi.b  #$FE, ccr"), [0x02, 0x3C, 0x00, 0xFE]);
    assert_eq!(ok("ori.b   #1, ccr"), [0x00, 0x3C, 0x00, 0x01]);
}
