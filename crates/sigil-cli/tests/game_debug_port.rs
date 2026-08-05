//! Tranche 26 (lane B) — the REAL `game_debug.emp` port, the FIRST game-side
//! `.emp` CODE module. Post-flip (Stage 2): the AS twin `game_debug.asm` is
//! DELETED, so the twin-parity oracle retires; the surviving proofs gate on the
//! `.emp` alone plus the Config-A whole-ROM golden.
//!
//! game_debug is whole-file `ifdef SOUND_DEBUG_HOTKEYS` / `ifdef
//! SOUND_DRIVER_ENABLED` — it emits ZERO bytes in both canonical shapes
//! (SOUND_DEBUG_HOTKEYS is a dev opt-in, off in every shipped build) and is
//! placed only at the Config-A hotkeys shape, where the config_a whole-ROM native
//! golden (the provenance-chain tip) is the byte oracle for its emission. This file keeps the
//! `.emp`-side proofs: (1) at the hotkeys shape the module compiles, its
//! game-const drift guards PASS at the true values, and it emits non-empty code;
//! (2) doctoring a mirrored const's `extern(...)` truth FIRES the guard (liveness,
//! non-vacuous); (3) the two-module flip proves the extern-vs-import split — with
//! `game_loop.emp` compiled TOGETHER, its `jsr Debug_MusicToggle` resolves
//! module-to-module to this `.emp`'s proc (the old extern decl deleted, kill-list
//! row 33).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test game_debug_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

const BASE: u32 = 0x1000;
const REGION_SIZE: u32 = 0x200;

/// Cross-seam ADDRESS symbols (VMAs chosen to exercise the real addressing
/// modes: RAM $FFFF80xx → abs.w; the four Sound_* in ROM within bsr.w reach of
/// BASE). Shared by the .emp (as phased link carriers) and the AS twin (as equs).
fn addr_seam() -> Vec<(&'static str, u32)> {
    vec![
        ("Ctrl_1_Press", 0xFFFF8010),
        ("Dbg_Music_On", 0xFFFF8100),
        ("Dbg_Sfx_Sel", 0xFFFF8102),
        ("Sound_PlayMusic", 0x2000),
        ("Sound_PlaySFX", 0x2010),
        ("Sound_PlayRing", 0x2020),
        ("Sound_StopMusic", 0x2030),
        // SoundTest_BootPing (the Game.boot_hook impl, L1 P2) pings the driver.
        ("Sound_Ping", 0x2040),
    ]
}

/// Cross-seam VALUE symbols the standalone game_debug lower defers to the link:
/// the controller BUTTON_* masks, used as instruction immediates (link externs,
/// filled by these equ carriers). The song / SFX ids are NOT here — post-Parcel-F2
/// they are comptime `use`d from games.sonic4.sound_ids (the test provides them as
/// comptime defines, `id_defines`), so no equ carrier + no drift guard is involved.
fn value_seam() -> Vec<(&'static str, String)> {
    let v: Vec<(&'static str, i64)> = vec![
        ("BUTTON_UP", 0x01),
        ("BUTTON_B", 0x10),
        ("BUTTON_C", 0x20),
        ("BUTTON_A", 0x40),
        ("BUTTON_START", 0x80),
    ];
    v.into_iter().map(|(n, x)| (n, format!("${x:X}"))).collect()
}

/// The song / SFX id VALUES game_debug reads at COMPTIME (its DEBUG song
/// selector + the SfxIdRemap data table). Post-Parcel-F2 game_debug `use`s these from its
/// authority (games.sonic4.sound_ids); the standalone single-module lower resolves
/// them as comptime defines — the test analog of that `use` (the real build folds
/// the same values from the authority module). No local mirror + no drift guard
/// survives in game_debug, so these are plain comptime values, not extern carriers.
fn id_defines() -> Vec<(String, i128)> {
    [
        ("SONG_MOVINGTRUCKS", 1i128),
        ("SONG_DRUMTEST", 2),
        ("SONG_HCZ2", 3),
        ("SFXID_RING_RIGHT", 0x33),
        ("SFXID_DEATH", 0x35),
        ("SFXID_SKID", 0x36),
        ("SFXID_ROLL", 0x3C),
        ("SFXID_JUMP", 0x62),
        ("SFXID_SPINDASH", 0xAB),
        ("SFXID_DASH", 0xB6),
        ("SFXID_RINGLOSS", 0xB9),
    ]
    .into_iter()
    .map(|(n, v)| (n.to_string(), v))
    .collect()
}

/// Phased one-byte carriers for the address seam (the compression_selftest
/// addr_labels pattern).
fn addr_carriers(start_lma: u32) -> Vec<Section> {
    let mut out = Vec::new();
    let mut lma = start_lma;
    for (name, vma) in addr_seam() {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble (carrier {name}): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        out.extend(secs);
        lma += 0x1_0000;
    }
    out
}

/// Value-seam equ carriers (the BUTTON_* link externs game_debug defers).
fn value_carriers() -> Vec<Section> {
    let owned = value_seam();
    let pairs: Vec<(&str, &str)> = owned.iter().map(|(n, v)| (*n, v.as_str())).collect();
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// Compile game_debug.emp at the hotkeys shape, placed at BASE; return the
/// resolved sections + linked image + the module's link asserts (the drift
/// guards).
fn compile_emp() -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("games/sonic4/debug");
    let src = std::fs::read_to_string(dir.join("game_debug.emp"))
        .unwrap_or_else(|e| panic!("read game_debug.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_debug.emp parse errors: {pdiags:?}"
    );
    let mut defines = vec![
        ("SOUND_DEBUG_HOTKEYS".to_string(), 1i128),
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("DEBUG".to_string(), 1),
    ];
    defines.extend(id_defines());
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_debug.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let link_asserts = module.link_asserts.clone();
    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"game_debug\"\nlma_base = {BASE:#x}\nsize = {REGION_SIZE:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    sections.extend(addr_carriers(0x0100_0000));
    sections.extend(value_carriers());
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// THE `.emp` positive gate (post-twin-deletion): at the Config-A hotkeys shape
/// game_debug.emp compiles, its game-const drift guards PASS at the true values,
/// and it emits non-empty code. The byte-shape oracle is now the config_a
/// whole-ROM native golden (the provenance-chain tip); the non-vacuity is `doctored_extern_fires_
/// drift_guard` below.
#[test]
fn game_debug_emp_compiles_and_guards_pass_at_hotkeys() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, linked, link_asserts) = compile_emp();
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_debug.emp's surviving link asserts must PASS at the true values: {diags:?}"
    );
    let emp = linked.section("game_debug").expect("linked image carries game_debug").bytes.clone();
    assert!(!emp.is_empty(), "game_debug.emp must emit code at the hotkeys shape");
}

// RETIRED (Parcel F2): `doctored_extern_fires_drift_guard`. game_debug.emp's song /
// SFX id mirror consts + their `ensure(extern(...))` drift guards are GONE — the ids
// are now `use`d from their sole authority (games.sonic4.sound_ids), so there is no
// local mirror to drift and no game_debug guard to doctor. Drift-guard liveness for
// this contract re-homes to (a) the config_a whole-ROM native byte-identity golden,
// and (b) the SURVIVING authority guards on the engine side that CANNOT `use` a game
// module — sound_api.emp's `ensure(extern("SFXID_RING_*") == …)` and mt_bank.emp's
// `ensure(extern("SONG_MOVINGTRUCKS"/"SONG_COUNT") == …)`, resolved through the
// harvest EquSym (sfx_negative_probes / sound_migration cover their liveness).

/// The extern-vs-import split (kill-list row 33), gate-ON state: compile
/// game_loop.emp + game_debug.emp TOGETHER at the hotkeys shape. game_loop's
/// `jsr Debug_MusicToggle` must resolve MODULE-TO-MODULE to game_debug.emp's
/// proc (the extern decl is GONE). Proven by the jsr operand resolving to the
/// proc's placed VMA. Gate-OFF (game_loop.emp alone, synthetic AS symbol) is
/// game_loop_port::combo_matrix_matches_as_twin.
#[test]
fn two_module_flip_resolves_debug_music_toggle() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing");
        }
        eprintln!("skip: aeon sources missing");
        return;
    }
    let mut defines = vec![
        ("SOUND_DEBUG_HOTKEYS".to_string(), 1i128),
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("DEBUG".to_string(), 1),
    ];
    // game_debug reads the song / SFX ids at comptime (Parcel F2 `use` authority);
    // provide them as defines for this standalone two-module lower.
    defines.extend(id_defines());
    // game_loop.emp at a base within jsr abs.w reach isn't needed — jsr
    // Debug_MusicToggle is abs.w/.l by the width rule; we assert the emitted
    // operand equals game_debug's placed VMA. Place game_loop at $800, game_debug
    // at BASE.
    let gl_src = std::fs::read_to_string(aeon.join("engine/system/game_loop.emp"))
        .unwrap_or_else(|e| panic!("read game_loop.emp: {e}"));
    let (gl_file, _) = parse_str(&gl_src);
    // The Config-A hotkeys shape: the game binds `Game.debug_tick` to
    // Debug_MusicToggle, so game_loop's `invoke Game.debug_tick` lowers to a
    // `jsr Debug_MusicToggle` — exactly what this test resolves across the seam.
    let gl_env = sigil_harness::test_support::game_contract_env(
        "module engine.game_contract\n\
         pub interface Game {\n\
         \x20   hook debug_tick () clobbers(d0-d7/a0-a6) = empty\n\
         }\n",
        "module games.g.game\n\
         pub implement Game {\n\
         \x20   hook debug_tick = Debug_MusicToggle\n\
         }\n",
        &[],
    );
    let (gl_mod, gld) = lower_module_with_contracts(
        &gl_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/system")),
            embed_base: None,
            defines: defines.clone(),
        },
        &gl_env,
    );
    assert!(gld.iter().all(|d| d.level != sigil_span::Level::Error), "game_loop lower: {gld:?}");
    let gd_src = std::fs::read_to_string(aeon.join("games/sonic4/debug/game_debug.emp"))
        .unwrap_or_else(|e| panic!("read game_debug.emp: {e}"));
    let (gd_file, _) = parse_str(&gd_src);
    let (gd_mod, gdd) = lower_module(
        &gd_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("games/sonic4/debug")),
            embed_base: None,
            defines: defines.clone(),
        },
    );
    assert!(gdd.iter().all(|d| d.level != sigil_span::Level::Error), "game_debug lower: {gdd:?}");

    let map = format!(
        "fill = 0x00\n\n[[region]]\nname = \"game_loop\"\nlma_base = 0x800\nsize = 0x80\nkind = \"rom\"\n\n[[region]]\nname = \"game_debug\"\nlma_base = {BASE:#x}\nsize = {REGION_SIZE:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map).expect("map loads");
    let mut sections = gl_mod.sections;
    sections.extend(gd_mod.sections);
    place_sections(&mut sections, &map);
    sections.extend(addr_carriers(0x0100_0000));
    sections.extend(value_carriers());
    // game_loop's other cross-seam callees (VSync_Wait/Sound_DrainSfxRing/
    // Game_State) — synthetic carriers so the game_loop region links.
    for (i, (name, vma)) in
        [("VSync_Wait", 0x900u32), ("Sound_DrainSfxRing", 0x920), ("Input_Tick", 0x940), ("Logic_Tick", 0xFFFF8004), ("Game_State", 0xFFFF8008)]
            .iter()
            .enumerate()
    {
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs =
            assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS ({name}): {d:?}")).sections;
        for sec in &mut secs {
            sec.lma = 0x0500_0000 + (i as u32) * 0x1_0000;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(secs);
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("flip resolve failed: {d:?}"));
    // Debug_MusicToggle's placed VMA (the expected resolution).
    let dmt_vma = {
        let mut found = None;
        for sec in &resolved {
            if sec.name != "game_debug" {
                continue;
            }
            for l in &sec.labels {
                if l.name == "Debug_MusicToggle" {
                    found = Some(sec.vma_origin().wrapping_add(l.offset));
                }
            }
        }
        found.expect("game_debug.emp must export Debug_MusicToggle")
    };
    assert_eq!(dmt_vma, BASE, "Debug_MusicToggle is the game_debug region's first proc (at BASE)");
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("flip link failed: {d:?}"));
    // The `jsr Debug_MusicToggle` in GameLoop: find the abs operand in game_loop's
    // bytes that equals dmt_vma. jsr abs.w = 4E B8 <word>; abs.l = 4E B9 <long>.
    let gl = linked.section("game_loop").expect("game_loop region");
    let bytes = &gl.bytes;
    let mut resolved_to_proc = false;
    for i in 0..bytes.len().saturating_sub(3) {
        if bytes[i] == 0x4E && bytes[i + 1] == 0xB8 {
            let w = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as i16 as i64 as u32;
            if w == dmt_vma {
                resolved_to_proc = true;
            }
        }
        if bytes[i] == 0x4E && bytes[i + 1] == 0xB9 && i + 5 < bytes.len() {
            let l = u32::from_be_bytes([bytes[i + 2], bytes[i + 3], bytes[i + 4], bytes[i + 5]]);
            if l == dmt_vma {
                resolved_to_proc = true;
            }
        }
    }
    assert!(
        resolved_to_proc,
        "GameLoop's `jsr Debug_MusicToggle` must resolve module-to-module to game_debug.emp's proc at {dmt_vma:#x}; game_loop bytes: {:02x?}",
        bytes
    );
    // Keep the pins import live (BASE-relative sanity vs the campaign convention).
    let _ = pins::GAME_LOOP.plain_base;
}
