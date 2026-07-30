//! sigil-harness — reference-build helpers shared by the strict gates and the CLI.
//!
//! ## History (M1.D T6)
//!
//! This crate once drove an M0 "bounded harness": it assembled the Z80 sound
//! driver's Region A + Region B *in isolation* (`harness_root.asm`), stubbing the
//! ~42 68k leaf symbols the driver referenced but that the isolated build did not
//! define (`golden/stub-syms.toml`, re-derived by the `regen` bin). That
//! scaffolding existed only because Sigil could not yet assemble the whole 68k
//! ROM.
//!
//! It now can. The `m1d_rom` gate proves the full non-debug `main.asm` assembles
//! BYTE-EXACT to the reference with **zero stubs**, and `m0_regions` proves the
//! sound driver's Region A + Region B fall out of that full build byte-exact. So
//! the bounded harness, its stub table, and `regen` were all retired, leaving a
//! single reference-build entry point: "assemble the full non-debug ROM".

use std::path::Path;

/// Shared test-support: the AS-truth equ blob for the `engine.constants` twin
/// and the drift-guard filter, consolidated out of ~9 hand-copied port/probe
/// test files. Reachable by both the CLI tests and this crate's own tests
/// (both depend on `sigil-harness`).
pub mod test_support;

/// The `repin` pin generator (tranche-10 step 0): listing parsing, the
/// `repin.toml` manifest, and the `pins.rs` renderer. Driven by the `repin`
/// bin and the `repin_pins` staleness test.
pub mod repin;

/// GENERATED layout pins (regions/symbols/offsets in both build shapes) —
/// the single source the port tests import. Regenerate with
/// `cargo run -p sigil-harness --bin repin`; never edit by hand.
pub mod pins;

/// The seam-1 resident-sound-blob native link: the five sound `.emp` files linked
/// as ONE Z80 module set, shared by the whole-ROM gates and the `emit_sound_blob`
/// bin (Option A — sigil emits the canonical build inputs asl packs).
pub mod seam1;

pub mod seam2;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, Module, SymbolTable};
use sigil_link::LinkedImage;

/// Region A base LMA in the assembled ROM: the resident phase-0 Z80 driver.
/// Provenance: the retired `golden/windows.toml`, `regen`-derived from the
/// bracketing 68k anchor label `Z80_Sound_Start`. Slid `0x3EA → 0x3E0` in
/// phase2.5 c5 (the dead Cold_Boot CROSS_RESET store removed upstream −0xA),
/// then `0x3E0 → 0x3DE` in the t23 boot clr.w wave (−2 upstream; the Z80
/// blob content is byte-identical, only its ROM position shifts).
pub const REGION_A_LMA: u32 = 0x3DE;
/// Region B base LMA: the phase-`08000h` Moving-Trucks / SFX engine-table bank.
/// Provenance: `MovingTrucks_Bank_Start`. Slid `0x60000 → 0x58000` in the
/// 2026-07-21 Deep-Forest-BG re-baseline (the OJZ BG art ahead of it shrank).
pub const REGION_B_LMA: u32 = 0x58000;

/// Assemble the full non-debug Aeon ROM from `<aeon>/games/sonic4/main.asm` and
/// link it, with **no stubs** — the full include tree defines everything. Mirrors
/// `build.sh`'s default ASFLAGS: `SOUND_DRIVER_ENABLED` on, `__DEBUG__` off.
///
/// Returns the linked image (each section carries name / LMA / bytes); call
/// [`sigil_link::emit_rom`] on it for a flat ROM. This is the one reference-build
/// entry point shared by the CLI and the region gates.
pub fn assemble_full_rom(aeon: &Path) -> Result<LinkedImage, String> {
    assemble_full_rom_with(aeon, false)
}

/// Assemble the full **`__DEBUG__`** Aeon ROM (`DEBUG=1 ./build.sh`): everything
/// `assemble_full_rom` does, plus `__DEBUG__` defined, which pulls in
/// `debugger.asm`'s assertion / KDebug / `__FSTRING` error-message code. Used by
/// the `m1d_debug_rom` gate (A2).
pub fn assemble_full_rom_debug(aeon: &Path) -> Result<LinkedImage, String> {
    assemble_full_rom_with(aeon, true)
}

/// Shared body of the two entry points above. `debug` toggles the `__DEBUG__`
/// define; `SOUND_DRIVER_ENABLED` is always on (build.sh's default), no stubs.
fn assemble_full_rom_with(aeon: &Path, debug: bool) -> Result<LinkedImage, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    let module = assemble_root(&root, &opts)
        .map_err(|d| format!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &stubs, true)
        .map_err(|d| format!("resolve_layout: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("link: {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Assemble the AS side of the MIXED `.asm`+`.emp` build: everything
/// `assemble_full_rom` does (SOUND_DRIVER_ENABLED on, no stubs), PLUS
/// `SIGIL_EMP_DAC` defined so `main.asm`'s `gameSoundDataIncludes` macro SKIPS
/// `dac_samples.asm` and `org $60000` resumes placement for the Moving-Trucks
/// bank (leaving the $50000/$58000 DAC banks for the `.emp` side to supply).
/// `debug` toggles `__DEBUG__` exactly as the two `assemble_full_rom*` entry
/// points do — the mixed harness proves BOTH debug shapes compose.
///
/// Returns the UNLINKED [`Module`] (raw sections), not a `LinkedImage`: the
/// mixed harness concatenates these with the `.emp` module's placed sections and
/// runs ONE `resolve_layout` + `link` over the union, so the cross-seam symbols
/// (`SND_*_BANK/PTR/LEN` etc.) resolve through a single shared symbol table.
pub fn assemble_mixed_dac_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        // `asl`'s `ifndef` tests symbol EXISTENCE, so any value works; 1 mirrors
        // the other `-D` defines. This is the gate that flips main.asm's
        // dac_samples.asm include to `org $60000`.
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts)
        .map_err(|d| format!("assemble (mixed AS side): {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Assemble the AS side of the T2 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_dac_as_side` does, PLUS `SIGIL_EMP_MT` defined so
/// `main.asm`'s Moving-Trucks block (lines 150-208: the six streaming-bank
/// includes + the pitch-contiguity fatal) is REPLACED by an `org` resume — per
/// shape, `$5D53A` (`__DEBUG__`) or `$5BAE8` (plain) — leaving the whole
/// `$58607..end` window for the `.emp` side's `mt_bank` section to supply.
/// Both `SIGIL_EMP_DAC` and `SIGIL_EMP_MT` are independent gates (R6); T2's
/// mixed build exercises both ON together, DAC-only stays covered by the
/// unchanged `assemble_mixed_dac_as_side` T1 tests.
///
/// Returns the UNLINKED [`Module`], exactly like `assemble_mixed_dac_as_side`:
/// the T2 mixed harness concatenates these sections with BOTH `.emp` modules'
/// placed sections (`dac_samples.emp` + `mt_bank.emp`) and runs ONE
/// `resolve_layout` + `link` over the union, so every cross-seam symbol
/// (including `movea.l #SongTable`/`#SongPatchTable` in `sound_api.asm`,
/// deferred by Task 3's imm32 fixup) resolves through a single shared table.
pub fn assemble_mixed_mt_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed MT AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the T3 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_mt_as_side` does, PLUS `SIGIL_EMP_SFX` defined so
/// `main.asm`'s SFX block (the 19 blob/patch/table includes + the two SFX
/// fatals, R6) is REPLACED by an `org` resume — per shape, `$5DC82`
/// (`__DEBUG__`) or `$5C230` (plain), i.e. `SfxTable_End` — leaving the whole
/// `$5BAE8..SfxTable_End` window for the `.emp` side's `sfx_bank` section to
/// supply. All three gates (`SIGIL_EMP_DAC`, `SIGIL_EMP_MT`, `SIGIL_EMP_SFX`)
/// are independent (R6); T3's mixed build exercises all three ON together.
///
/// Returns the UNLINKED [`Module`], exactly like the two sibling helpers: the
/// T3 mixed harness concatenates these sections with all THREE `.emp` modules'
/// placed sections (`dac_samples.emp` + `mt_bank.emp` + `sfx_bank.emp`) and
/// runs ONE `resolve_layout` + `link` over the union. The cross-seam reads
/// unique to this tranche are the `soundBankHead` win-tab's nine
/// `dw sfx_winptr(Sfx_NN)` entries (a compound `(Sfx_NN & $7FFF) | $8000` in a
/// Z80 `phase 08000h` LE `dw`): with `SIGIL_EMP_SFX` on the `Sfx_NN` labels are
/// `.emp`-side, so those entries assemble here with the target UNRESOLVED (T0's
/// dw deferral, P1-proven) and are satisfied by the joint link against
/// `sfx_bank.emp`'s labels through the same shared symbol table everything else
/// uses.
pub fn assemble_mixed_sfx_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed SFX AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the port #1 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_sfx_as_side` does, PLUS `SIGIL_EMP_HBLANK` defined so
/// `engine/engine.inc:92`'s `ifndef SIGIL_EMP_HBLANK` block (which normally
/// includes `engine/system/hblank.asm`) is REPLACED by an `org` resume — per
/// shape, `$228C` (plain) or `$231A` (`__DEBUG__`) — leaving the 18-byte
/// `$227A..$228C` / `$2308..$231A` window for the `.emp` side's `hblank`
/// section to supply. All FOUR gates (`SIGIL_EMP_DAC`, `SIGIL_EMP_MT`,
/// `SIGIL_EMP_SFX`, `SIGIL_EMP_HBLANK`) are independent; this is the
/// cumulative shape exercising all four together — the campaign's first CODE
/// port riding on top of the three sound-migration data ports.
///
/// `HBlank_Handler_Ptr` (referenced by `hblank.emp`'s `HBlank_Dispatch`) is a
/// real `.asm` RAM label defined UNCONDITIONALLY in `engine/ram.asm` (outside
/// the gate) — so, like `MovingTrucks_Bank_Start`, no synthetic cross-seam
/// symbol injection is needed here: the real AS module supplies it through
/// the same shared symbol table. `vectors.asm`'s `dc.l HBlank_Dispatch` and
/// `boot.asm`'s `move.l #HBlank_Null, (HBlank_Handler_Ptr).w` are likewise
/// unconditional AS-side consumers of the `.emp` module's two `pub proc`
/// names — the latter is only assemblable at all because of the
/// `try_defer_long_imm` extension (port #1 T3) that lets a `move.l #imm,
/// (abs).w` with an unresolved source immediate defer to a `Value32Be` link
/// fixup, mirroring the register-destination deferral R3 already proved for
/// `movea.l #SongTable, a0`.
///
/// Returns the UNLINKED [`Module`], exactly like the three sibling helpers:
/// the port #1 mixed harness concatenates these sections with all FOUR
/// `.emp` modules' placed sections (`dac_samples.emp` + `mt_bank.emp` +
/// `sfx_bank.emp` + `hblank.emp`) and runs ONE `resolve_layout` + `link` over
/// the union.
pub fn assemble_mixed_hblank_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed HBLANK AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the port #2 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_hblank_as_side` does, PLUS `SIGIL_EMP_CONTROLLERS` and
/// `SIGIL_EMP_MATH` defined so `engine/engine.inc`'s two `ifndef` blocks
/// (which normally include `engine/system/controllers.asm` /
/// `engine/system/math.asm`) are each REPLACED by an `org` resume — per
/// shape, controllers `$22FE` (plain) / `$238C` (`__DEBUG__`), math `$26FC`
/// (plain) / `$288E` (`__DEBUG__`) — leaving the two windows for the `.emp`
/// side's `controllers`/`math` sections to supply. All SIX gates
/// (`SIGIL_EMP_DAC`, `SIGIL_EMP_MT`, `SIGIL_EMP_SFX`, `SIGIL_EMP_HBLANK`,
/// `SIGIL_EMP_CONTROLLERS`, `SIGIL_EMP_MATH`) are independent; this is the
/// cumulative shape exercising all six together — port #2 riding on top of
/// port #1 and the three sound-migration data ports.
///
/// `HW_PORT_1_DATA`/`HW_PORT_2_DATA` (equs, `engine/constants.asm`) and
/// `Ctrl_1_Held`/`Ctrl_2_Held`/`Ctrl_1_Press_Accum`/`Ctrl_2_Press_Accum` (RAM
/// labels, `engine/ram.asm`) — read by `controllers.emp`'s
/// `Read_Controllers` — are real `.asm` symbols defined UNCONDITIONALLY
/// (outside every gate), so no synthetic cross-seam symbol injection is
/// needed here: the real AS module supplies them through the same shared
/// symbol table. `vblank.asm`'s two `bsr.w Read_Controllers` sites and
/// `test_parent.asm`/`player_ground.asm`'s six `jsr GetSineCosine` sites are
/// likewise unconditional AS-side consumers of the two `.emp` modules' `pub
/// proc` names — the `jsr` sites are only assemblable at all because of the
/// `Fragment::JmpJsrSym` deferral (port #2 follow-up) that lets a bare `jsr
/// Sym`/`jmp Sym` whose target is genuinely unresolved within the AS compile
/// defer to a linker-resolved fixup, mirroring the `.emp` front-end's
/// `jbra`/`jbsr` ladder.
///
/// Returns the UNLINKED [`Module`], exactly like the four sibling helpers:
/// the port #2 mixed harness concatenates these sections with all SIX
/// `.emp` modules' placed sections (`dac_samples.emp` + `mt_bank.emp` +
/// `sfx_bank.emp` + `hblank.emp` + `controllers.emp` + `math.emp`) and runs
/// ONE `resolve_layout` + `link` over the union.
pub fn assemble_mixed_tranche2_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche2 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 3's cumulative shape: everything `assemble_mixed_tranche2_as_side`
/// does, PLUS `SIGIL_EMP_VDP_INIT` and `SIGIL_EMP_COLLISION_LOOKUP` defined
/// so `engine/engine.inc`'s two new `ifndef` blocks (which normally include
/// `engine/system/vdp_init.asm` / `engine/level/collision_lookup.asm`) are
/// each REPLACED by an `org` resume — per shape, vdp_init `$1C5C` (plain) /
/// `$1CDE` (`__DEBUG__`), collision_lookup `$4C38` (plain) / `$545C`
/// (`__DEBUG__`) — leaving the two windows for the `.emp` side's
/// `vdp_init`/`collision_lookup` sections to supply. All EIGHT gates are
/// independent; this is the cumulative shape exercising all eight together.
///
/// The cross-seam symbols the two new `.emp` modules read — `VDP_CTRL` (equ),
/// `VDP_Shadow_Table`/`VDP_Dirty_Mask`/`Cache_*` (RAM labels),
/// `BootData_VDPRegs`/`Tile_Cache_GetCollision` (ROM labels, PC-RELATIVE
/// targets) — are real `.asm` symbols defined UNCONDITIONALLY (outside every
/// gate), so no synthetic injection is needed: the real AS module supplies
/// them through the shared symbol table, including the two pc-relative
/// targets at their true per-shape VMAs (the first cross-seam PC-RELATIVE
/// consumers in the campaign — the fixup is a distance, so the supplied
/// positions are load-bearing in a way the abs-widthed reads never were).
/// `boot.asm`'s `bsr.w VDP_Shadow_Init`, the VBlank path's
/// `Flush_VDP_Shadow` call, and `player_sensors.asm`'s
/// `bsr.w Collision_GetType` sites are the unconditional AS-side consumers
/// of the new `pub proc` names.
///
/// Returns the UNLINKED [`Module`], exactly like the sibling helpers: the
/// tranche-3 mixed harness concatenates these sections with all EIGHT `.emp`
/// modules' placed sections and runs ONE `resolve_layout` + `link` over the
/// union.
pub fn assemble_mixed_tranche3_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche3 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the tranche-4 NINE-module mixed build: everything
/// `assemble_mixed_tranche3_as_side` gates PLUS `SIGIL_EMP_PARTICLE_ANIMS`
/// (the campaign's first GAME-DATA gate — `games/sonic4/main.asm`'s include
/// site, past `org $10000`, so the resume org lives in main.asm rather than
/// engine.inc).
pub fn assemble_mixed_tranche4_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche4 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the tranche-5 THIRTEEN-module mixed build:
/// everything `assemble_mixed_tranche4_as_side` gates PLUS tranche 5's two:
///
/// - `SIGIL_EMP_GAME_LOOP` — the `engine/engine.inc:136` gate (engine-side,
///   like controllers/math/collision_lookup). The window it opens
///   (plain `$22FE..$2310`, debug `$238C..$239E`) is filled by
///   `engine/system/game_loop.emp`, whose body takes the
///   `SOUND_DRIVER_ENABLED`/`SOUND_DEBUG_HOTKEYS` defines (tranche-5 H1/H2 —
///   the first CODE module with build-shape conditionals).
/// - `SIGIL_EMP_SOUND_API` — the gate INSIDE engine.inc's
///   `ifdef SOUND_DRIVER_ENABLED` block (plain `$5D94..$5F7C`, debug
///   `$7252..$743A`), filled by `engine/sound/sound_api.emp`. Its slot
///   addresses are extern-equ sums over AS-owned equs, its `SongTable`/
///   `SongPatchTable` reads are LINK-TIME imm32s — and those two symbols are
///   .emp-side under `SIGIL_EMP_MT`, so the mixed build exercises
///   .emp-defines/.emp-consumes through the shared link.
pub fn assemble_mixed_tranche5_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
        ("SIGIL_EMP_GAME_LOOP".to_string(), 1),
        ("SIGIL_EMP_SOUND_API".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche5 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the tranche-6 FIFTEEN-module mixed build:
/// everything `assemble_mixed_tranche5_as_side` gates PLUS
/// `SIGIL_EMP_TEST_OBJECTS` — ONE gate covering TWO modules (a first:
/// `games/sonic4/main.asm:43` wraps the `test_solid.asm` +
/// `test_particle.asm` includes together, else-arm `org $10FDC`). The
/// campaign's first GAME-CODE gate, inside the object code bank
/// (`org $10000`, ObjCodeBase): the window it opens (`$10F7C..$10FDC`) is
/// SHAPE-INVARIANT — the bank's contents up to here don't move with
/// `__DEBUG__`, so one org serves both shapes; only the cross-seam
/// engine/data targets (`Draw_Sprite`/`ObjectMove`/`AnimateSprite` abs.w,
/// `Ani_Particle` imm32) take per-shape values. `ObjDef_Solid`'s
/// `dc.w objroutine(TestSolid_Init)` word (`data/objdefs/test_objects.asm`)
/// and the emitters' `objroutine(TestParticle)` spawn words are the
/// unconditional AS-side consumers of the new `pub proc` names — the
/// outbound direction, `.w` differences against ObjCodeBase.
pub fn assemble_mixed_tranche6_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
        ("SIGIL_EMP_GAME_LOOP".to_string(), 1),
        ("SIGIL_EMP_SOUND_API".to_string(), 1),
        ("SIGIL_EMP_TEST_OBJECTS".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche6 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the tranche-7 SIXTEEN-module mixed build: everything
/// `assemble_mixed_tranche6_as_side` gates PLUS `SIGIL_EMP_COLLISION` — the
/// `engine/engine.inc` gate wrapping the `engine/objects/collision.asm` include
/// (else-arm `org $3070` plain / `org $332A` debug). Back in the ENGINE block
/// (like game_loop/collision_lookup), the window it opens (`$2F0A..$3070` plain
/// / `$31C4..$332A` debug) is filled by `engine/objects/collision.emp` — whose
/// `TouchResponse` is the sole `pub proc` export (called from the engine object
/// manager). The module reads only GAME-RAM `Player_1`/`Dynamic_Slots` across
/// the seam (abs.w, per-shape); its dispatch is a self-contained module-level
/// handler table (pc-indexed `jsr`), so no ROM cross-seam target moves.
pub fn assemble_mixed_tranche7_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
        ("SIGIL_EMP_GAME_LOOP".to_string(), 1),
        ("SIGIL_EMP_SOUND_API".to_string(), 1),
        ("SIGIL_EMP_TEST_OBJECTS".to_string(), 1),
        ("SIGIL_EMP_COLLISION".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche7 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// `assemble_mixed_tranche7_as_side` gates PLUS `SIGIL_EMP_RINGS` — the
/// `engine/engine.inc` gate wrapping the `engine/objects/rings.asm` include
/// (else-arm `org $33A8` plain / `org $36BE` debug). The window it opens
/// (`$3070..$33A8` plain / `$332A..$36BE` debug) is filled by
/// `engine/objects/rings.emp` — the campaign's FIRST shape-dependent-LENGTH
/// region (the `__DEBUG__` assert block exists only in the debug shape), so
/// the two orgs differ by more than the usual base slide. All five procs are
/// `pub` exports (entity_window/sprites/game states call them); the module
/// reads seven `Ring_*` RAM cells + `Camera_X/Y` + `Player_1` across the seam
/// (abs.w, per-shape), calls four ROM procs (`Collected_MarkRing`,
/// `EntityWindow_EntryForSection`, `EntityLoaded_Clear`, `Sound_PlayRing`),
/// and in the debug shape jumps into the two `MDDBG__ErrorHandler*` entries.
pub fn assemble_mixed_tranche8_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
        ("SIGIL_EMP_GAME_LOOP".to_string(), 1),
        ("SIGIL_EMP_SOUND_API".to_string(), 1),
        ("SIGIL_EMP_TEST_OBJECTS".to_string(), 1),
        ("SIGIL_EMP_COLLISION".to_string(), 1),
        ("SIGIL_EMP_RINGS".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche8 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// `assemble_mixed_tranche8_as_side` gates PLUS `SIGIL_EMP_ANIMATE` — the
/// `engine/engine.inc` gate wrapping the `engine/objects/animate.asm` include
/// (else-arm `org $2F0A` plain / `org $31C4` debug). The window it opens
/// (`$2D78..$2F0A` plain / `$3032..$31C4` debug — 0x192 bytes, length
/// shape-INVARIANT: no `__DEBUG__` code) is filled by
/// `engine/objects/animate.emp`. animate sits UPSTREAM of every other gated
/// engine region, so its gate orgs are the first in the ladder's sliding
/// window. Both procs are `pub` exports (player_common + the test
/// objects `jsr AnimateSprite` across the seam — bare jsr relaxing to
/// abs.w); the module reads NO RAM cells and calls two ROM procs
/// (`DeleteObject` via bare abs.w `jmp`, `Sound_PlaySFX` under
/// SOUND_DRIVER_ENABLED). The AF_* control-code equs script data files read
/// were re-homed to `engine/constants.asm` at this port, so they survive
/// the gate on the AS side.
pub fn assemble_mixed_tranche9_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        // The DSM mixed harness composes the DAC banks IN-MEMORY as dac_samples.emp
        // sections (org-stub body); the real build + seam-2 whole-ROM gate BINCLUDE
        // the emitted .bins. Both paths produce the byte-identical ROM. The head is
        // AS-BINCLUDE'd in BOTH (it can't be stubbed — the MT song must land past it).
        ("SIGIL_EMP_DAC_BODY_STUB".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
        ("SIGIL_EMP_HBLANK".to_string(), 1),
        ("SIGIL_EMP_CONTROLLERS".to_string(), 1),
        ("SIGIL_EMP_MATH".to_string(), 1),
        ("SIGIL_EMP_VDP_INIT".to_string(), 1),
        ("SIGIL_EMP_COLLISION_LOOKUP".to_string(), 1),
        ("SIGIL_EMP_PARTICLE_ANIMS".to_string(), 1),
        ("SIGIL_EMP_SONIC_ANIMS".to_string(), 1),
        ("SIGIL_EMP_ACT_DESCRIPTOR".to_string(), 1),
        ("SIGIL_EMP_GAME_LOOP".to_string(), 1),
        ("SIGIL_EMP_SOUND_API".to_string(), 1),
        ("SIGIL_EMP_TEST_OBJECTS".to_string(), 1),
        ("SIGIL_EMP_COLLISION".to_string(), 1),
        ("SIGIL_EMP_RINGS".to_string(), 1),
        ("SIGIL_EMP_ANIMATE".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche9 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}

/// The Genesis header's checksum word (`$18E..$190`) — a SEMANTIC header
/// fact, not a layout pin. One of the only two fields the out-of-scope
/// `convsym -a`/`fixheader` post-steps rewrite (`convsym -a` appends the
/// MD-Debugger `deb2` symbol table; `fixheader` re-checksums the appended
/// image — M1.B models `convsym` as a no-op, so Sigil's `emit_rom` target is
/// the pre-append ASSEMBLED ROM).
pub const CHECKSUM_FIELD_RANGE: std::ops::Range<usize> = 0x18E..0x190;
/// The header's `dc.l EndOfRom-1` ROM-end pointer (`$1A4..$1A8`) — the other
/// convsym-rewritten field (bumped to the POST-append end). Which of its four
/// bytes actually differ shifts whenever the deb2 append changes size (the
/// tranche-9 PerFrame deletion flipped `$1A5` back), which is why the
/// rewritten set is DERIVED per comparison rather than pinned — see
/// [`derive_convsym_rewritten`] (tranche-10 step 0, D-T10.6).
pub const ROM_END_FIELD_RANGE: std::ops::Range<usize> = 0x1A4..0x1A8;

/// The offsets at which `rom` (assembled) and `refrom` (final, post-convsym)
/// differ WITHIN the two semantic header fields above — the derived
/// convsym/fixheader allowlist. Confinement is NOT checked here: pass the
/// result to [`assert_rom_matches`] (or use [`assert_rom_matches_convsym`]),
/// whose unexpected-offset check then asserts the FULL diff set ⊆ these two
/// fields with the DSM.9 evidence format on failure.
pub fn derive_convsym_rewritten(rom: &[u8], refrom: &[u8]) -> Vec<usize> {
    let n = rom.len().min(refrom.len());
    (0..n)
        .filter(|&i| CHECKSUM_FIELD_RANGE.contains(&i) || ROM_END_FIELD_RANGE.contains(&i))
        .filter(|&i| rom[i] != refrom[i])
        .collect()
}

/// [`assert_rom_matches`] with the convsym allowlist DERIVED-and-CONFINED
/// (D-T10.6): the allowlist is computed from the actual header diff instead
/// of pinned, killing that re-pin row; confinement to the two semantic
/// fields is enforced by the unexpected-offset check (any diff outside them
/// is unlisted and fails with full evidence). Each field must genuinely
/// differ somewhere — a reference rebuilt WITHOUT the convsym append would
/// otherwise silently change shape under us (the guard the pinned arrays'
/// "allowlisted bytes must differ" assert used to provide).
pub fn assert_rom_matches_convsym(rom: &[u8], refrom: &[u8], expected_len: usize, label: &str) {
    let allow = derive_convsym_rewritten(rom, refrom);
    for (field, range) in
        [("checksum", CHECKSUM_FIELD_RANGE), ("ROM-end pointer", ROM_END_FIELD_RANGE)]
    {
        assert!(
            allow.iter().any(|i| range.contains(i)),
            "{label}: expected the convsym/fixheader post-steps to rewrite the header {field} \
             field ({range:#X?}), but assembled and final ROMs match there — did the reference \
             lose its deb2 append?"
        );
    }
    assert_rom_matches(rom, refrom, expected_len, &allow, label);
}

/// Assert `rom` is byte-identical to `refrom` modulo the `allow`-listed offsets,
/// after pinning `rom`'s length to `expected_len` (guards against a regression
/// that drops/adds a trailing section while leaving the header-adjacent prefix —
/// and the allowlisted diffs — byte-identical, which would otherwise silently
/// pass the diff check below).
///
/// On mismatch, reports the FIRST unexpected differing offset with 16 bytes of
/// context from each side (the DSM.9 STOP-RULE evidence format), plus every
/// unexpected offset's sigil/ref byte values, then panics. Finally confirms the
/// allowlisted bytes genuinely differ — this guards against the reference
/// silently changing shape under us (e.g. a rebuild without the convsym append
/// would make these match, and this assertion would catch it).
///
/// `label` names the ROM under test in panic messages (e.g. `"mixed"`,
/// `"sigil"`, `"sigil debug"`) so failures from different gates are
/// distinguishable.
pub fn assert_rom_matches(
    rom: &[u8],
    refrom: &[u8],
    expected_len: usize,
    allow: &[usize],
    label: &str,
) {
    assert_eq!(
        rom.len(),
        expected_len,
        "{label} ROM length changed (dropped/added section, or an org skip lost content?); \
         expected EndOfRom {expected_len:#x}"
    );
    assert!(
        rom.len() <= refrom.len(),
        "{label} ROM {} longer than reference {}",
        rom.len(),
        refrom.len()
    );

    let unexpected: Vec<usize> =
        (0..rom.len()).filter(|&i| rom[i] != refrom[i] && !allow.contains(&i)).collect();
    if let Some(&i) = unexpected.first() {
        let ctx = |b: &[u8]| {
            let hi = (i + 16).min(b.len());
            b[i..hi].to_vec()
        };
        let detail: Vec<String> = unexpected
            .iter()
            .map(|&j| format!("{j:#x} ({label} {:#04x} != ref {:#04x})", rom[j], refrom[j]))
            .collect();
        panic!(
            "{label} ROM diverges from the reference at {} unexpected offset(s); \
             FIRST at {i:#x} ({label} {:#04x} != ref {:#04x})\n\
             {label}[{i:#x}..] = {:02X?}\n  ref[{i:#x}..] = {:02X?}\n\
             (all unexpected offsets: {})",
            unexpected.len(),
            rom[i],
            refrom[i],
            ctx(rom),
            ctx(refrom),
            detail.join(", "),
        );
    }
    // The allowlisted bytes MUST genuinely differ — else the reference changed
    // shape under us (e.g. a rebuild without the convsym append).
    for &i in allow {
        assert!(
            i < rom.len() && rom[i] != refrom[i],
            "expected convsym-rewritten byte at {i:#x} to differ, but it matched"
        );
    }
}

/// Assemble the AS side of the TRANCHE-20 mixed `.asm`+`.emp` build:
/// `SIGIL_EMP_DMA_QUEUE` and `SIGIL_EMP_LOAD_ART` defined so engine.inc's two
/// `ifndef` blocks (which normally include `engine/system/dma_queue.asm` /
/// `engine/level/load_art.asm`) are each REPLACED by an `org` resume — per
/// shape, dma_queue `$1F56` (plain) / `$1FDC` (`__DEBUG__`), load_art `$6110`
/// / `$6DE4` — leaving the two windows for the `.emp` side's sections to
/// supply. The gates are INDEPENDENT of the sound/hblank ladder (R6), so this
/// arm carries only the two tranche-20 gates: it proves the two new regions
/// splice into the REAL full ROM, not just their standalone windows.
///
/// Cross-seam: vblank.asm's `bsr.w Process_DMA_*`, boot.asm's
/// `bsr.w Init_DMA_Queue`, dplc.asm's `bsr.w QueueDMA_*` and the game states'
/// `jsr QueueDMA_Critical` are unconditional AS-side consumers of the `.emp`
/// modules' `pub proc` names (the `JmpJsrSym` / branch deferrals); all queue
/// RAM labels are unconditional in engine/ram.asm, so no synthetic symbol
/// injection is needed.
pub fn assemble_mixed_tranche20_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DMA_QUEUE".to_string(), 1),
        ("SIGIL_EMP_LOAD_ART".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche20 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the TRANCHE-21 mixed `.asm`+`.emp` build:
/// `SIGIL_EMP_BUFFERS` and `SIGIL_EMP_VBLANK` defined so engine.inc's two
/// `ifndef` blocks (which normally include `engine/system/buffers.asm` /
/// `engine/system/vblank.asm`) are each REPLACED by an `org` resume, leaving
/// the two windows for the `.emp` side's sections to supply.
///
/// Cross-seam headline: this arm carries the campaign's FIRST `.asm` data
/// directive and immediate-operand references to `.emp`-exported procs —
/// vectors.asm's `dc.l VBlank_Handler` (the IRQ6 vector) and boot.asm's /
/// ojz_scroll_test.asm's `move.l #VInt_Level, (VInt_Ptr).w` — plus the
/// exercised bsr/jsr classes (boot's Init_SpriteTable/BuildStaticDMA, the
/// game states' `jsr Init_SpriteTable`, game_loop.asm's `bsr.w VSync_Wait`).
pub fn assemble_mixed_tranche21_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_BUFFERS".to_string(), 1),
        ("SIGIL_EMP_VBLANK".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche21 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the TRANCHE-22 mixed `.asm`+`.emp` build:
/// `SIGIL_EMP_S4LZ`, `SIGIL_EMP_ZX0` and `SIGIL_EMP_COMPRESSION_SELFTEST`
/// defined so engine.inc's three blocks are each REPLACED by an `org` resume
/// — the selftest block is the campaign's first DEBUG-ONLY region (the twin
/// is whole-file `ifdef __DEBUG__`): its gated arm emits NOTHING in the
/// plain shape and, in debug, orgs past the .emp code window and keeps the
/// generated golden-vector data (`engine/debug/generated/vectors.asm`)
/// AS-side.
///
/// Cross-seam headline: tile_cache.asm's `bsr.w S4LZ_DecompressDict`,
/// load_art.asm's `bra.w S4LZ_Decompress` / `bra.w ZX0_Decompress`, boot.asm's
/// debug `bsr.w CompressionSelfTest` (.asm→.emp), and the REVERSE data seam —
/// the .emp selftest consuming the AS-side `CSelf_*` labels and generated
/// `CSELF_*` equ values at link.
pub fn assemble_mixed_tranche22_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_S4LZ".to_string(), 1),
        ("SIGIL_EMP_ZX0".to_string(), 1),
        ("SIGIL_EMP_COMPRESSION_SELFTEST".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche22 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 23 (boot): the full AS-side game with `SIGIL_EMP_BOOT` on — the
/// engine.inc arm skips boot.asm's code and org-resumes at the per-shape
/// BootData address, keeping the DATA TAIL (boot_data.asm — movem preloads,
/// VDP reg bytes, Z80 blob include, PSG bytes, post-DMA commands) AS-side in
/// both arms, so no splice window crosses the nested Z80 source include.
///
/// Cross-seam headline: vectors.asm's reset `dc.l EntryPoint` resolving
/// against the .emp-owned symbol (the t21 IRQ6 class, now at the RESET
/// vector), the .emp's forward `lea BootData(pc)` into the AS-side table,
/// and the link-time value seam (`Z80_SOUND_SIZE` imm16 arithmetic +
/// `GAME_ENTRY_ID` through the tranche-23 `.b` deferral).
pub fn assemble_mixed_tranche23_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_BOOT".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche23 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 24 (children): the full AS-side game with `SIGIL_EMP_CHILDREN` on —
/// the engine.inc arm skips children.asm and org-resumes at the per-shape
/// `Load_Object` address. The descriptor TABLES stay game-side in both arms
/// (they live in `games/sonic4/objects/test_parent.asm`, never in the region).
///
/// Cross-seam headline: the SHARED ANCHORS. `PopulateSpawnedPieceCount` is the
/// gate's start and entity_window's end; `Load_Object` is the gate's end and
/// load_object's start — with children.asm's include gated out, the AS side
/// must still land `Load_Object` exactly at its canonical address. Plus the
/// `.asm → .emp` caller class at four game-side objects: test_parent's
/// `jsr CreateChild_Normal` / `jsr DeleteChildren`, test_emitter's and
/// test_stress_emitter's `jsr CreateEffect_Normal`, and test_churn's
/// `jsr PopulateSpawnedPieceCount` all resolve against the .emp exports.
pub fn assemble_mixed_tranche24_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_CHILDREN".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche24 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 29 (game-side G1): the full AS-side game with `SIGIL_EMP_TEST_STATIC`
/// + `SIGIL_EMP_TEST_ANIMATED` on — main.asm skips test_static.asm/test_animated.asm
/// and org-resumes at the per-shape (shape-invariant) `$10C6A` / `$10CC4` bank
/// addresses; the two `.emp` modules fill the windows. Shared anchor: `TestPlayer`
/// is test_animated's gate end AND the next object's start, so with both includes
/// gated out the AS side must still land `TestPlayer` at its canonical address.
/// The `.asm → .emp` caller class: `ObjDef_Static`'s `dc.w objroutine(TestStatic_Main)`
/// (data/objdefs/test_objects.asm) and the object-test harness `TestAnimated`
/// spawn word resolve against the `.emp` exports. The DplcV overlay's AS truth
/// (`_dplc_ptr`/`_art_base`) is defined by the surviving test_player.asm.
pub fn assemble_mixed_tranche29_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_TEST_STATIC".to_string(), 1),
        ("SIGIL_EMP_TEST_ANIMATED".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche29 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 30 (game-side G2): the full AS-side game with `SIGIL_EMP_TEST_EMITTER`
/// + `SIGIL_EMP_TEST_STRESS_EMITTER` + `SIGIL_EMP_TEST_CHURN` on — main.asm skips
/// the three effect/child-lifecycle object includes and org-resumes at the
/// per-shape (shape-invariant) `$11030` (test_parent's first label TestChildPart)
/// / `$111B6` (TestChurnObj) / `$1122E` (ObjDef_PathSwap) bank addresses; the
/// three `.emp` modules fill the windows. The `.asm → .emp` caller class:
/// object_test_state.asm's `dc.w objroutine(TestStressEmitter)` /
/// `objroutine(TestChurnObj)` spawn words resolve against the `.emp` exports; the
/// emitter descriptors' `TestParticle` word resolves against the surviving
/// test_particle.asm. All effect-seam callees (CreateEffect_Normal /
/// PopulateSpawnedPieceCount / AllocDynamic / DeleteObject / Draw_Sprite) are the
/// real AS symbols.
pub fn assemble_mixed_tranche30_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_TEST_EMITTER".to_string(), 1),
        ("SIGIL_EMP_TEST_STRESS_EMITTER".to_string(), 1),
        ("SIGIL_EMP_TEST_CHURN".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche30 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 31 (game-side G3): the full AS-side game with `SIGIL_EMP_TEST_PARENT`
/// on — main.asm skips the test_parent object include and org-resumes at the
/// (shape-invariant) `$1115C` (test_stress_emitter's first label TestStressEmitter)
/// bank address; the `.emp` module fills the window. The effect/child-lifecycle
/// emitters stay AS-side (test_emitter/stress_emitter/churn INCLUDED). The
/// cross-seam callees (CreateChild_Normal / DeleteChildren / GetSineCosine /
/// DeleteObject / Draw_Sprite) are the real AS symbols; TestChildPart is internal
/// to the `.emp`. There is no outbound objroutine-word consumer — object_test_state
/// spawns TestParent, not TestChildPart, and TestParent's dispatch word is the
/// objdef data (which stays AS-side).
pub fn assemble_mixed_tranche31_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_TEST_PARENT".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche31 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 34 (P1 player keystone): the full AS-side game with BOTH
/// `SIGIL_EMP_PLAYER_COMMON` and `SIGIL_EMP_SONIC` on. player_common.asm's internal
/// gate keeps its zero-byte header (PlayerV struct / _pl_* equates / macros) —
/// which the surviving player_ground/air/spindash still read — and org-resumes at
/// PState_Ground; the sonic.asm arm org-resumes at TestStatic_Main. The two arms
/// gate out TOGETHER, so player_common.asm's `lea PhysTable_Sonic` reference does
/// not dangle (both AS bodies vanish; the .emp side owns them). PhysTable_Sonic /
/// Sonic_InitAssets / Sonic_LoadArt become unresolved externs here — sonic.emp
/// owns them (the ownership flip); Player_States' PState_* targets stay AS
/// (surviving state files).
pub fn assemble_mixed_tranche34_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_PLAYER_COMMON".to_string(), 1),
        ("SIGIL_EMP_SONIC".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche34 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 35 (P2+P3 player state machines): the full AS-side game with
/// `SIGIL_EMP_PLAYER_GROUND` + `SIGIL_EMP_PLAYER_AIR` + `SIGIL_EMP_PLAYER_SPINDASH`
/// on. The three state-body includes gate out and org-resume at the next file's
/// first label (PState_Air / PState_Spindash / Sonic_InitAssets). player_common.asm
/// and sonic.asm stay AS — their Player_States table entries (PState_Ground/Roll/
/// Spindash/Air/Jump/RollJump/AirBall) and the state files' calls to
/// Player_SetState/SnapToSurface resolve cross-seam to the spliced .emp regions
/// (the duplicate-local-label combined-link class, fixed at 03d29cd).
pub fn assemble_mixed_tranche35_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_PLAYER_GROUND".to_string(), 1),
        ("SIGIL_EMP_PLAYER_AIR".to_string(), 1),
        ("SIGIL_EMP_PLAYER_SPINDASH".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche35 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 38 (P4 player_sensors): the full AS-side game with
/// `SIGIL_EMP_PLAYER_SENSORS` on. player_sensors is the FIRST engine-block
/// include (gameEngineBlockIncludes); its gate arm org-resumes at Section_Init
/// (game_debug.asm emits zero canonical bytes) — a PER-SHAPE org (the region's
/// base shifts with upstream __DEBUG__ growth). The AS callers of the sensors
/// (player_common/ground/air/spindash.asm + test_player.asm) stay AS and resolve
/// Player_Sensor*/Collision_Probe*/Player_AtLedgeEdge cross-seam to the spliced
/// .emp region (the combined-link duplicate-local-label class).
pub fn assemble_mixed_tranche38_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_PLAYER_SENSORS".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche38 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 39 (the FINAL three objects): the full AS-side game with
/// `SIGIL_EMP_TEST_PLAYER` + `SIGIL_EMP_TEST_ENEMY` + `SIGIL_EMP_PATH_SWAP` on.
/// test_player.asm and test_enemy.asm are INTERNAL-gated (their zero-byte headers
/// stay AS-visible so test_animated.emp's DplcV guards, test_objects.emp's
/// ENEMY_PATROL_SPEED guard, and object_test_state.asm's STUB_FLOOR_Y still
/// resolve); their CODE arms org-resume at TestEnemy_Init / TestSolid_Init.
/// path_swap.asm is WHOLE-FILE gated with a PER-SHAPE org (2 __DEBUG__ blocks,
/// debug +$68), resuming at DeformTable_Zero (gameDataIncludes' first label). The
/// AS-side objdef consumers (test_objects.asm objroutine(TestEnemy_Init),
/// act_descriptor/objdef-table `dc.l ObjDef_PathSwap`) resolve cross-seam to the
/// .emp-owned labels (the ownership flip). After t39 the object bank is ALL-.emp.
pub fn assemble_mixed_tranche39_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_TEST_PLAYER".to_string(), 1),
        ("SIGIL_EMP_TEST_ENEMY".to_string(), 1),
        ("SIGIL_EMP_PATH_SWAP".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche39 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 41 (the T1 harness states — the LAST game-side code tranche): the full
/// AS-side game with `SIGIL_EMP_OBJECT_TEST_STATE` + `SIGIL_EMP_OJZ_SCROLL_TEST`
/// on. Both gameStatesIncludes files are SHAPE-DEPENDENT, so each gate arm
/// org-resumes at a PER-SHAPE address (object_test_state → ojz start; ojz →
/// NullInterrupt, the post-gameStates level-1 stub). The AS-side link resolves
/// ojz's `TestArt`/`TestArt_End` references to object_test_state.emp's exports and
/// config/game.asm's `Game_Entry = GameState_OJZScroll_Init` to the .emp export
/// (the ownership flip). After t41 the 68k game SIDE is code-complete — only
/// main/config remain (the Spec-5 flip itself).
pub fn assemble_mixed_tranche41_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_OBJECT_TEST_STATE".to_string(), 1),
        ("SIGIL_EMP_OJZ_SCROLL_TEST".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed tranche41 AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Seam-1 (the resident sound blob): the full AS-side game with
/// `SIGIL_EMP_Z80_SOUND` on — `boot_data.asm`'s `include z80_sound_driver.asm`
/// (nested inside `ifdef SOUND_DRIVER_ENABLED`) is REPLACED by the gate arm that
/// defines `Z80_Sound_Start` (label at the include position = `$3DE` plain / `$3E2`
/// debug), the numeric per-shape `Z80_SOUND_SIZE` (`$181C` / `$189A`), and
/// org-resumes at `Z80_Sound_End` — leaving the whole `$3DE..$1BFA` / `$3E2..$1C7C`
/// window for the `.emp` side's five natively-linked resident sections
/// (z80_sound_driver / sound_sequencer / sound_sfx / sound_fm / sound_psg) to
/// supply. The AS-side survivors of the blob are the boot copy count
/// (`move.w #Z80_SOUND_SIZE-1, d1`) and the boot_data.asm layout-assert wall
/// (`Z80_Sound_Start-BootData == 54`, `Z80_SOUND_SIZE` parity + total), all
/// satisfied by the numeric carrier. The banked `$8000`-window sound tables
/// (SeqOpcodeTable / SfxBlobWinTab / the FM+PSG LUTs) stay AS-included (seam-2),
/// so the `.emp` side's references to them resolve through the shared symbol table.
pub fn assemble_mixed_z80sound_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_Z80_SOUND".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed z80sound AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Seam-2 (Option Y) — the REAL-BUILD DAC path: the full AS-side game with
/// `SIGIL_EMP_DAC` on and NO `SIGIL_EMP_DAC_BODY_STUB`, so `main.asm`'s
/// `gameSoundDataIncludes` macro takes the BINCLUDE arm (dac_blip_bank.bin @
/// $48000 + dac_shared_bank.bin @ $50000) and `soundBankHead` BINCLUDEs
/// dac_sample_tab.bin at VMA $85AD — exactly what `build.sh` assembles once the
/// two `.asm` twins are deleted. Everything else (MT/SFX/engine) stays pure AS.
/// The three DAC `.bin`s (+ the resident blob) must exist in the aeon tree at the
/// BINCLUDE paths; the seam-2 whole-ROM gate emits them first (like seam-1's
/// `ensure_generated`). Returns the UNLINKED [`Module`]; the caller resolves+links
/// and emits the ROM for the byte gate.
pub fn assemble_seam2_dac_rom_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (seam2 DAC BINCLUDE AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Tranche 25 (error_handler): the full AS-side game with `SIGIL_EMP_ERROR_HANDLER`
/// on — the engine.inc arm skips error_handler.asm, org-resumes at EndOfRom, and
/// defines the numeric `ErrorHandler` base so the always-included
/// mddbg_symbols.asm equ table folds. The 12 exception-vector labels
/// (BusError..ErrorTrap) that vectors.asm spells as `dc.l` become unresolved
/// externs here — the .emp side owns them (the ownership flip). The blob's
/// `.emp` label is renamed (ErrorHandlerBlob), so the numeric `ErrorHandler`
/// equ is the sole owner of that name.
pub fn assemble_mixed_error_handler_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_ERROR_HANDLER".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed error_handler AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}
