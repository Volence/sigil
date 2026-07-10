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

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, Module, SymbolTable};
use sigil_link::LinkedImage;

/// Region A base LMA in the assembled ROM: the resident phase-0 Z80 driver.
/// Provenance: the retired `golden/windows.toml`, `regen`-derived from the
/// bracketing 68k anchor label `Z80_Sound_Start`.
pub const REGION_A_LMA: u32 = 0x3EA;
/// Region B base LMA: the phase-`08000h` Moving-Trucks / SFX engine-table bank.
/// Provenance: `MovingTrucks_Bank_Start`.
pub const REGION_B_LMA: u32 = 0x60000;

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
/// shape, `$6553A` (`__DEBUG__`) or `$63AE8` (plain) — leaving the whole
/// `$60607..end` window for the `.emp` side's `mt_bank` section to supply.
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
/// fatals, R6) is REPLACED by an `org` resume — per shape, `$65C82`
/// (`__DEBUG__`) or `$64230` (plain), i.e. `SfxTable_End` — leaving the whole
/// `$63AE8..SfxTable_End` window for the `.emp` side's `sfx_bank` section to
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
/// (else-arm `org $31FA` plain / `org $34B4` debug). Back in the ENGINE block
/// (like game_loop/collision_lookup), the window it opens (`$308A..$31FA` plain
/// / `$3344..$34B4` debug) is filled by `engine/objects/collision.emp` — whose
/// `TouchResponse` is the sole `pub proc` export (called from the engine object
/// manager). The module reads only GAME-RAM `Player_1`/`Dynamic_Slots` across
/// the seam (abs.w, per-shape); its dispatch is a self-contained module-level
/// handler table (pc-indexed `jsr`), so no ROM cross-seam target moves.
pub fn assemble_mixed_tranche7_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
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

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}

/// The only offsets at which Sigil's assembled (non-debug) ROM legitimately
/// differs from the pinned `s4.bin`: the header checksum and the low half of the
/// `dc.l EndOfRom-1` ROM-end pointer, both rewritten by the out-of-scope
/// `convsym -a`/`fixheader` post-steps (`convsym -a` appends the MD-Debugger
/// `deb2` symbol table and rewrites two header fields; `fixheader` re-checksums
/// the appended image — M1.B models `convsym` as a no-op, so Sigil's `emit_rom`
/// target is the pre-append ASSEMBLED ROM). See `m1d_rom`/`m1d_debug_rom`/
/// `mixed_dac_rom` for the full provenance.
pub const CONVSYM_REWRITTEN: &[usize] = &[0x18E, 0x18F, 0x1A6, 0x1A7];
/// The debug reference's convsym/fixheader-rewritten set: the larger `__DEBUG__`
/// deb2 append pushes the ROM-end pointer over a byte boundary, so three bytes
/// (`$1A5`/`$1A6`/`$1A7`) differ instead of two.
pub const CONVSYM_REWRITTEN_DEBUG: &[usize] = &[0x18E, 0x18F, 0x1A5, 0x1A6, 0x1A7];

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
