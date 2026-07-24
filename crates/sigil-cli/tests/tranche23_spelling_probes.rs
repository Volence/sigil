//! Tranche 23 — step-0 spelling probes for the boot port, pinned as
//! permanent tests against the independent AS front-end.
//!
//! Each probe replicates the REAL site's binding class (2026-07-15 probe rule);
//! see `docs/superpowers/notes/2026-07-24-t23-step0-design.md` for the design
//! decisions each outcome feeds.
//!
//! - P1 `lea BootData(pc), a5` — cross-seam pc-rel where the target label sits
//!   immediately AFTER the .emp region (FORWARD displacement into the adjacent
//!   window; vdp_init.emp:27 proved the backward direction).
//! - P2 `move.w #Z80_SOUND_SIZE-1, d1` — link-time imm16 EXPRESSION over a bare
//!   cross-seam value symbol (the t22 P4 class at boot's real width/value).
//! - P3 `moveq #Z80_IDLE_SIZE-1, d1` — link-fed imm8 inside moveq. RULED
//!   demanded-feature at checkpoint (a) (overseer ruling 1) and SHIPPED:
//!   the dedicated SIGNED-8 `ImmSigned8` fixup (opcode-word low byte,
//!   window [-128, 127]); the gameBootHook mirror's
//!   `moveq #SONG_MOVINGTRUCKS, d0` rides the same path (hotkeys arm).
//! - P4 imm32 SYMBOL immediates with an absolute destination — the row-109
//!   shipped class at boot's two real sites (`move.l #VInt_Level, (VInt_Ptr).w`
//!   / `move.l #Game_Entry, (Game_State).w`) — plus the imm8 sibling
//!   `move.b #GAME_ENTRY_ID, (Game_State_ID).w` (link-fed byte immediate).
//!
//! (P5 = the mixed_tranche23 arm and P6 = the split-commit byte-neutrality
//! proof are full-scale gates, not synthetic probes — their artifacts live in
//! `mixed_dac_rom.rs` and the split commit's recorded dual-rebuild CRCs.)

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_span::Level;

/// Lower an `.emp` source into raw sections. Panics on lower ERRORS.
fn emp_sections(emp: &str, defines: &[(&str, i128)]) -> Vec<Section> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        !pdiags.iter().any(|d| d.level == Level::Error),
        "emp parse errors: {:?}",
        pdiags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        !ldiags.iter().any(|d| d.level == Level::Error),
        "emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).map(|d| &d.message).collect::<Vec<_>>()
    );
    module.sections
}

fn pin_at(mut secs: Vec<Section>, lma: u32) -> Vec<Section> {
    for sec in secs.iter_mut() {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    secs
}

/// A phased one-byte AS-side label carrier at an absolute VMA (the port-test
/// address-carrier technique) — the label's position is load-bearing.
fn addr_carrier(name: &str, vma: u32, lma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut secs = assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
        .sections;
    for s in secs.iter_mut() {
        s.lma = lma;
        s.placement = SectionPlacement::Pinned;
        s.group = None;
    }
    secs
}

fn link_flatten(sections: Vec<Section>) -> Vec<u8> {
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("probe resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &empty)
        .unwrap_or_else(|d| panic!("probe link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

// ---------------------------------------------------------------------------
// P1 — FORWARD cross-seam pc-rel: the target label sits immediately after the
// .emp region (boot.emp at $200 reaching BootData at the region's own end).
// vdp_init_port proved the backward direction; this pins the forward one at
// the real geometry (region base $200, target = first byte past the region).
// ---------------------------------------------------------------------------

const EMP_FWD_PCREL: &str = "\
module m in bootprobe
pub proc P () clobbers(a5) {
        lea     TailData(pc), a5
        rts
}
";

#[test]
fn forward_pcrel_lea_into_adjacent_window() {
    // .emp region [$200, $206); TailData carrier phased at $206 — the first
    // byte past the region, the boot/BootData adjacency exactly.
    let mut sections = pin_at(emp_sections(EMP_FWD_PCREL, &[]), 0x200);
    sections.extend(addr_carrier("TailData", 0x206, 0x206));
    let rom = link_flatten(sections);
    // lea d16(pc), a5 = 4BFA <disp>; disp = $206 - $202 = 4. rts = 4E75.
    assert_eq!(&rom[0x200..0x206], &[0x4B, 0xFA, 0x00, 0x04, 0x4E, 0x75], "forward pc-rel lea");
}

// ---------------------------------------------------------------------------
// P2 — link-time imm16 arithmetic over a bare cross-seam value symbol at
// boot's real value: `move.w #Z80_SOUND_SIZE-1, d1` (this build: $181C).
// ---------------------------------------------------------------------------

const EMP_LINKIMM_W: &str = "\
module m in bootprobe
pub proc P () clobbers(d1) {
        move.w  #Z80_SOUND_SIZE-1, d1
        rts
}
";

#[test]
fn linkimm_word_bare_arith_at_boot_value() {
    let mut sections = pin_at(emp_sections(EMP_LINKIMM_W, &[]), 0x400);
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[(
        "Z80_SOUND_SIZE",
        "$181C",
    )]));
    let rom = link_flatten(sections);
    // move.w #$181B, d1 = 323C 181B.
    assert_eq!(&rom[0x400..0x404], &[0x32, 0x3C, 0x18, 0x1B], "move.w #Z80_SOUND_SIZE-1");
}

// ---------------------------------------------------------------------------
// P3 — link-fed imm8 inside moveq: `moveq #Z80_IDLE_SIZE-1, d1` (the twin's
// sound-OFF arm; the gameBootHook mirror's `moveq #SONG_MOVINGTRUCKS` is the
// same class). RULED demanded-feature (overseer, checkpoint (a)): moveq's
// immediate embeds in the OPCODE WORD's low byte and sign-extends to 32
// bits, so it rides its own SIGNED-8 fixup (`ImmSigned8`, offset 1, window
// [-128, 127]) — deliberately DISTINCT from the unsigned `Value8` ext-word
// class (a shared [-128, 255] union would silently mis-assemble). The
// quick/shift opcode-embedded families stay refused (zero demand sites).
// ---------------------------------------------------------------------------

const EMP_MOVEQ_LINK: &str = "\
module m in bootprobe
pub proc P () clobbers(d1) {
        moveq   #Z80_IDLE_SIZE-1, d1
        rts
}
";

fn link_moveq_probe(idle_size: &str) -> Result<Vec<u8>, String> {
    let mut sections = pin_at(emp_sections(EMP_MOVEQ_LINK, &[]), 0x400);
    sections
        .extend(sigil_harness::test_support::assemble_equ_pairs(&[("Z80_IDLE_SIZE", idle_size)]));
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .map_err(|d| format!("resolve: {d:?}"))?;
    let linked = sigil_link::link(&resolved, &empty).map_err(|d| format!("{d:?}"))?;
    Ok(sigil_link::flatten(&linked, 0x00))
}

#[test]
fn moveq_link_folded_imm8_in_window_lowers() {
    // moveq #$25, d1 = 7225 (idle size $26 − 1); rts = 4E75.
    let rom = link_moveq_probe("$26").expect("in-window moveq link imm must link");
    assert_eq!(&rom[0x400..0x404], &[0x72, 0x25, 0x4E, 0x75], "moveq #Z80_IDLE_SIZE-1, d1");
}

#[test]
fn moveq_link_imm8_out_of_window_is_loud() {
    // 201 − 1 = 200 > 127: outside moveq's signed-8 window — the link must
    // FAIL naming the instruction-specific window, never wrap to a negative.
    let err = link_moveq_probe("201").expect_err("moveq #200 must fail the signed-8 range check");
    assert!(
        err.contains("moveq") && (err.contains("-128") || err.contains("signed")),
        "expected the moveq signed-8 window diagnostic, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// P4 — symbol immediates with absolute destinations, boot's three real sites:
//   (a) move.l #VInt_Level, (VInt_Ptr).w   — imm32 .emp-owned label source
//   (b) move.l #Game_Entry, (Game_State).w — imm32 game-side symbol source
//   (c) move.b #GAME_ENTRY_ID, (Game_State_ID).w — link-fed imm8 (move.b has
//       a real immediate extension word, unlike moveq)
// Sources are link-fed; destinations are the RAM cells' explicit `.w`
// spelling (row-1046 item 2: a link-imm source keeps the explicit dest width).
// ---------------------------------------------------------------------------

const EMP_IMM32_ABS: &str = "\
module m in bootprobe
pub proc P () clobbers() {
        move.l  #VInt_Level, (VInt_Ptr).w
        move.l  #Game_Entry, (Game_State).w
        move.b  #GAME_ENTRY_ID, (Game_State_ID).w
        rts
}
";

#[test]
fn imm32_and_imm8_symbol_sources_with_absw_dest() {
    let mut sections = pin_at(emp_sections(EMP_IMM32_ABS, &[]), 0x400);
    // VInt_Level: a ROM label carrier at its address class; the rest are
    // equ-fed (game-side equ/RAM symbols resolve identically at link).
    sections.extend(addr_carrier("VInt_Level", 0x1C46, 0x1C46));
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[
        ("VInt_Ptr", "$FFFF8036"),
        ("Game_Entry", "$34F2"),
        ("Game_State", "$FFFF8004"),
        ("GAME_ENTRY_ID", "5"),
        ("Game_State_ID", "$FFFF8008"),
    ]));
    let rom = link_flatten(sections);
    // move.l #$1C46, (VInt_Ptr).w   = 21FC 0000 1C46 8036
    assert_eq!(
        &rom[0x400..0x408],
        &[0x21, 0xFC, 0x00, 0x00, 0x1C, 0x46, 0x80, 0x36],
        "move.l #VInt_Level, (VInt_Ptr).w"
    );
    // move.l #$34F2, (Game_State).w = 21FC 0000 34F2 8004
    assert_eq!(
        &rom[0x408..0x410],
        &[0x21, 0xFC, 0x00, 0x00, 0x34, 0xF2, 0x80, 0x04],
        "move.l #Game_Entry, (Game_State).w"
    );
    // move.b #5, (Game_State_ID).w  = 11FC 0005 8008
    assert_eq!(
        &rom[0x410..0x416],
        &[0x11, 0xFC, 0x00, 0x05, 0x80, 0x08],
        "move.b #GAME_ENTRY_ID, (Game_State_ID).w"
    );
}

// ---------------------------------------------------------------------------
// P7 (found at the boot step-1 compile, demanded-feature law) — movem with a
// width-PINNED symbolic absolute memory EA: `movem.l (RAM_Start).w, d0-a6`
// (boot's regs-from-cleared-RAM zeroing). movem lowers on its own path, so
// the general abs-sym seam never saw it. The pinned `.w` spelling ships; the
// RELAXABLE bare-name form for movem stays a ledgered ask.
// ---------------------------------------------------------------------------

const EMP_MOVEM_ABS: &str = "\
module m in bootprobe
pub proc P () clobbers(d0-d7/a0-a6) {
        movem.l (RAM_Start).w, d0-a6
        rts
}
";

#[test]
fn movem_pinned_absw_source_lowers() {
    let mut sections = pin_at(emp_sections(EMP_MOVEM_ABS, &[]), 0x400);
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[(
        "RAM_Start",
        "$FFFF8000",
    )]));
    let rom = link_flatten(sections);
    // movem.l (xxx).w, d0-a6 = 4CF8 7FFF 8000 (mask excludes a7); rts = 4E75.
    assert_eq!(
        &rom[0x400..0x408],
        &[0x4C, 0xF8, 0x7F, 0xFF, 0x80, 0x00, 0x4E, 0x75],
        "movem.l (RAM_Start).w, d0-a6"
    );
}

/// The `.b` deferral's soundness half: a link value outside the unsigned
/// 8-bit window must FAIL the link (`Value8` range check), never truncate.
#[test]
fn imm8_link_value_out_of_range_is_loud() {
    const EMP: &str = "\
module m in bootprobe
pub proc P () clobbers() {
        move.b  #GAME_ENTRY_ID, (Game_State_ID).w
        rts
}
";
    let mut sections = pin_at(emp_sections(EMP, &[]), 0x400);
    sections.extend(sigil_harness::test_support::assemble_equ_pairs(&[
        ("GAME_ENTRY_ID", "$1F5"),
        ("Game_State_ID", "$FFFF8008"),
    ]));
    let empty = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&sections, &empty, true)
        .unwrap_or_else(|d| panic!("resolve failed: {d:?}"));
    let err = sigil_link::link(&resolved, &empty)
        .expect_err("a 9-bit value in a .b link immediate must fail the Value8 range check");
    let msg = format!("{err:?}");
    assert!(
        msg.to_lowercase().contains("range") || msg.contains("Value8") || msg.contains("8-bit"),
        "expected a range-check diagnostic, got: {msg}"
    );
}
