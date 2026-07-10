//! Tranche 5 — negative probes for `game_loop_port.rs` + `sound_api_port.rs`
//! (the house one-file-per-tranche style).
//!
//! (a) missing cross-seam symbol is LOUD — a doctored copy misspelling
//!     `Sound_DrainSfxRing` fails to resolve/link rather than emitting
//!     silently-wrong displacement bytes.
//! (b) oversize-combo overlap is LOUD — the gate's engine.inc resume org
//!     sits 0x12 bytes past the region base (the (1,0) combo both reference
//!     shapes carry); the hotkeys-on combo emits 0x16 bytes, so a build that
//!     flips `SOUND_DEBUG_HOTKEYS=1` against the pinned layout runs the
//!     section INTO the AS-side resume bytes — refused at resolve/link
//!     (placement itself doesn't police region budgets; overlap detection
//!     is the enforcement — `place_sections`' §7.3 note).
//! (c) define-genuineness — `SOUND_DRIVER_ENABLED=0` produces DIFFERENT
//!     bytes than the reference window (the comptime `if` is load-bearing;
//!     the byte-diff gate is non-vacuous).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    std::env::var("AEON_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/volence/sonic_hacks/aeon"))
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

fn real_src() -> Option<String> {
    let path = aeon_dir().join("engine/system/game_loop.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// Lower `src` with the given defines and place into a `game_loop` region of
/// `region_size` at the plain base. Returns (sections, all diagnostics).
fn lower_and_place(
    src: &str,
    defines: &[(&str, i128)],
    region_size: u32,
) -> (Vec<sigil_ir::Section>, Vec<sigil_span::Diagnostic>) {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse: {pdiags:?}");
    let (module, mut diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
        },
    );
    let map_toml = format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"game_loop\"\n\
         lma_base = 0x22FE\n\
         size = {region_size:#x}\n\
         kind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let mut sections = module.sections;
    diags.extend(place_sections(&mut sections, &map));
    (sections, diags)
}

/// Synthetic sections supplying the four cross-seam labels the real file
/// reads, at harness-private positions (the probes don't diff bytes against
/// the reference, so exact positions are irrelevant — presence is the point).
fn synthetic_labels(names: &[&str]) -> Vec<sigil_ir::Section> {
    use sigil_frontend_as::{assemble, Options as AsOptions};
    let mut out = Vec::new();
    let mut lma = 0x0200_0000u32;
    for name in names {
        let asm = format!("cpu 68000\nphase $4000\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}"))
            .sections;
        for sec in &mut secs {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        out.extend(secs);
        lma += 0x10_0000;
    }
    out
}

const ON_DEFINES: [(&str, i128); 2] = [("SOUND_DRIVER_ENABLED", 1), ("SOUND_DEBUG_HOTKEYS", 0)];

/// (a) A doctored copy misspelling the drain target fails LOUD at
/// resolve/link — never silent bytes.
#[test]
fn misspelled_cross_seam_symbol_is_loud() {
    let Some(src) = real_src() else { return };
    let doctored = src.replace("Sound_DrainSfxRing", "Sound_DrainSfxRungg");
    assert_ne!(src, doctored, "the probe must actually doctor the source");

    let (mut sections, diags) = lower_and_place(&doctored, &ON_DEFINES, 0x12);
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
    // Supply the CORRECT names only — the doctored reference dangles.
    sections.extend(synthetic_labels(&[
        "VSync_Wait",
        "Sound_DrainSfxRing",
        "Game_State",
    ]));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true);
    let loud = match resolved {
        Err(_) => true,
        Ok(resolved) => sigil_link::link(&resolved, &SymbolTable::new()).is_err(),
    };
    assert!(loud, "a misspelled cross-seam symbol must fail resolve or link, not emit");
}

/// (b) The hotkeys-on combo (0x16 bytes) collides with the AS-side bytes at
/// the engine.inc resume org (base + 0x12) — refused at resolve/link, never
/// truncated or silently shifted.
#[test]
fn oversize_combo_overlapping_resume_bytes_is_loud() {
    let Some(src) = real_src() else { return };
    let (mut sections, diags) = lower_and_place(
        &src,
        &[("SOUND_DRIVER_ENABLED", 1), ("SOUND_DEBUG_HOTKEYS", 1)],
        0x12,
    );
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
    sections.extend(synthetic_labels(&[
        "VSync_Wait",
        "Sound_DrainSfxRing",
        "Game_State",
        "Debug_MusicToggle",
    ]));
    // The AS side resumes at $2310 (engine.inc's gate-else org) — simulate
    // its first bytes with a pinned carrier there.
    let mut resume = synthetic_labels(&["S4lz_Decompress"]);
    for sec in &mut resume {
        sec.lma = 0x2310;
    }
    sections.extend(resume);

    let loud = match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
        Err(_) => true,
        Ok(resolved) => sigil_link::link(&resolved, &SymbolTable::new()).is_err(),
    };
    assert!(
        loud,
        "the 0x16-byte hotkeys-on body must collide loudly with the resume bytes at $2310"
    );
}

/// (c) `SOUND_DRIVER_ENABLED=0` genuinely changes the bytes (the comptime
/// `if` is load-bearing): the off-combo body is 4 bytes shorter than the
/// pinned reference window.
#[test]
fn drain_define_is_load_bearing() {
    let Some(src) = real_src() else { return };
    let (mut sections, diags) = lower_and_place(
        &src,
        &[("SOUND_DRIVER_ENABLED", 0), ("SOUND_DEBUG_HOTKEYS", 0)],
        0x12,
    );
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
    sections.extend(synthetic_labels(&["VSync_Wait", "Sound_DrainSfxRing", "Game_State"]));
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    let section = linked.section("game_loop").expect("game_loop section");
    assert_eq!(
        section.bytes.len(),
        0x12 - 4,
        "the sound-off combo must drop exactly the 4-byte bsr.w drain line"
    );
}

// ---- sound_api (tranche-5 port #2) ----------------------------------------

fn sound_api_src() -> Option<String> {
    let path = aeon_dir().join("engine/sound/sound_api.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but {} missing", path.display()),
        Err(_) => {
            eprintln!("skip: {} not found (set AEON_DIR)", path.display());
            None
        }
    }
}

/// Lower the (possibly doctored) sound_api source standalone. Returns the
/// module's link asserts plus lower diagnostics; the caller decides which
/// failure surface it is probing.
fn lower_sound_api(
    src: &str,
) -> (sigil_ir::Module, Vec<sigil_ir::LinkAssert>, Vec<sigil_span::Diagnostic>) {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse: {pdiags:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let mut m = module;
    let asserts = std::mem::take(&mut m.link_asserts);
    (m, asserts, diags)
}

/// The full AS-side truth composition (equs + labels) shared by the sound_api
/// probes — everything the real file reads, at representative positions.
fn sound_api_truth_sections() -> Vec<sigil_ir::Section> {
    use sigil_frontend_as::{assemble, Options as AsOptions};
    let asm = "cpu 68000\n\
               Z80_BUS_REQUEST = $A11100\n\
               SND_Z80_BASE = $A00000\n\
               SND_STAT_ALIVE = $1F10\n\
               SND_REQ_PING = $1F00\n\
               SND_REQ_SAMPLE = $1F01\n\
               SND_REQ_MUSIC = $1F02\n\
               SND_REQ_SFX = $1F03\n\
               SND_REQ_FADE = $1F05\n\
               SND_REQ_TEMPO = $1F06\n\
               SND_MUSIC_PARAM_BANK = $1CA6\n\
               SND_MUSIC_PARAM_PTR = $1CA7\n\
               SND_MUSIC_PARAM_FLAGS = $1CA9\n\
               SND_MUSIC_PARAM_PATCHPTR = $1CAA\n\
               SND_ALIVE_MARKER = $5A\n\
               SND_MUSIC_STOP = $FF\n\
               SND_FADE_CMD_OUT = 1\n\
               SND_FADE_CMD_IN = 2\n\
               SFX_RING_MASK = $07\n\
               SFXID_RING_RIGHT = $33\n\
               SFXID_RING_LEFT = $34\n\
               phase $FFFFAF30\n\
               Ring_Sfx_Speaker:\n\
               \tdc.b 0\n\
               \tdc.b 0\n\
               Sfx_Ring_Buf:\n\
               \tdc.b 0,0,0,0,0,0,0,0\n\
               Sfx_Ring_Wr:\n\
               \tdc.b 0\n\
               Sfx_Ring_Rd:\n\
               \tdc.b 0\n\
               dephase\n\
               phase $63AE0\n\
               SongTable:\n\
               \tdc.l 0\n\
               SongPatchTable:\n\
               \tdc.l 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut truth = assemble(asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (truth): {d:?}"))
        .sections;
    let mut lma = 0x0100_0000u32;
    for sec in &mut truth {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        lma += 0x10_0000;
    }
    truth
}

/// (d) drift-guard genuineness — a doctored `SND_ALIVE_MARKER` mirror makes
/// its `ensure` FAIL against the AS-side truth (supplied synthetically), so
/// a drifted immediate can never link silently.
#[test]
fn doctored_immediate_mirror_fails_its_drift_guard() {
    let Some(src) = sound_api_src() else { return };
    let doctored = src.replace(
        "const SND_ALIVE_MARKER  = $5A",
        "const SND_ALIVE_MARKER  = $5B",
    );
    assert_ne!(src, doctored, "the probe must actually doctor the source");
    let (module, asserts, diags) = lower_sound_api(&doctored);
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower: {diags:?}");

    // Supply the full AS-side truth (equs + the cross-seam labels) so the
    // composition RESOLVES — the doctored mirror must then fail its guard,
    // not the link.
    use sigil_frontend_as::{assemble, Options as AsOptions};
    let asm = "cpu 68000\n\
               Z80_BUS_REQUEST = $A11100\n\
               SND_Z80_BASE = $A00000\n\
               SND_STAT_ALIVE = $1F10\n\
               SND_REQ_PING = $1F00\n\
               SND_REQ_SAMPLE = $1F01\n\
               SND_REQ_MUSIC = $1F02\n\
               SND_REQ_SFX = $1F03\n\
               SND_REQ_FADE = $1F05\n\
               SND_REQ_TEMPO = $1F06\n\
               SND_MUSIC_PARAM_BANK = $1CA6\n\
               SND_MUSIC_PARAM_PTR = $1CA7\n\
               SND_MUSIC_PARAM_FLAGS = $1CA9\n\
               SND_MUSIC_PARAM_PATCHPTR = $1CAA\n\
               SND_ALIVE_MARKER = $5A\n\
               SND_MUSIC_STOP = $FF\n\
               SND_FADE_CMD_OUT = 1\n\
               SND_FADE_CMD_IN = 2\n\
               SFX_RING_MASK = $07\n\
               SFXID_RING_RIGHT = $33\n\
               SFXID_RING_LEFT = $34\n\
               phase $FFFFAF30\n\
               Ring_Sfx_Speaker:\n\
               \tdc.b 0\n\
               \tdc.b 0\n\
               Sfx_Ring_Buf:\n\
               \tdc.b 0,0,0,0,0,0,0,0\n\
               Sfx_Ring_Wr:\n\
               \tdc.b 0\n\
               Sfx_Ring_Rd:\n\
               \tdc.b 0\n\
               dephase\n\
               phase $63AE0\n\
               SongTable:\n\
               \tdc.l 0\n\
               SongPatchTable:\n\
               \tdc.l 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let mut sections = module.sections;
    // Give the emp section a placement so the union resolves.
    let map_toml = "fill = 0x00\n\
                    \n\
                    [[region]]\n\
                    name = \"sound_api\"\n\
                    lma_base = 0x5D94\n\
                    size = 0x1E8\n\
                    kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place: {pdiags:?}");
    let mut truth = assemble(asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (truth): {d:?}"))
        .sections;
    let mut lma = 0x0100_0000u32;
    for sec in &mut truth {
        sec.lma = lma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        lma += 0x10_0000;
    }
    sections.extend(truth);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout must succeed (only the GUARD drifts): {d:?}"));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &asserts);
    assert!(
        diags.iter().any(|d| d.level == Level::Error
            && d.message.contains("SND_ALIVE_MARKER drifted")),
        "the doctored mirror must fail its drift guard, got: {diags:?}"
    );
}

/// (e) a misspelled extern in a slot equ dangles at resolve/link — never a
/// silent wrong address. Non-vacuity: the SAME truth composition first
/// resolves the UNDOCTORED source cleanly, so the failure is provably the
/// one misspelled name, not a generally-dangling link.
#[test]
fn misspelled_extern_slot_is_loud() {
    let Some(src) = sound_api_src() else { return };

    fn resolves(src: &str) -> bool {
        let (module, _asserts, diags) = lower_sound_api(src);
        if diags.iter().any(|d| d.level == Level::Error) {
            return false;
        }
        let mut sections = module.sections;
        let map_toml = "fill = 0x00\n\
                        \n\
                        [[region]]\n\
                        name = \"sound_api\"\n\
                        lma_base = 0x5D94\n\
                        size = 0x1E8\n\
                        kind = \"rom\"\n";
        let map = sigil_link::load_map(map_toml).expect("map must load");
        let pdiags = place_sections(&mut sections, &map);
        if pdiags.iter().any(|d| d.level == Level::Error) {
            return false;
        }
        sections.extend(sound_api_truth_sections());
        match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
            Err(_) => false,
            Ok(resolved) => sigil_link::link(&resolved, &SymbolTable::new()).is_ok(),
        }
    }

    assert!(resolves(&src), "control: the undoctored source must resolve against the truth");
    let doctored = src.replace("extern(\"SND_REQ_MUSIC\")", "extern(\"SND_REQ_MUSICC\")");
    assert_ne!(src, doctored, "the probe must actually doctor the source");
    assert!(
        !resolves(&doctored),
        "the misspelled extern must dangle loudly while every correct name resolves"
    );
}
