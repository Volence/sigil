//! Sound-migration T1+T2+T3 acceptance — the MIXED `.asm`+`.emp` full-ROM harness.
//!
//! This is each tranche's acceptance bar (DSM.9): assemble aeon's REAL
//! `games/sonic4/main.asm` with one or both sound gates ON (so `main.asm`'s
//! `gameSoundDataIncludes` macro skips the matching `.asm` block and resumes
//! placement by `org`), compile the matching REAL `.emp` module(s) from aeon's
//! tree, COMPOSE everything into ONE linked image, and prove the full ROM is
//! BYTE-IDENTICAL to the assembled reference — the same target as the
//! all-`.asm` `m1d_rom` / `m1d_debug_rom` gates.
//!
//! Two variants per tranche (plain + `__DEBUG__`), mirroring the two m1d tests,
//! prove BOTH build shapes compose. The all-`.asm` m1d gates build WITHOUT
//! either define; the mixed tests build WITH them — all coexist.
//!
//! ## Composition (the T1 technique, from ports.rs + dac_port.rs)
//!
//! The `.emp` side is placed into a bank map BY SECTION NAME (dac_port.rs:
//! `dac_blip_bank` @ $50000, `dac_shared_bank` @ $58000; T2 adds `mt_bank` @
//! $60607, size $79F9 — mt_port.rs's region, R7). The top-level `SND_*`/`equ`
//! carriers land in zero-byte `text` sections given a benign home at LMA 0 —
//! T2 lowers TWO `.emp` modules (`dac_samples.emp` + `mt_bank.emp`), each
//! opening its OWN `text` carrier, and P5/R7 already proved a same-named-`text`
//! pair chains cleanly through one region (both zero bytes, cumulative
//! per-region cursor). The `.asm` side's sections are org-pinned by AS itself.
//! Every `Vec<Section>` (AS + both `.emp` modules) is concatenated and run
//! through ONE `resolve_layout` + `link` (ports.rs T4 technique) so every
//! cross-seam symbol resolves through a single shared table. No new link infra.
//!
//! The zero-byte `text` carrier(s) at LMA 0 alias the AS reset section's LMA,
//! but `resolve_layout`'s R7p.4 overlap check filters out zero-image-byte
//! sections (`overlap_diag` keys on `image_final_size`), and `flatten` skips
//! empty sections — so the carrier(s) are benign, contributing nothing and
//! colliding with nothing (proven for the pair in Task 2's P5 probe).
//!
//! ## Gap-fill (Task 9 §3 — inspected in the reference before pinning)
//!
//! In the all-`.asm` ROM the bytes between the pre-DAC content and $50000,
//! between the blip bank's end ($50B40) and $58000, and between the drums' end
//! ($5F8BC) and $60000 are produced by asl `align $8000`. In the mixed build
//! those become INTER-SECTION gaps produced by the flatten fill. `xxd` of the
//! reference `aeon/s4.bin` at all three ranges (0x4FFF0, 0x57FF0, 0x5FFF0, and
//! the two bank tails 0x50B40 / 0x5F8C0) shows the pad byte is `0x00`
//! throughout — matching `sigil.map.toml`'s `fill = 0x00` (which `emit_rom` uses
//! for every gap). The pre-DAC content ends at $4867A (Art_Sonic's `align 2`
//! tail, per s4.lst); the blip bank REALLY starts at $50000 in the reference
//! (verified: 0x4FFFx is all-zero, 0x50000 is the first blip byte `80 A6 …`), so
//! nothing lives in $4867A..$50000 except align pad — exactly the gap the
//! flatten fill reproduces. The `org` skip drops ONLY the two BINCLUDE banks +
//! comments + equates from `dac_samples.asm`; the byte-identity assertion below
//! is what proves nothing else was lost.
//!
//! **T2 adds NO new gap.** The MT block's `.asm` else-arm resumes placement
//! EXACTLY at `mt_bank`'s section end (`$63AE8` plain / `$6553A` debug — the
//! fact base's tail addresses): `mt_bank.emp`'s items emit contiguously
//! (§4.3 no-auto-pad) all the way to `SongPatchTable_End`, and the SFX block
//! that follows in `.asm` picks up at that exact address with no `align`
//! between — so there is no inter-section pad to reason about here, unlike the
//! DAC banks' bank-aligned boundaries above.
//!
//! ## Cross-seam resolution (T2 — the imm32 deferral proving out end-to-end)
//!
//! `engine/sound/sound_api.asm`'s `movea.l #SongTable, a0` / `movea.l
//! #SongPatchTable, a0` are UNCONDITIONAL engine code (not gated by
//! `SIGIL_EMP_MT`) that reference labels `mt_bank.emp`'s `mt_bank` section
//! defines (`SongTable`/`SongPatchTable`, at `$63AE0`/`$63AE4` plain,
//! `$65522`/`$6552E` debug). Since the AS side assembles these two operands
//! before the `.emp` module is even lowered, they are unresolved AT AS-TIME —
//! Task 3's `Value32Be` imm32 deferral (R3) is what lets `main.asm` assemble at
//! all here instead of hard-erroring; the deferred fixups are then satisfied by
//! the ONE joint `resolve_layout` + `link` pass below, exactly like every other
//! cross-seam symbol. `MovingTrucks_Bank_Start` (main.asm:138, read by
//! `mt_bank.emp`'s five `bankid(...)` co-residency ensures) is a real `.asm`
//! label defined UNCONDITIONALLY (outside both gates) — so unlike `mt_port.rs`,
//! no synthetic cross-seam symbol injection is needed here: the real AS module
//! supplies it for real, through the same shared symbol table.
//!
//! ## Cross-seam resolution (T3 — the win-tab `dw` deferral proving out)
//!
//! With `SIGIL_EMP_SFX` on, `sfx_blob_win_tab.asm`'s nine
//! `dw sfx_winptr(Sfx_NN)` entries (a compound `(Sfx_NN & SFX_WIN_MASK) |
//! SFX_WIN_BASE` in the Z80 `phase 08000h` blob) reference `.emp`-side `Sfx_NN`
//! labels, unresolved at AS-time — T0's `dw` deferral (P1) is what lets the AS
//! side assemble. Because ONE leaf (`Sfx_NN`) is external, the whole expr
//! deferred; the front-end's `partial_fold` bakes the env-only equs
//! (`SFX_WIN_MASK`/`SFX_WIN_BASE`, invisible to the linker's section-label
//! table) at that site, so the linker fold sees `(Sfx_NN & 32767) | 32768` and
//! resolves the sole cross-seam leaf through the joint link. `SfxBlobWinTab[0] =
//! sfx_winptr($63AE8) = $BAE8` → LE `E8 BA` at ROM `$6045F`, pinned explicitly
//! in the plain test and re-proven by the full-ROM byte assertion.
//!
//! ## STOP RULE (DSM.9)
//!
//! Expected divergences from the reference: NONE beyond the
//! `convsym`/`fixheader`-rewritten header bytes (identical sets to `m1d_rom` /
//! the T1 mixed tests, since the composed ROM content is byte-identical to the
//! all-`.asm` build). Any other differing offset is a REAL divergence — this
//! test reports it (offset + 16 bytes context each side) and FAILS. It does NOT
//! allowlist new offsets.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (and `aeon/s4.debug.bin`
//! for the debug variants). Absent it SKIPS green unless `SIGIL_STRICT_GATE=1`.
//! Mirrors `m1d_rom` / `m1d_debug_rom`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test mixed_dac_rom
//! ```

use std::path::{Path, PathBuf};

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::{
    assemble_mixed_dac_as_side, assemble_mixed_hblank_as_side, assemble_mixed_mt_as_side,
    assemble_mixed_sfx_as_side, assemble_mixed_tranche2_as_side, assemble_mixed_tranche3_as_side,
    assert_rom_matches, CONVSYM_REWRITTEN, CONVSYM_REWRITTEN_DEBUG,
};
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Section, SymbolTable};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

// `CONVSYM_REWRITTEN` / `CONVSYM_REWRITTEN_DEBUG` (imported from `sigil_harness`):
// IDENTICAL to `m1d_rom` / `m1d_debug_rom`'s sets — the mixed build's ROM content
// is byte-identical to the all-`.asm` build, so `convsym` rewrites the same
// bytes to the same values relative to the same assembled length.

/// The assembled (pre-convsym-append) ROM length pins, from `m1d_rom` /
/// `m1d_debug_rom` — `EndOfRom` of each build shape. The mixed build reproduces
/// the same `EndOfRom` (identical content), so these pins double as a
/// dropped-section guard here too.
const ASSEMBLED_LEN: usize = 0x658B4;
const DEBUG_ASSEMBLED_LEN: usize = 0x673A2;

/// The `.emp` module's own directory in aeon's tree — the `include_root` under
/// which `embed("temp_blip.bin")` / `embed("dac/*.pcm")` resolve (dac_port.rs).
fn sound_dir(aeon: &Path) -> PathBuf {
    aeon.join("games/sonic4/data/sound")
}

/// The two-bank map for placing `dac_samples.emp`'s sections BY NAME, at the
/// aeon-f828406 pins (dac_port.rs verbatim): `dac_blip_bank` @ $50000,
/// `dac_shared_bank` @ $58000. The top-level `SND_*` equs land in the default
/// `text` section — a ZERO-byte carrier here (all equs, no data cells) —
/// which `place_sections` still requires a home for; a nominal `text` region
/// at LMA 0 is benign (the R7p.4 overlap check and `flatten` both skip
/// zero-image-byte sections, so it never collides with the AS reset section
/// that also anchors at LMA 0).
fn emp_bank_map() -> &'static str {
    "fill = 0x00\n\
     \n\
     [[region]]\n\
     name = \"text\"\n\
     lma_base = 0x0000\n\
     size = 0x10\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_blip_bank\"\n\
     lma_base = 0x50000\n\
     size = 0x8000\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_shared_bank\"\n\
     lma_base = 0x58000\n\
     size = 0x8000\n\
     kind = \"rom\"\n"
}

/// T2/T3's map: `emp_bank_map`'s three regions PLUS `mt_bank` @ `0x60607` size
/// `0x79F9` (mt_port.rs's R7 region, to the `$68000` bank top) PLUS the T3
/// `sfx_bank` region — the FIRST shape-dependent region base (R7), so this map
/// is a `fn of debug` where it was a const: plain `$63AE8`/`$4518`, debug
/// `$6553A`/`$2AC6` (both to the same `$68000` bank top). The MT/DAC/`text`
/// regions are byte-for-byte T2's.
///
/// `dac_samples.emp`, `mt_bank.emp`, and `sfx_bank.emp` each open their own
/// zero-byte `text` carrier — Task 2's P5 probe proved a same-named `text`
/// chain resolves fine through one region (cumulative per-region cursor) — so a
/// single `text` region still covers all three modules' carriers here.
///
/// The `mt_bank` region ends at `$60607+$79F9 = $68000` and the `sfx_bank`
/// region opens at `$63AE8`/`$6553A` — the two OVERLAP in LMA space, but this
/// is benign exactly as in T2: `place_sections` matches BY NAME, and each `.emp`
/// module's real section lands only in its OWN named region (`mt_bank`'s section
/// is `$63AE8`-sized within its window, `sfx_bank`'s is 1864 bytes at its base),
/// so no two placed sections collide. `resolve_layout`'s overlap check runs on
/// the placed sections, not the map regions.
fn emp_bank_map_with_mt(debug: bool) -> String {
    let (sfx_base, sfx_size) = if debug {
        ("0x6553A", "0x2AC6")
    } else {
        ("0x63AE8", "0x4518")
    };
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_blip_bank\"\n\
         lma_base = 0x50000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_shared_bank\"\n\
         lma_base = 0x58000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"mt_bank\"\n\
         lma_base = 0x60607\n\
         size = 0x79F9\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = {sfx_base}\n\
         size = {sfx_size}\n\
         kind = \"rom\"\n"
    )
}

/// Port #1's map: `emp_bank_map_with_mt`'s FOUR regions PLUS `hblank` — the
/// SECOND shape-dependent region base (after `sfx_bank`, R7): plain `$227E`,
/// debug `$230C`, both size `$12` (18 bytes, the campaign's first CODE port —
/// shape-invariant CONTENT, shape-dependent BASE, exactly like `sfx_bank`).
/// The DAC/MT/SFX/`text` regions are byte-for-byte `emp_bank_map_with_mt`'s.
///
/// `hblank.emp` itself emits exactly ONE section (`hblank`, pinned) and NO
/// `text` carrier — the braceless `module … in hblank` form routes every item
/// into the named section — so the shared `text` region is spare capacity for
/// this module, kept for map parity with the sound modules.
fn emp_bank_map_with_mt_hblank(debug: bool) -> String {
    let (sfx_base, sfx_size) = if debug {
        ("0x6553A", "0x2AC6")
    } else {
        ("0x63AE8", "0x4518")
    };
    let hblank_base = if debug { "0x230C" } else { "0x227E" };
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"hblank\"\n\
         lma_base = {hblank_base}\n\
         size = 0x12\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_blip_bank\"\n\
         lma_base = 0x50000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_shared_bank\"\n\
         lma_base = 0x58000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"mt_bank\"\n\
         lma_base = 0x60607\n\
         size = 0x79F9\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = {sfx_base}\n\
         size = {sfx_size}\n\
         kind = \"rom\"\n"
    )
}

/// Port #2's map: `emp_bank_map_with_mt_hblank`'s FIVE regions PLUS
/// `controllers` and `math` — the THIRD and FOURTH shape-dependent region
/// bases (after `sfx_bank`/`hblank`, R7): controllers plain `$2290` / debug
/// `$231E` (size `$72`), math plain `$2468` / debug `$25FA` (size `$298`).
/// The DAC/MT/SFX/HBLANK/`text` regions are byte-for-byte
/// `emp_bank_map_with_mt_hblank`'s.
///
/// Neither `controllers.emp` nor `math.emp` opens a `text` carrier (both use
/// the braceless `module … in <section>` form, routing every item into
/// their own named section) — the shared `text` region is spare capacity
/// for both, kept for map parity with the sound/hblank modules.
fn emp_bank_map_with_mt_hblank_tranche2(debug: bool) -> String {
    let (sfx_base, sfx_size) = if debug {
        ("0x6553A", "0x2AC6")
    } else {
        ("0x63AE8", "0x4518")
    };
    let hblank_base = if debug { "0x230C" } else { "0x227E" };
    let controllers_base = if debug { "0x231E" } else { "0x2290" };
    let math_base = if debug { "0x25FA" } else { "0x2468" };
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"hblank\"\n\
         lma_base = {hblank_base}\n\
         size = 0x12\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"controllers\"\n\
         lma_base = {controllers_base}\n\
         size = 0x72\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"math\"\n\
         lma_base = {math_base}\n\
         size = 0x298\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_blip_bank\"\n\
         lma_base = 0x50000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_shared_bank\"\n\
         lma_base = 0x58000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"mt_bank\"\n\
         lma_base = 0x60607\n\
         size = 0x79F9\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = {sfx_base}\n\
         size = {sfx_size}\n\
         kind = \"rom\"\n"
    )
}

/// Tranche 3's map: `emp_bank_map_with_mt_hblank_tranche2`'s SEVEN regions
/// PLUS `vdp_init` and `collision_lookup` — the FIFTH and SIXTH
/// shape-dependent region bases: vdp_init plain `$1C14` / debug `$1C96`
/// (size `$4C`), collision_lookup plain `$4C06` / debug `$542A` (size
/// `$32`). The prior regions are byte-for-byte the tranche-2 map's.
/// `collision_lookup` is the campaign's first region outside the
/// `engine/system`+sound neighborhoods (`engine/level/`).
fn emp_bank_map_tranche3(debug: bool) -> String {
    let vdp_init_base = if debug { "0x1C96" } else { "0x1C14" };
    let collision_base = if debug { "0x542A" } else { "0x4C06" };
    format!(
        "{}\
         \n\
         [[region]]\n\
         name = \"vdp_init\"\n\
         lma_base = {vdp_init_base}\n\
         size = 0x4C\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"collision_lookup\"\n\
         lma_base = {collision_base}\n\
         size = 0x32\n\
         kind = \"rom\"\n",
        emp_bank_map_with_mt_hblank_tranche2(debug)
    )
}

/// Port #1: `placed_emp_sections_with_mt_sfx`'s four-module successor — DAC +
/// MT + SFX + HBLANK, all placed into the per-shape `emp_bank_map_with_mt_hblank`
/// (DAC/SFX/HBLANK defines-less, MT's `DEBUG` — R4). Returns all FOUR modules'
/// placed sections concatenated (dac, mt, sfx, hblank — declaration order
/// only) AND all three asserts-bearing modules' link_asserts (mt == 5,
/// sfx == 1; `hblank.emp` carries none), so the caller can `check_link_asserts`
/// and pin every count after the joint link — the ONE lower pass per module
/// (M2), no second lowering to recover the asserts.
fn placed_emp_sections_with_mt_sfx_hblank(
    aeon: &Path,
    debug_val: i128,
) -> (Vec<Section>, Vec<LinkAssert>, Vec<LinkAssert>) {
    let map = emp_bank_map_with_mt_hblank(debug_val != 0);
    let (mut sections, _dac_asserts) =
        placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], &map);
    let (mt_sections, mt_asserts) = placed_module_sections(
        &sound_dir(aeon),
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        &map,
    );
    // `sfx_bank.emp` lives in `sound/sfx/` (own include_root for its 18
    // `embed`s); NO defines: shape-invariant (R4).
    let (sfx_sections, sfx_asserts) =
        placed_module_sections(&sound_dir(aeon).join("sfx"), "sfx_bank.emp", &[], &map);
    // `hblank.emp` lives in `engine/system/`; NO defines: shape-invariant
    // content, shape-dependent map base only (like sfx_bank).
    let (hblank_sections, _hblank_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "hblank.emp", &[], &map);
    sections.extend(mt_sections);
    sections.extend(sfx_sections);
    sections.extend(hblank_sections);
    (sections, mt_asserts, sfx_asserts)
}

/// Port #2: `placed_emp_sections_with_mt_sfx_hblank`'s six-module successor —
/// DAC + MT + SFX + HBLANK + CONTROLLERS + MATH, all placed into the
/// per-shape `emp_bank_map_with_mt_hblank_tranche2` (DAC/SFX/HBLANK/
/// CONTROLLERS/MATH defines-less, MT's `DEBUG` — R4). Returns all SIX
/// modules' placed sections concatenated (declaration order only) AND all
/// THREE asserts-bearing modules' link_asserts (mt == 5, sfx == 1,
/// controllers == 6 — `engine.constants`'s drift guards, tranche 2's step-2
/// modernize pass; hblank/math carry none) — the ONE lower pass per module
/// (M2).
fn placed_emp_sections_with_mt_sfx_hblank_tranche2(
    aeon: &Path,
    debug_val: i128,
) -> (Vec<Section>, Vec<LinkAssert>, Vec<LinkAssert>, Vec<LinkAssert>) {
    let map = emp_bank_map_with_mt_hblank_tranche2(debug_val != 0);
    let (mut sections, _dac_asserts) =
        placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], &map);
    let (mt_sections, mt_asserts) = placed_module_sections(
        &sound_dir(aeon),
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        &map,
    );
    // `sfx_bank.emp` lives in `sound/sfx/` (own include_root for its 18
    // `embed`s); NO defines: shape-invariant (R4).
    let (sfx_sections, sfx_asserts) =
        placed_module_sections(&sound_dir(aeon).join("sfx"), "sfx_bank.emp", &[], &map);
    // `hblank.emp` lives in `engine/system/`; NO defines: shape-invariant
    // content, shape-dependent map base only (like sfx_bank).
    let (hblank_sections, _hblank_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "hblank.emp", &[], &map);
    // `controllers.emp` lives in `engine/system/` too — same include_root
    // convention as hblank; NO defines: shape-invariant (like hblank). Its
    // `use engine.constants.{...}` edge means `constants_ambient_items`
    // (inside `placed_module_sections_with_roots`) prepends the twin's items,
    // whose six drift-guard `ensure`s ride along as `controllers_asserts`.
    let (controllers_sections, controllers_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "controllers.emp", &[], &map);
    // `math.emp` lives in `engine/system/`, but its `embed("../data/sine.bin")`
    // climbs ONE level above its own directory — `include_root` must be the
    // BROADER `engine/` (the sandbox boundary), `embed_base` the module's OWN
    // dir `engine/system/` (the join point) — see `math_port.rs`'s doc and
    // the campaign gap ledger. NO defines: shape-invariant.
    let (math_sections, _math_asserts) = placed_module_sections_with_roots(
        &aeon.join("engine"),
        &aeon.join("engine/system"),
        "math.emp",
        &[],
        &map,
    );
    sections.extend(mt_sections);
    sections.extend(sfx_sections);
    sections.extend(hblank_sections);
    sections.extend(controllers_sections);
    sections.extend(math_sections);
    (sections, mt_asserts, sfx_asserts, controllers_asserts)
}

/// Tranche 3: the eight-module successor — everything
/// `placed_emp_sections_with_mt_sfx_hblank_tranche2` composes PLUS
/// `vdp_init.emp` (`engine/system/`) and `collision_lookup.emp`
/// (`engine/level/` — the campaign's first module outside `engine/system` +
/// sound), both defines-less (shape-invariant SOURCE; their linked bytes
/// still differ per shape because the cross-seam pc-relative distances and
/// the game-RAM `Cache_*` abs.w addresses resolve per shape — the map bases
/// and the AS side supply the shape). Neither carries link asserts in step 1
/// (local const twins; the `extern()` drift guards arrive with the step-2
/// twin migration), so the asserts tuple shape is unchanged.
fn placed_emp_sections_tranche3(
    aeon: &Path,
    debug_val: i128,
) -> (Vec<Section>, Vec<LinkAssert>, Vec<LinkAssert>, Vec<LinkAssert>) {
    let map = emp_bank_map_tranche3(debug_val != 0);
    let (mut sections, _dac_asserts) =
        placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], &map);
    let (mt_sections, mt_asserts) = placed_module_sections(
        &sound_dir(aeon),
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        &map,
    );
    let (sfx_sections, sfx_asserts) =
        placed_module_sections(&sound_dir(aeon).join("sfx"), "sfx_bank.emp", &[], &map);
    let (hblank_sections, _hblank_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "hblank.emp", &[], &map);
    let (controllers_sections, controllers_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "controllers.emp", &[], &map);
    let (math_sections, _math_asserts) = placed_module_sections_with_roots(
        &aeon.join("engine"),
        &aeon.join("engine/system"),
        "math.emp",
        &[],
        &map,
    );
    // The two tranche-3 modules: no defines, no embeds, no `use` edges in
    // step 1 — the plainest `placed_module_sections` shape in the file.
    let (vdp_init_sections, _vdp_init_asserts) =
        placed_module_sections(&aeon.join("engine/system"), "vdp_init.emp", &[], &map);
    let (collision_sections, _collision_asserts) =
        placed_module_sections(&aeon.join("engine/level"), "collision_lookup.emp", &[], &map);
    sections.extend(mt_sections);
    sections.extend(sfx_sections);
    sections.extend(hblank_sections);
    sections.extend(controllers_sections);
    sections.extend(math_sections);
    sections.extend(vdp_init_sections);
    sections.extend(collision_sections);
    (sections, mt_asserts, sfx_asserts, controllers_asserts)
}

/// Compile the REAL `dac_samples.emp` and PLACE its sections into the two-bank
/// map (dac_port.rs pipeline). Returns the placed sections ready to concat with
/// the AS side. Placement runs against `emp_bank_map`, NOT the whole-ROM
/// `sigil.map.toml`, because `place_sections` matches BY NAME and errors on any
/// section without a region — so the AS sections (org-pinned already) are never
/// fed through it; only the emp sections are placed here, then the union is
/// resolved+linked once.
fn placed_emp_sections(aeon: &Path) -> Vec<Section> {
    placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], emp_bank_map()).0
}

/// Compile the REAL `dac_samples.emp` (no defines) and `mt_bank.emp` (`DEBUG`
/// matching the shape), each PLACED into `emp_bank_map_with_mt`'s regions by
/// name, returning BOTH modules' placed sections concatenated (dac first, mt
/// second — declaration order only, `resolve_layout` doesn't care) AND
/// `mt_bank.emp`'s link_asserts (M2: this is the ONE lower pass over
/// `mt_bank.emp` — `build_mixed_mt_rom` used to lower it a second time just to
/// recover the link_asserts list, which risked the two passes drifting; now
/// there is exactly one source of truth for both the asserts-check and the
/// byte composition).
fn placed_emp_sections_with_mt(
    aeon: &Path,
    debug_val: i128,
) -> (Vec<Section>, Vec<LinkAssert>) {
    let map = emp_bank_map_with_mt(debug_val != 0);
    let (mut sections, _dac_asserts) =
        placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], &map);
    let (mt_sections, mt_asserts) = placed_module_sections(
        &sound_dir(aeon),
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        &map,
    );
    sections.extend(mt_sections);
    (sections, mt_asserts)
}

/// T3: `placed_emp_sections_with_mt`'s three-module successor — DAC + MT +
/// SFX, all placed into the per-shape `emp_bank_map_with_mt` (DAC/MT defines-
/// less except MT's `DEBUG`, SFX defines-less in BOTH shapes — R4). Returns all
/// THREE modules' placed sections concatenated (dac, mt, sfx — declaration
/// order only) AND BOTH the MT and SFX modules' link_asserts, so the caller can
/// `check_link_asserts` and pin BOTH counts (mt == 5, sfx == 1) after the joint
/// link — the ONE lower pass per module (M2), no second lowering to recover the
/// asserts.
fn placed_emp_sections_with_mt_sfx(
    aeon: &Path,
    debug_val: i128,
) -> (Vec<Section>, Vec<LinkAssert>, Vec<LinkAssert>) {
    let map = emp_bank_map_with_mt(debug_val != 0);
    let (mut sections, _dac_asserts) =
        placed_module_sections(&sound_dir(aeon), "dac_samples.emp", &[], &map);
    let (mt_sections, mt_asserts) = placed_module_sections(
        &sound_dir(aeon),
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        &map,
    );
    // `sfx_bank.emp` lives in `sound/sfx/` and its eighteen `embed("sfx_*.bin")`
    // resolve bare against that dir — so its module directory (and include_root)
    // is `sound/sfx`, not `sound`. NO defines: the block is shape-invariant (R4).
    let (sfx_sections, sfx_asserts) =
        placed_module_sections(&sound_dir(aeon).join("sfx"), "sfx_bank.emp", &[], &map);
    sections.extend(mt_sections);
    sections.extend(sfx_sections);
    (sections, mt_asserts, sfx_asserts)
}

/// Shared body of `placed_emp_sections`/`placed_emp_sections_with_mt`/
/// `placed_emp_sections_with_mt_sfx`: parse + lower the named `.emp` file (from
/// `dir`, which is ALSO its `include_root` — `sound` for dac/mt, `sound/sfx`
/// for sfx_bank, whose eighteen `embed`s resolve bare against its own dir — with
/// the given comptime `defines`), place its sections into `map_src` by name, and
/// return the placed sections ALONGSIDE the module's link_asserts (captured
/// before `place_sections` consumes `module.sections`) — the single lowering
/// pass all callers above rely on (M2).
fn placed_module_sections(
    dir: &Path,
    module_file: &str,
    defines: &[(String, i128)],
    map_src: &str,
) -> (Vec<Section>, Vec<LinkAssert>) {
    placed_module_sections_with_roots(dir, dir, module_file, defines, map_src)
}

/// `engine.constants`'s items (its six `pub const`s + six drift-guard
/// `ensure`s), read from `controllers.emp`'s own directory (`engine/system/`
/// — where `constants.emp` also lives). `controllers.emp` `use`s this twin;
/// plain `lower_module` (used throughout this file, not the whole-program
/// resolver — see `controllers_port.rs`'s doc comment for why: the resolver's
/// `report_unresolved` wrongly rejects this module's genuinely AS-side-only
/// symbols like `Ctrl_1_Held`) never resolves cross-module `use`, so the
/// twin's items are prepended by hand before lowering, mirroring
/// `controllers_port.rs`'s `controllers_with_ambient_constants`.
fn constants_ambient_items(controllers_dir: &Path) -> Vec<sigil_frontend_emp::ast::Item> {
    let src = std::fs::read_to_string(controllers_dir.join("constants.emp"))
        .unwrap_or_else(|e| panic!("cannot read constants.emp: {e}"));
    let (file, cdiags) = parse_str(&src);
    assert!(
        cdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "constants.emp parse errors: {cdiags:?}"
    );
    file.items
}

/// Like [`placed_module_sections`], but with `include_root` and `embed_base`
/// supplied independently (port #2, `math.emp`'s `embed("../data/sine.bin")`
/// — see `math_port.rs`'s doc for why this module needs a BROADER
/// `include_root` than its own directory). Every other module's call goes
/// through `placed_module_sections`, which passes the same `dir` for both
/// (unaffected — the front-end's `embed_base: None` fallback already made
/// this identical to the pre-`embed_base` behavior).
///
/// `module_file == "controllers.emp"` gets `engine.constants`'s items
/// prepended (its `use engine.constants.{...}` edge) — see
/// `constants_ambient_items`'s doc. Every other module has no cross-module
/// `use`, so this is a no-op for them.
fn placed_module_sections_with_roots(
    include_root: &Path,
    embed_base: &Path,
    module_file: &str,
    defines: &[(String, i128)],
    map_src: &str,
) -> (Vec<Section>, Vec<LinkAssert>) {
    let dir = embed_base.to_path_buf();
    let emp_path = dir.join(module_file);
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} parse errors: {pdiags:?}"
    );
    let file = if module_file == "controllers.emp" {
        sigil_frontend_emp::ast::File {
            module: file.module.clone(),
            attrs: file.attrs.clone(),
            items: constants_ambient_items(&dir).into_iter().chain(file.items).collect(),
            docs: file.docs.clone(),
        }
    } else {
        file
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(include_root.to_path_buf()),
        embed_base: Some(embed_base.to_path_buf()),
        defines: defines.to_vec(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(map_src).expect("emp bank map must load");
    let mut sections = module.sections;
    let link_asserts = module.link_asserts;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} place_sections errors (region-per-section): {pdiags:?}"
    );

    // The `text` carrier MUST stay zero-byte — the whole "benign at LMA 0"
    // argument (module doc) rests on it contributing no image bytes.
    assert!(
        sections.iter().filter(|s| s.name == "text").all(|s| s.image_bytes().is_empty()),
        "{module_file} `text` carrier gained image bytes — the zero-byte-carrier invariant is broken"
    );

    (sections, link_asserts)
}

/// Shared body: assemble the AS side (gate ON, `debug` toggling `__DEBUG__`),
/// compose with the placed emp sections, resolve+link ONCE, and emit the full
/// ROM through the whole-ROM `sigil.map.toml`.
fn build_mixed_rom(aeon: &Path, debug: bool) -> Vec<u8> {
    let as_module = assemble_mixed_dac_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));

    // Concat: AS sections (org-pinned by AS) + placed emp sections. ONE joint
    // resolve_layout + link over the union — the ports.rs T4 mixed-seam shape.
    let mut sections = as_module.sections;
    sections.extend(placed_emp_sections(aeon));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed): {d:?}"));

    // The whole-ROM map: `region_for` covers [0, 0x400000) so every section
    // (AS + emp banks) validates by LMA, and `fill = 0x00` matches the reference
    // align-pad byte inspected above.
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed): {e}"))
}

/// T2's shared body: assemble the AS side with BOTH `SIGIL_EMP_DAC` +
/// `SIGIL_EMP_MT` on, compose with BOTH placed `.emp` modules' sections,
/// resolve+link ONCE (so `sound_api.asm`'s deferred `movea.l
/// #SongTable`/`#SongPatchTable` fixups resolve against `mt_bank.emp`'s
/// labels through the SAME shared table everything else uses), check the five
/// cross-seam `ensure`s actually ran and passed, and emit the full ROM.
/// Returns `(rom_bytes, link_assert_diags)` — the caller asserts the diags are
/// all non-Error, mirroring `mt_port.rs`'s explicit `check_link_asserts` call.
fn build_mixed_mt_rom(aeon: &Path, debug: bool) -> (Vec<u8>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_mt_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    // M2: ONE lower pass over `mt_bank.emp`, via `placed_emp_sections_with_mt`
    // -> `placed_module_sections`, produces BOTH the placed sections (for the
    // byte composition below) and the link_asserts (for the check right
    // after) — so the two can never drift apart the way two independent
    // lowerings could.
    let (emp_sections, link_asserts) = placed_emp_sections_with_mt(aeon, debug_val);
    let mut sections = as_module.sections;
    sections.extend(emp_sections);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed MT): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed MT): {d:?}"));
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert_eq!(
        guard_assert_count(&link_asserts),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed MT): {e}"));
    (rom, assert_diags)
}

/// T3's shared body — the tranche's final byte-identity proof: assemble the AS
/// side with ALL THREE sound gates on (`SIGIL_EMP_DAC` + `SIGIL_EMP_MT` +
/// `SIGIL_EMP_SFX`), compose with ALL THREE `.emp` modules' placed sections
/// (dac + mt + sfx), and run ONE joint `resolve_layout` + `link`.
///
/// This is where the win-tab `dw` deferral proves out END-TO-END: with
/// `SIGIL_EMP_SFX` on, the `soundBankHead` win-tab's nine
/// `dw sfx_winptr(Sfx_NN)` entries assemble on the AS side with `Sfx_NN`
/// UNRESOLVED (P1's deferral — a compound `(Sfx_NN & $7FFF) | $8000` lowered to
/// a `Value16Le` fixup in the Z80 `phase 08000h` blob) and are satisfied by
/// `sfx_bank.emp`'s labels through this ONE shared symbol table. The first
/// entry resolves to `sfx_winptr($63AE8) = ($63AE8 & $7FFF) | $8000 = $BAE8` →
/// LE bytes `E8 BA` at ROM `$6045F` (`SfxBlobWinTab` @ Z80 vma `$845F`,
/// phase-based at `$60000+$45F`) — covered by the full-ROM byte assertion below.
///
/// Returns `(rom_bytes, mt_assert_diags, sfx_assert_diags)` — the caller pins
/// BOTH modules' `check_link_asserts` (mt == 5, sfx == 1) and asserts every
/// diagnostic is non-Error (the I1 non-vacuous lesson: a positive gate that
/// never ran would silently pass).
fn build_mixed_sfx_rom(
    aeon: &Path,
    debug: bool,
) -> (Vec<u8>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_sfx_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    // ONE lower pass per module (M2): both the placed sections AND both modules'
    // link_asserts come from the same lowering, so the byte composition and the
    // asserts-check can never drift.
    let (emp_sections, mt_asserts, sfx_asserts) =
        placed_emp_sections_with_mt_sfx(aeon, debug_val);
    let mut sections = as_module.sections;
    sections.extend(emp_sections);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed SFX): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed SFX): {d:?}"));

    // The mixed path does NOT run the asserts as part of `link()` — check both
    // modules' ensures explicitly and pin both counts.
    let mt_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &mt_asserts);
    let sfx_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &sfx_asserts);
    assert_eq!(
        guard_assert_count(&mt_asserts),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert_eq!(
        guard_assert_count(&sfx_asserts),
        1,
        "sfx_bank.emp's single co-residency ensure must be captured"
    );

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom =
        sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed SFX): {e}"));
    (rom, mt_diags, sfx_diags)
}

/// Port #1's shared body — the campaign's first CODE-port acceptance: assemble
/// the AS side with ALL FOUR gates on (`SIGIL_EMP_DAC` + `SIGIL_EMP_MT` +
/// `SIGIL_EMP_SFX` + `SIGIL_EMP_HBLANK`), compose with ALL FOUR `.emp` modules'
/// placed sections (dac + mt + sfx + hblank), and run ONE joint
/// `resolve_layout` + `link`.
///
/// This is where the port #1 cross-seam reads prove out END-TO-END:
/// `vectors.asm`'s `dc.l HBlank_Dispatch` (an Abs32 fixup deferral) and
/// `boot.asm`'s `move.l #HBlank_Null, (HBlank_Handler_Ptr).w` (the
/// `try_defer_long_imm` abs-dest extension's Value32Be fixup) both resolve
/// against `hblank.emp`'s BARE `pub proc` symbols through this shared table —
/// the same joint-link mechanism as `sound_api.asm`'s `movea.l #SongTable`
/// (T2) and the win-tab `dw sfx_winptr` deferral (T3), now proven for a
/// register-absent `move.l #imm, (abs).w` immediate fixup too.
///
/// Returns `(rom_bytes, mt_assert_diags, sfx_assert_diags)` — the caller pins
/// both asserts-bearing modules' `check_link_asserts` (mt == 5, sfx == 1) and
/// asserts every diagnostic is non-Error. `hblank.emp` carries no link
/// asserts of its own (no `ensure`/`extern`), so it contributes none here.
fn build_mixed_hblank_rom(
    aeon: &Path,
    debug: bool,
) -> (Vec<u8>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_hblank_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    let (emp_sections, mt_asserts, sfx_asserts) =
        placed_emp_sections_with_mt_sfx_hblank(aeon, debug_val);
    let mut sections = as_module.sections;
    sections.extend(emp_sections);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed HBLANK): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed HBLANK): {d:?}"));

    let mt_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &mt_asserts);
    let sfx_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &sfx_asserts);
    assert_eq!(
        guard_assert_count(&mt_asserts),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert_eq!(
        guard_assert_count(&sfx_asserts),
        1,
        "sfx_bank.emp's cross-seam co-residency ensure must be captured"
    );

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map)
        .unwrap_or_else(|e| panic!("emit_rom (mixed HBLANK): {e}"));
    (rom, mt_diags, sfx_diags)
}

/// Port #1 acceptance — plain (non-debug) DAC+MT+SFX+HBLANK mixed build ==
/// `aeon/s4.bin`, modulo the four convsym bytes. All four gates are ON; all
/// four `.emp` modules are lowered and composed; the mt/sfx cross-seam
/// ensures must genuinely run and pass; the `hblank` section itself is pinned
/// explicitly (the port's own byte gate — `hblank_port.rs`'s region-level
/// twin) before the whole-ROM assertion re-proves it.
#[test]
fn mixed_hblank_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags) = build_mixed_hblank_rom(&aeon, false);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );

    // The hblank block itself, pinned explicitly (the port's own 18-byte
    // window) before the whole-ROM assertion below re-proves it in context.
    assert_eq!(
        &rom[0x227E..0x2290],
        &[0x48, 0xE7, 0xC0, 0x80, 0x20, 0x78, 0x80, 0x22, 0x4E, 0x90, 0x4C, 0xDF, 0x01, 0x03, 0x4E, 0x73, 0x4E, 0x75],
        "hblank block must match the reference bytes exactly (plain)"
    );

    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed HBLANK");
}

/// Port #1 acceptance — `__DEBUG__` DAC+MT+SFX+HBLANK mixed build ==
/// `aeon/s4.debug.bin`, modulo the five convsym bytes. Same four-module
/// composition as the plain variant, with `DEBUG=1` driving `mt_bank.emp`'s
/// if-expressions and `__DEBUG__` on the AS side; `hblank.emp` is
/// shape-invariant (its content is identical in both shapes — only its map
/// base moves, R7, exactly like `sfx_bank.emp`).
#[test]
fn mixed_hblank_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags) = build_mixed_hblank_rom(&aeon, true);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );

    assert_eq!(
        &rom[0x230C..0x231E],
        &[0x48, 0xE7, 0xC0, 0x80, 0x20, 0x78, 0x80, 0x22, 0x4E, 0x90, 0x4C, 0xDF, 0x01, 0x03, 0x4E, 0x73, 0x4E, 0x75],
        "hblank block must match the reference bytes exactly (debug)"
    );

    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed HBLANK debug",
    );
}

/// Port #2's shared body — the campaign's cumulative six-module acceptance:
/// assemble the AS side with ALL SIX gates on (`SIGIL_EMP_DAC` +
/// `SIGIL_EMP_MT` + `SIGIL_EMP_SFX` + `SIGIL_EMP_HBLANK` +
/// `SIGIL_EMP_CONTROLLERS` + `SIGIL_EMP_MATH`), compose with ALL SIX
/// `.emp` modules' placed sections, and run ONE joint `resolve_layout` +
/// `link`.
///
/// This is where port #2's cross-seam reads prove out END-TO-END:
/// `vblank.asm`'s two `bsr.w Read_Controllers` sites (a `PcRelDisp16`
/// deferral, already supported) and `test_parent.asm`/`player_ground.asm`'s
/// six `jsr GetSineCosine` sites (the NEW `Fragment::JmpJsrSym` deferral,
/// port #2 follow-up) both resolve against the `.emp` modules' BARE `pub
/// proc` symbols through this shared table — the jsr deferral is only
/// exercised end-to-end here (a real, unconditionally-included AS-side
/// caller of a gated `.emp` proc), not by any prior port.
///
/// Returns `(rom_bytes, mt_assert_diags, sfx_assert_diags,
/// controllers_assert_diags)` — the caller pins all THREE asserts-bearing
/// modules' `check_link_asserts` (mt == 5, sfx == 1, controllers == 6 —
/// `engine.constants`'s drift guards) and asserts every diagnostic is
/// non-Error. `hblank.emp`/`math.emp` carry no link asserts of their own (no
/// `ensure`/`extern`), so they contribute none here.
fn build_mixed_tranche2_rom(
    aeon: &Path,
    debug: bool,
) -> (Vec<u8>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_tranche2_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    let (emp_sections, mt_asserts, sfx_asserts, controllers_asserts) =
        placed_emp_sections_with_mt_sfx_hblank_tranche2(aeon, debug_val);
    let mut sections = as_module.sections;
    sections.extend(emp_sections);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed tranche2): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed tranche2): {d:?}"));

    let mt_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &mt_asserts);
    let sfx_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &sfx_asserts);
    let controllers_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &controllers_asserts);
    assert_eq!(
        guard_assert_count(&mt_asserts),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert_eq!(
        guard_assert_count(&sfx_asserts),
        1,
        "sfx_bank.emp's cross-seam co-residency ensure must be captured"
    );
    assert_eq!(
        guard_assert_count(&controllers_asserts),
        6,
        "engine.constants's six drift-guard ensures must be captured"
    );

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map)
        .unwrap_or_else(|e| panic!("emit_rom (mixed tranche2): {e}"));
    (rom, mt_diags, sfx_diags, controllers_diags)
}

/// Port #2 acceptance — plain (non-debug) DAC+MT+SFX+HBLANK+CONTROLLERS+MATH
/// mixed build == `aeon/s4.bin`, modulo the four convsym bytes. All six gates
/// are ON; all six `.emp` modules are lowered and composed; the mt/sfx
/// cross-seam ensures must genuinely run and pass; the `controllers`/`math`
/// sections themselves are pinned explicitly (each port's own byte gate —
/// `controllers_port.rs`/`math_port.rs`'s region-level twins) before the
/// whole-ROM assertion re-proves them in context.
///
/// This is the acceptance gate for the Org-aware relaxation work (port #2,
/// task 4): the real object-code-bank section (`org $10000`, `engine.inc:174`)
/// is never closed before `gameDataIncludes` chains the parallax data tables
/// into the SAME section — `engine/parallax_macros.inc`'s
/// `parallax_section_end` macro emits a genuine mid-section back-patch
/// (`org pscStart` / `org pscEndPos`, a real `Fragment::Org`), and
/// `test_parent.asm`'s/`player_ground.asm`'s six `jsr GetSineCosine` sites
/// (deferred to `Fragment::JmpJsrSym`, since `GetSineCosine` is external to the
/// AS compile when `SIGIL_EMP_MATH` is on) land EARLIER in that same section.
/// The M1.C T6b categorical `Org`+relaxable refusal was REPLACED by
/// `resolve_layout`'s run/barrier layout math (`shift_breakpoints` /
/// `frag_start_vma` / `run_overrun_diag` now treat every `Org` as a position
/// barrier that resets the per-run growth delta and pins post-org content to
/// the org target), so this full six-module composition links byte-identically
/// to `aeon/s4.bin` — proving the change is byte-neutral for every layout that
/// worked before AND correct for the real `Org`+`JmpJsrSym` co-residency.
#[test]
fn mixed_tranche2_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags, controllers_diags) = build_mixed_tranche2_rom(&aeon, false);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );
    assert!(
        controllers_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's six drift-guard ensures must all PASS (link succeeded): {controllers_diags:?}"
    );

    // The controllers block itself, pinned explicitly (the port's own
    // 0x72-byte window) before the whole-ROM assertion below re-proves it.
    assert_eq!(
        &rom[0x2290..0x2302],
        &[
            0x41, 0xF9, 0x00, 0xA1, 0x00, 0x03, 0x61, 0x2A, 0x12, 0x38, 0x80, 0x2C, 0x11, 0xC0, 0x80, 0x2C,
            0xB1, 0x01, 0xC2, 0x00, 0x83, 0x38, 0x80, 0x30, 0x41, 0xF9, 0x00, 0xA1, 0x00, 0x05, 0x61, 0x12,
            0x12, 0x38, 0x80, 0x2E, 0x11, 0xC0, 0x80, 0x2E, 0xB1, 0x01, 0xC2, 0x00, 0x83, 0x38, 0x80, 0x31,
            0x4E, 0x75, 0x10, 0xBC, 0x00, 0x40, 0x4E, 0x71, 0x10, 0x10, 0x10, 0xBC, 0x00, 0x00, 0x4E, 0x71,
            0x12, 0x10, 0x02, 0x00, 0x00, 0x3F, 0x02, 0x01, 0x00, 0x30, 0xE5, 0x09, 0x80, 0x01, 0x46, 0x00,
            0x12, 0x00, 0x02, 0x01, 0x00, 0x0C, 0x0C, 0x01, 0x00, 0x0C, 0x66, 0x04, 0x02, 0x00, 0x00, 0xF3,
            0x12, 0x00, 0x02, 0x01, 0x00, 0x03, 0x0C, 0x01, 0x00, 0x03, 0x66, 0x04, 0x02, 0x00, 0x00, 0xFC,
            0x4E, 0x75,
        ][..],
        "controllers block must match the reference bytes exactly (plain)"
    );

    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed tranche2");
}

/// Port #2 acceptance — `__DEBUG__`
/// DAC+MT+SFX+HBLANK+CONTROLLERS+MATH mixed build == `aeon/s4.debug.bin`,
/// modulo the five convsym bytes. Same six-module composition as the plain
/// variant, with `DEBUG=1` driving `mt_bank.emp`'s if-expressions and
/// `__DEBUG__` on the AS side; `hblank.emp`/`controllers.emp`/`math.emp` are
/// all shape-invariant (identical content in both shapes — only their map
/// bases move, R7, exactly like `sfx_bank.emp`).
///
/// The debug twin of the Org-aware acceptance gate — see
/// `mixed_tranche2_rom_matches_assembled_reference`'s doc comment (the plain
/// variant); same six-module composition, same run/barrier layout math, both
/// shapes byte-identical to their references.
#[test]
fn mixed_tranche2_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags, controllers_diags) = build_mixed_tranche2_rom(&aeon, true);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );
    assert!(
        controllers_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's six drift-guard ensures must all PASS (link succeeded): {controllers_diags:?}"
    );

    assert_eq!(
        &rom[0x231E..0x2390],
        &[
            0x41, 0xF9, 0x00, 0xA1, 0x00, 0x03, 0x61, 0x2A, 0x12, 0x38, 0x80, 0x2C, 0x11, 0xC0, 0x80, 0x2C,
            0xB1, 0x01, 0xC2, 0x00, 0x83, 0x38, 0x80, 0x30, 0x41, 0xF9, 0x00, 0xA1, 0x00, 0x05, 0x61, 0x12,
            0x12, 0x38, 0x80, 0x2E, 0x11, 0xC0, 0x80, 0x2E, 0xB1, 0x01, 0xC2, 0x00, 0x83, 0x38, 0x80, 0x31,
            0x4E, 0x75, 0x10, 0xBC, 0x00, 0x40, 0x4E, 0x71, 0x10, 0x10, 0x10, 0xBC, 0x00, 0x00, 0x4E, 0x71,
            0x12, 0x10, 0x02, 0x00, 0x00, 0x3F, 0x02, 0x01, 0x00, 0x30, 0xE5, 0x09, 0x80, 0x01, 0x46, 0x00,
            0x12, 0x00, 0x02, 0x01, 0x00, 0x0C, 0x0C, 0x01, 0x00, 0x0C, 0x66, 0x04, 0x02, 0x00, 0x00, 0xF3,
            0x12, 0x00, 0x02, 0x01, 0x00, 0x03, 0x0C, 0x01, 0x00, 0x03, 0x66, 0x04, 0x02, 0x00, 0x00, 0xFC,
            0x4E, 0x75,
        ][..],
        "controllers block must match the reference bytes exactly (debug)"
    );

    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed tranche2 debug",
    );
}

/// Tranche 3's shared body — the campaign's cumulative EIGHT-module
/// acceptance: assemble the AS side with all eight gates on, compose with
/// all eight `.emp` modules' placed sections, and run ONE joint
/// `resolve_layout` + `link`.
///
/// This is where tranche 3's cross-seam reads prove out END-TO-END against
/// the REAL tree (not the port gates' synthetic label sections): `lea.l
/// BootData_VDPRegs(pc), a0` and `bsr.w Tile_Cache_GetCollision` resolve
/// their PC-RELATIVE fixups against the real `boot.asm`/`tile_cache.asm`
/// labels at whatever address the joint layout puts them — the first
/// campaign proof that a `.emp` module's pc-relative distance to an AS-side
/// label survives full-ROM composition (the port gates proved it against
/// `phase`d stand-ins).
///
/// Returns the same tuple shape as `build_mixed_tranche2_rom` — the two new
/// modules carry no link asserts in step 1 (local const twins; `extern()`
/// drift guards arrive with the step-2 twin migration).
fn build_mixed_tranche3_rom(
    aeon: &Path,
    debug: bool,
) -> (Vec<u8>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_tranche3_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    let (emp_sections, mt_asserts, sfx_asserts, controllers_asserts) =
        placed_emp_sections_tranche3(aeon, debug_val);
    let mut sections = as_module.sections;
    sections.extend(emp_sections);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed tranche3): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed tranche3): {d:?}"));

    let mt_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &mt_asserts);
    let sfx_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &sfx_asserts);
    let controllers_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &controllers_asserts);
    assert_eq!(
        guard_assert_count(&mt_asserts),
        5,
        "mt_bank.emp's five co-residency ensures must be captured"
    );
    assert_eq!(
        guard_assert_count(&sfx_asserts),
        1,
        "sfx_bank.emp's cross-seam co-residency ensure must be captured"
    );
    assert_eq!(
        guard_assert_count(&controllers_asserts),
        6,
        "engine.constants's six drift-guard ensures must be captured"
    );

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map)
        .unwrap_or_else(|e| panic!("emit_rom (mixed tranche3): {e}"));
    (rom, mt_diags, sfx_diags, controllers_diags)
}

/// Tranche 3 acceptance — plain (non-debug) EIGHT-module mixed build ==
/// `aeon/s4.bin`, modulo the four convsym bytes. Both new blocks are pinned
/// explicitly (each port's own byte window) before the whole-ROM assertion
/// re-proves them in context. Note the two windows' bytes are
/// SHAPE-DEPENDENT even though the `.emp` sources are shape-invariant: the
/// `lea (pc)` / `bsr.w` displacements and the game-RAM `Cache_*` abs.w
/// addresses are link-resolved per shape.
#[test]
fn mixed_tranche3_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags, controllers_diags) = build_mixed_tranche3_rom(&aeon, false);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );
    assert!(
        controllers_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's six drift-guard ensures must all PASS (link succeeded): {controllers_diags:?}"
    );

    // The vdp_init block, pinned explicitly (the port's own 0x4C-byte
    // window). First four bytes: `lea.l BootData_VDPRegs(pc), a0` = 41FA
    // + disp16 ($3CE - $1C16 = -$1848 = $E7B8) — the cross-seam pc-rel EA.
    assert_eq!(
        &rom[0x1C14..0x1C60],
        &[
            0x41, 0xFA, 0xE7, 0xB8, 0x43, 0xF8, 0x80, 0x0A, 0x70, 0x12, 0x12, 0xD8, 0x51, 0xC8, 0xFF, 0xFC,
            0x70, 0x00, 0x21, 0xC0, 0x80, 0x1E, 0x4E, 0x75, 0x22, 0x38, 0x80, 0x1E, 0x67, 0x2C, 0x41, 0xF8,
            0x80, 0x0A, 0x43, 0xF9, 0x00, 0xC0, 0x00, 0x04, 0x30, 0x3C, 0x80, 0x00, 0x74, 0x00, 0x76, 0x12,
            0x05, 0x01, 0x67, 0x06, 0x10, 0x30, 0x20, 0x00, 0x32, 0x80, 0x06, 0x40, 0x01, 0x00, 0x52, 0x42,
            0x51, 0xCB, 0xFF, 0xEE, 0x70, 0x00, 0x21, 0xC0, 0x80, 0x1E, 0x4E, 0x75,
        ][..],
        "vdp_init block must match the reference bytes exactly (plain)"
    );

    // The collision_lookup block, pinned explicitly (the port's own
    // 0x32-byte window). Offset 0x26: `bsr.w Tile_Cache_GetCollision` =
    // 6100 + disp16 ($431E - $4C2E = -$910 = $F6F0) — the cross-seam
    // pc-relative CALL.
    assert_eq!(
        &rom[0x4C06..0x4C38],
        &[
            0x34, 0x01, 0xE6, 0x48, 0xB0, 0x78, 0xA8, 0x34, 0x6D, 0x24, 0xB0, 0x78, 0xA8, 0x36, 0x6E, 0x1E,
            0x3F, 0x00, 0x30, 0x02, 0xE6, 0x48, 0x32, 0x00, 0xB2, 0x78, 0xA8, 0x38, 0x6D, 0x0E, 0xB2, 0x78,
            0xA8, 0x3A, 0x6E, 0x08, 0x30, 0x1F, 0x61, 0x00, 0xF6, 0xF0, 0x4E, 0x75, 0x54, 0x8F, 0x70, 0x00,
            0x4E, 0x75,
        ][..],
        "collision_lookup block must match the reference bytes exactly (plain)"
    );

    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed tranche3");
}

/// Tranche 3 acceptance — `__DEBUG__` EIGHT-module mixed build ==
/// `aeon/s4.debug.bin`, modulo the five convsym bytes. Same composition as
/// the plain variant; the two new blocks' displacements and `Cache_*`
/// addresses take their debug-shape values.
#[test]
fn mixed_tranche3_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags, controllers_diags) = build_mixed_tranche3_rom(&aeon, true);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );
    assert!(
        controllers_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "engine.constants's six drift-guard ensures must all PASS (link succeeded): {controllers_diags:?}"
    );

    assert_eq!(
        &rom[0x1C96..0x1CE2],
        &[
            0x41, 0xFA, 0xE7, 0x3A, 0x43, 0xF8, 0x80, 0x0A, 0x70, 0x12, 0x12, 0xD8, 0x51, 0xC8, 0xFF, 0xFC,
            0x70, 0x00, 0x21, 0xC0, 0x80, 0x1E, 0x4E, 0x75, 0x22, 0x38, 0x80, 0x1E, 0x67, 0x2C, 0x41, 0xF8,
            0x80, 0x0A, 0x43, 0xF9, 0x00, 0xC0, 0x00, 0x04, 0x30, 0x3C, 0x80, 0x00, 0x74, 0x00, 0x76, 0x12,
            0x05, 0x01, 0x67, 0x06, 0x10, 0x30, 0x20, 0x00, 0x32, 0x80, 0x06, 0x40, 0x01, 0x00, 0x52, 0x42,
            0x51, 0xCB, 0xFF, 0xEE, 0x70, 0x00, 0x21, 0xC0, 0x80, 0x1E, 0x4E, 0x75,
        ][..],
        "vdp_init block must match the reference bytes exactly (debug)"
    );

    assert_eq!(
        &rom[0x542A..0x545C],
        &[
            0x34, 0x01, 0xE6, 0x48, 0xB0, 0x78, 0xA8, 0x56, 0x6D, 0x24, 0xB0, 0x78, 0xA8, 0x58, 0x6E, 0x1E,
            0x3F, 0x00, 0x30, 0x02, 0xE6, 0x48, 0x32, 0x00, 0xB2, 0x78, 0xA8, 0x5A, 0x6D, 0x0E, 0xB2, 0x78,
            0xA8, 0x5C, 0x6E, 0x08, 0x30, 0x1F, 0x61, 0x00, 0xF6, 0x38, 0x4E, 0x75, 0x54, 0x8F, 0x70, 0x00,
            0x4E, 0x75,
        ][..],
        "collision_lookup block must match the reference bytes exactly (debug)"
    );

    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed tranche3 debug",
    );
}

/// Plain (non-debug) mixed build == `aeon/s4.bin`, modulo the four convsym bytes.
#[test]
fn mixed_dac_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let rom = build_mixed_rom(&aeon, false);
    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed");
}

/// `__DEBUG__` mixed build == `aeon/s4.debug.bin`, modulo the five convsym bytes.
#[test]
fn mixed_dac_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let rom = build_mixed_rom(&aeon, true);
    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed debug",
    );
}

/// T2 acceptance — plain (non-debug) DAC+MT mixed build == `aeon/s4.bin`,
/// modulo the four convsym bytes. Both `SIGIL_EMP_DAC` and `SIGIL_EMP_MT` are
/// ON; both `.emp` modules are lowered and composed; the five `mt_bank.emp`
/// cross-seam ensures must genuinely run (via `check_link_asserts`) and pass.
#[test]
fn mixed_mt_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, assert_diags) = build_mixed_mt_rom(&aeon, false);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );
    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed MT");
}

/// T2 acceptance — `__DEBUG__` DAC+MT mixed build == `aeon/s4.debug.bin`,
/// modulo the five convsym bytes. Same composition as the plain variant, with
/// `DEBUG=1` driving both `mt_bank.emp`'s if-expressions and `__DEBUG__` on
/// the AS side.
#[test]
fn mixed_mt_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let (rom, assert_diags) = build_mixed_mt_rom(&aeon, true);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );
    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed MT debug",
    );
}

/// T3 acceptance — plain (non-debug) DAC+MT+SFX mixed build == `aeon/s4.bin`,
/// modulo the four convsym bytes. All three sound gates are ON; all three
/// `.emp` modules are lowered and composed; BOTH the five `mt_bank.emp` and the
/// one `sfx_bank.emp` cross-seam ensures must genuinely run (via
/// `check_link_asserts`) and pass. The composed ROM content is byte-identical to
/// the all-`.asm` build, so the SAME `ASSEMBLED_LEN`/`CONVSYM_REWRITTEN` pins as
/// the T1/T2 gates apply. This test also proves the win-tab `dw sfx_winptr`
/// deferral resolves end-to-end (see `build_mixed_sfx_rom`).
#[test]
fn mixed_sfx_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags) = build_mixed_sfx_rom(&aeon, false);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );

    // The win-tab deferral, PINNED end-to-end: `SfxBlobWinTab[0]` lives at Z80
    // vma `$845F` in the `phase 08000h` blob → ROM `$60000 + ($845F - $8000) =
    // $6045F`. `sfx_winptr(Sfx_33)` = `($63AE8 & $7FFF) | $8000 = $BAE8` → LE
    // bytes `E8 BA`. This resolved through the joint link from `Sfx_33`
    // (`.emp`-side) with `SFX_WIN_MASK`/`SFX_WIN_BASE` baked at AS-time
    // (`partial_fold`). The full-ROM assert below re-proves it against the
    // reference; this pin makes the seam's payload explicit.
    assert_eq!(
        &rom[0x6045F..0x60461],
        &[0xE8, 0xBA],
        "SfxBlobWinTab[0] = sfx_winptr(Sfx_33) must resolve to $BAE8 (LE `E8 BA`) via the joint link"
    );

    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed SFX");
}

/// T3 acceptance — `__DEBUG__` DAC+MT+SFX mixed build == `aeon/s4.debug.bin`,
/// modulo the five convsym bytes. Same three-module composition as the plain
/// variant, with `DEBUG=1` driving `mt_bank.emp`'s if-expressions and
/// `__DEBUG__` on the AS side; `sfx_bank.emp` is shape-invariant (its content is
/// identical in both shapes — only its map base moves, R7).
#[test]
fn mixed_sfx_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let (rom, mt_diags, sfx_diags) = build_mixed_sfx_rom(&aeon, true);
    assert!(
        mt_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {mt_diags:?}"
    );
    assert!(
        sfx_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sfx_bank.emp's cross-seam co-residency ensure must PASS (link succeeded): {sfx_diags:?}"
    );
    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed SFX debug",
    );
}

/// Count the deferred GUARD asserts, excluding the D2.29 [layout.odd-item]
/// parity asserts that now also ride module.link_asserts.
fn guard_assert_count(asserts: &[sigil_ir::LinkAssert]) -> usize {
    asserts
        .iter()
        .filter(|a| {
            !a.message.iter().any(|p| {
                matches!(p, sigil_ir::assert::MsgPart::Text(t) if t.contains("[layout.odd-item]"))
            })
        })
        .count()
}
