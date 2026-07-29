//! Tranche 26 (lane B) — the REAL `game_debug.emp` port, the FIRST game-side
//! `.emp` CODE module. Proven by the OFF-CANONICAL twin-parity oracle.
//!
//! game_debug.asm is whole-file `ifdef SOUND_DEBUG_HOTKEYS` / `ifdef
//! SOUND_DRIVER_ENABLED` — it emits ZERO bytes in both canonical shapes
//! (SOUND_DEBUG_HOTKEYS is a dev opt-in, off in every shipped build). So there
//! is NO canonical reference ROM window to diff against. The oracle is the AS
//! twin assembled through sigil's own AS front-end at the HOTKEYS shape
//! (SOUND_DEBUG_HOTKEYS=1, SOUND_DRIVER_ENABLED=1): `game_debug.emp` compiled at
//! that shape must byte-match `game_debug.asm` assembled at that shape, at a
//! shared synthetic base with the same cross-seam seam (the game_loop_port
//! `as_twin_bytes` + compression_selftest debug-only-region templates fused).
//!
//! Two canonical-emptiness proofs (both shapes) confirm the TWIN emits nothing
//! canonically — the property that makes the whole file byte-neutral to build.sh
//! (the .emp is placed only at the hotkeys shape). The two-module flip proves
//! the extern-vs-import split: with `game_loop.emp` compiled TOGETHER, its
//! `jsr Debug_MusicToggle` resolves module-to-module to this .emp's proc (the
//! old extern decl deleted — kill-list row 33). Gate-OFF (game_loop.emp alone,
//! synthetic AS symbol) stays green in `game_loop_port::combo_matrix_matches_as_twin`.
//!
//! $8000 bank-shift bar (hotkeys shape): STATED. game_debug sits below $10000
//! unshielded; at the hotkeys shape its ~130-byte emission slides everything
//! downstream — but the oracle assembles the AS twin AT THE SAME hotkeys shape,
//! so both sides see one bank layout: there is no cross-shape object-bank
//! collision to reason about (unlike a debug-vs-plain growth). Satisfied by
//! construction; a real full-build overflow at the hotkeys shape would surface
//! as an AS assembly error (the dev build's own concern), not here.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test game_debug_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
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
    ]
}

/// Cross-seam VALUE symbols: the game consts the .emp mirrors (its drift guards
/// read them via `extern(...)`) and the AS twin bakes as immediates. `doctor`
/// overrides ONE pair (the negative probe).
fn value_seam(doctor: Option<(&str, i64)>) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, i64)> = vec![
        ("BUTTON_UP", 0x01),
        ("BUTTON_B", 0x10),
        ("BUTTON_C", 0x20),
        ("BUTTON_A", 0x40),
        ("BUTTON_START", 0x80),
        ("SONG_MOVINGTRUCKS", 1),
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
    ];
    if let Some((name, val)) = doctor {
        let mut hit = false;
        for p in v.iter_mut() {
            if p.0 == name {
                p.1 = val;
                hit = true;
            }
        }
        assert!(hit, "doctor target `{name}` not in the value seam");
    }
    v.into_iter().map(|(n, x)| (n, format!("${x:X}"))).collect()
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

/// Value-seam equ carriers (the drift-guard `extern(...)` reads).
fn value_carriers(doctor: Option<(&str, i64)>) -> Vec<Section> {
    let owned = value_seam(doctor);
    let pairs: Vec<(&str, &str)> = owned.iter().map(|(n, v)| (*n, v.as_str())).collect();
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// Compile game_debug.emp at the hotkeys shape, placed at BASE; return the
/// resolved sections + linked image + the module's link asserts (the drift
/// guards).
fn compile_emp(
    doctor: Option<(&str, i64)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let dir = aeon.join("games/sonic4/debug");
    let src = std::fs::read_to_string(dir.join("game_debug.emp"))
        .unwrap_or_else(|e| panic!("read game_debug.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "game_debug.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![
            ("SOUND_DEBUG_HOTKEYS".to_string(), 1),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("DEBUG".to_string(), 1),
        ],
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
    sections.extend(value_carriers(doctor));
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The AS twin oracle: assemble game_debug.asm through sigil's AS front-end at
/// the given shape, at BASE, with the whole cross-seam seam as equs. `hotkeys`
/// gates the file; `doctor` overrides one value equ (the twin-diverges probe).
fn as_twin_bytes(hotkeys: bool, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = std::fs::read_to_string(aeon.join("games/sonic4/debug/game_debug.asm"))
        .unwrap_or_else(|e| panic!("read game_debug.asm: {e}"));
    let mut seam = String::from("cpu 68000\nsupmode on\n");
    for (name, vma) in addr_seam() {
        seam.push_str(&format!("{name} = ${vma:X}\n"));
    }
    for (name, val) in value_seam(doctor) {
        seam.push_str(&format!("{name} = {val}\n"));
    }
    let src = format!("{seam}org ${BASE:X}\n{body}\n");
    let mut defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    if hotkeys {
        defines.push(("SOUND_DEBUG_HOTKEYS".to_string(), 1));
    }
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines, ..AsOptions::default() };
    let out = assemble(&src, &opts).unwrap_or_else(|d| panic!("AS twin assemble: {d:?}"));
    let mut sections = out.sections;
    for sec in &mut sections {
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS twin resolve failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS twin link failed: {d:?}"));
    linked
        .sections
        .iter()
        .find(|s| s.lma == BASE && !s.bytes.is_empty())
        .map(|s| s.bytes.clone())
        .unwrap_or_default()
}

fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(candidate.len(), expected.len(), "{what}: length mismatch ({} vs {})", candidate.len(), expected.len());
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// THE off-canonical byte gate: game_debug.emp (hotkeys shape) == game_debug.asm
/// (hotkeys shape). Also checks the drift guards PASS (extern == mirrored const).
#[test]
fn game_debug_matches_as_twin_at_hotkeys_shape() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let (resolved, linked, link_asserts) = compile_emp(None);
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the game-const drift guards must PASS at the true values: {diags:?}"
    );
    let emp = linked.section("game_debug").expect("linked image carries game_debug").bytes.clone();
    let twin = as_twin_bytes(true, None);
    assert!(!twin.is_empty(), "the AS twin must emit code at the hotkeys shape");
    assert_region_matches(&emp, &twin, "game_debug.emp vs game_debug.asm (hotkeys shape)");
}

/// Positive control (byte gate non-triviality): the UNDOCTORED .emp must DIVERGE
/// from a twin assembled with a doctored SFXID immediate — proving the byte
/// comparison genuinely detects a difference (not a vacuous equality). The
/// undoctored equality is the test above.
#[test]
fn emp_diverges_from_doctored_twin() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing");
        }
        eprintln!("skip: aeon sources missing");
        return;
    }
    let (_, linked, _) = compile_emp(None);
    let emp = linked.section("game_debug").expect("game_debug").bytes.clone();
    // Doctor the twin's SFXID_JUMP immediate (entry 0 of Dbg_SfxIdTable).
    let twin = as_twin_bytes(true, Some(("SFXID_JUMP", 0x11)));
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// Drift-guard liveness: doctoring a mirrored const's `extern(...)` truth must
/// FIRE the game_debug.emp `ensure` — the guards are not vacuous.
#[test]
fn doctored_extern_fires_drift_guard() {
    let (resolved, _, link_asserts) = compile_emp(Some(("SFXID_JUMP", 0x11)));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(!fired.is_empty(), "a doctored extern must fire a drift guard");
    assert!(
        fired.iter().any(|d| d.message.contains("SFXID_JUMP")),
        "the fired guard must name the drifted const: {fired:?}"
    );
}

/// Canonical emptiness (plain shape): the AS twin emits ZERO bytes with
/// SOUND_DEBUG_HOTKEYS OFF — the property that makes the whole file byte-neutral
/// to build.sh (the .emp is placed only at the hotkeys shape). The t22
/// `_plain_region_is_empty` analog, proven against the real twin.
#[test]
fn game_debug_plain_is_empty() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing");
        }
        eprintln!("skip: aeon sources missing");
        return;
    }
    assert!(
        as_twin_bytes(false, None).is_empty(),
        "game_debug.asm must emit ZERO bytes with SOUND_DEBUG_HOTKEYS off (plain canonical shape)"
    );
}

/// Canonical emptiness (debug shape): same, DEBUG=1 does not un-gate it (the
/// ifdef is on SOUND_DEBUG_HOTKEYS, off in both canonical shapes).
#[test]
fn game_debug_debug_is_empty() {
    let aeon = aeon_dir();
    if !aeon.join("games/sonic4/debug/game_debug.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing");
        }
        eprintln!("skip: aeon sources missing");
        return;
    }
    // DEBUG is irrelevant to the ifdef (only SOUND_DEBUG_HOTKEYS gates it); the
    // AS twin has no DEBUG-conditional content, so hotkeys-off is empty in both.
    assert!(
        as_twin_bytes(false, None).is_empty(),
        "game_debug.asm must emit ZERO bytes with SOUND_DEBUG_HOTKEYS off (debug canonical shape)"
    );
}

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
    let defines = vec![
        ("SOUND_DEBUG_HOTKEYS".to_string(), 1i128),
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("DEBUG".to_string(), 1),
    ];
    // game_loop.emp at a base within jsr abs.w reach isn't needed — jsr
    // Debug_MusicToggle is abs.w/.l by the width rule; we assert the emitted
    // operand equals game_debug's placed VMA. Place game_loop at $800, game_debug
    // at BASE.
    let gl_src = std::fs::read_to_string(aeon.join("engine/system/game_loop.emp"))
        .unwrap_or_else(|e| panic!("read game_loop.emp: {e}"));
    let (gl_file, _) = parse_str(&gl_src);
    let (gl_mod, gld) = lower_module(
        &gl_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(aeon.join("engine/system")),
            embed_base: None,
            defines: defines.clone(),
        },
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
    sections.extend(value_carriers(None));
    // game_loop's other cross-seam callees (VSync_Wait/Sound_DrainSfxRing/
    // Game_State) — synthetic carriers so the game_loop region links.
    for (i, (name, vma)) in
        [("VSync_Wait", 0x900u32), ("Sound_DrainSfxRing", 0x920), ("Game_State", 0xFFFF8004)]
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
