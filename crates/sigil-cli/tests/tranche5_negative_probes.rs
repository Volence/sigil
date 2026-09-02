//! Tranche 5 — negative probes for `game_loop_port.rs` + `sound_api_port.rs`
//! (the house one-file-per-tranche style).
//!
//! (a) missing cross-seam symbol is LOUD — a doctored copy misspelling
//!     `Sound_DrainSfxRing` fails to resolve/link rather than emitting
//!     silently-wrong displacement bytes.
//! (b) oversize-combo overlap is LOUD — the gate's engine.inc resume org
//!     sits 0x16 bytes past the region base (the (1,0) combo both reference
//!     shapes carry — 0x12 pre-I2, +4 for I2's unconditional `addq.l #1,
//!     Logic_Tick`); the hotkeys-on combo emits 0x1A bytes, so a build that
//!     flips `SOUND_DEBUG_HOTKEYS=1` against the pinned layout runs the
//!     section INTO the AS-side resume bytes — refused at resolve/link
//!     (placement itself doesn't police region budgets; overlap detection
//!     is the enforcement — `place_sections`' §7.3 note).
//! (c) define-genuineness — `SOUND_DRIVER_ENABLED=0` produces DIFFERENT
//!     bytes than the reference window (the comptime `if` is load-bearing;
//!     the byte-diff gate is non-vacuous).

use sigil_frontend_emp::lower::{lower_module, lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
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
    // The game-contract env (L1 P2). game_loop.emp names `invoke Game.debug_tick`;
    // whether it emits a `jsr Debug_MusicToggle` or nothing is decided by the
    // MANIFEST binding, not game_loop's `-D`. Reproduce the real manifest's
    // conditional bind against THESE defines, so the hotkeys-on combo binds the
    // hook (the jsr fires) and every other combo leaves it `= empty` — exactly
    // the byte behavior these probes exercise.
    let owned_defines: Vec<(String, i128)> =
        defines.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    let env = sigil_harness::test_support::game_contract_env(
        "module engine.game_contract\n\
         pub interface Game {\n\
         \x20   hook debug_tick () clobbers(d0-d7/a0-a6) = empty\n\
         }\n",
        "module games.g.game\n\
         pub implement Game {\n\
         \x20   if SOUND_DEBUG_HOTKEYS == 1 && SOUND_DRIVER_ENABLED == 1 {\n\
         \x20       hook debug_tick = Debug_MusicToggle\n\
         \x20   }\n\
         }\n",
        &owned_defines,
    );
    let (module, mut diags) = lower_module_with_contracts(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: owned_defines.clone(),
        },
        &env,
    );
    let map_toml = format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"game_loop\"\n\
         lma_base = {base:#x}\n\
         size = {region_size:#x}\n\
         kind = \"rom\"\n",
        base = pins::GAME_LOOP.plain_base
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

/// The `(SOUND_DRIVER_ENABLED=1, SOUND_DEBUG_HOTKEYS=0)` window: the extent
/// `engine.inc` gives the emp GameLoop before its gate-else `org` resumes the
/// AS side. The hotkeys-ON body is larger, which is what probe (b) collides.
const ON_OFF_WINDOW: u32 = 0x16;

/// (a) A doctored copy misspelling the drain target fails LOUD at
/// resolve/link — never silent bytes.
#[test]
fn misspelled_cross_seam_symbol_is_loud() {
    let Some(src) = real_src() else { return };
    let doctored = src.replace("Sound_DrainSfxRing", "Sound_DrainSfxRungg");
    assert_ne!(src, doctored, "the probe must actually doctor the source");

    let (mut sections, diags) = lower_and_place(&doctored, &ON_DEFINES, ON_OFF_WINDOW);
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
    // Supply the CORRECT names only — the doctored reference dangles.
    sections.extend(synthetic_labels(&[
        "VSync_Wait",
        "Sound_DrainSfxRing",
        "Input_Tick",           // I3 replay seam
        // Effects P2: GameLoop's jbsr Palette_Compose. MUST be supplied here even
        // though this probe is about a DIFFERENT symbol: the assert only checks that
        // resolve/link failed, so any unrelated unresolved name satisfies it for the
        // wrong reason and the probe silently stops testing the misspelling.
        "Palette_Compose",
        "Logic_Tick",
        "Game_State",
    ]));

    // PIN THE CAUSE, do not just assert "something failed". A bare `loud` check is
    // satisfied by ANY unresolved name, which is how this probe went vacuous when
    // game_loop.emp gained `Palette_Compose` (the supplied set above is the fix, but a
    // supplied set is a moving target — the next cross-seam callee breaks it again).
    // Requiring the diagnostic to NAME the doctored symbol makes the probe self-detecting:
    // it can only pass for its own reason. Pattern borrowed from tranche6's
    // objroutine_typo probe.
    let msgs = match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
        Err(d) => d.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        Ok(resolved) => match sigil_link::link(&resolved, &SymbolTable::new()) {
            Err(d) => d.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
            Ok(_) => Vec::new(),
        },
    };
    assert!(
        msgs.iter().any(|m| m.contains("Sound_DrainSfxRungg")),
        "a misspelled cross-seam symbol must fail resolve or link NAMING the typo, not \
         emit — and not fail for some unrelated unresolved symbol: {msgs:?}"
    );
}

/// (b) The hotkeys-on combo (0x1A bytes post-I2) collides with the AS-side bytes
/// at the engine.inc resume org (base + 0x16) — refused at resolve/link, never
/// truncated or silently shifted.
#[test]
fn oversize_combo_overlapping_resume_bytes_is_loud() {
    let Some(src) = real_src() else { return };
    let (mut sections, diags) = lower_and_place(
        &src,
        &[("SOUND_DRIVER_ENABLED", 1), ("SOUND_DEBUG_HOTKEYS", 1)],
        ON_OFF_WINDOW,
    );
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
    sections.extend(synthetic_labels(&[
        "VSync_Wait",
        "Sound_DrainSfxRing",
        "Input_Tick",           // I3 replay seam
        // Effects P2 — same vacuity hazard as probe (a): this assert only checks that
        // resolve/link failed, so an unresolved Palette_Compose would "pass" it without
        // the resume-byte overlap ever being exercised.
        "Palette_Compose",
        "Logic_Tick",
        "Game_State",
        "Debug_MusicToggle",
    ]));
    // The AS side resumes at `engine.inc`'s gate-else org — the region base plus
    // the (1,0) window — so a pinned carrier there is exactly the first byte the
    // oversized hotkeys-ON body must not reach. Both halves derive, so the
    // collision stays adjacent to the real body wherever the cartridge puts it.
    let resume_lma = pins::GAME_LOOP.plain_base + ON_OFF_WINDOW;
    let mut resume = synthetic_labels(&["S4lz_Decompress"]);
    for sec in &mut resume {
        sec.lma = resume_lma;
    }
    sections.extend(resume);

    // PIN THE CAUSE (same hazard as probe (a) above): a bare `loud` check is satisfied by
    // any unresolved symbol, so require the diagnostic to be about the COLLISION.
    let msgs = match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
        Err(d) => d.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        Ok(resolved) => match sigil_link::link(&resolved, &SymbolTable::new()) {
            Err(d) => d.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
            Ok(_) => Vec::new(),
        },
    };
    assert!(
        msgs.iter().any(|m| m.contains("overlap")),
        "the hotkeys-on body must collide loudly with the resume bytes at {resume_lma:#X}, \
         and the diagnostic must say so rather than the probe passing on an unrelated \
         unresolved symbol: {msgs:?}"
    );
}

/// (c) `SOUND_DRIVER_ENABLED=0` genuinely changes the bytes (the comptime
/// `if` is load-bearing): the off-combo body is 4 bytes shorter than the
/// pinned reference window.
#[test]
fn drain_define_is_load_bearing() {
    let Some(src) = real_src() else { return };
    // Emit the real GameLoop region for a given SOUND_DRIVER_ENABLED value and
    // return its byte length. Comparing on-vs-off directly is robust against the
    // GAME_LOOP region pin's trailing align pad (0x1C = 0x1A emitted + a 2-byte pad
    // post-I3; the padded pin would make a literal `- 4` wrong).
    let emitted = |sound_on: i128| -> usize {
        let (mut sections, diags) = lower_and_place(
            &src,
            &[("SOUND_DRIVER_ENABLED", sound_on), ("SOUND_DEBUG_HOTKEYS", 0)],
            pins::GAME_LOOP.plain_len as u32,
        );
        assert!(diags.iter().all(|d| d.level != Level::Error), "lower/place: {diags:?}");
        sections.extend(synthetic_labels(&["VSync_Wait", "Sound_DrainSfxRing", "Input_Tick", "Palette_Compose", "Logic_Tick", "Game_State"]));
        let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
        linked.section("game_loop").expect("game_loop section").bytes.len()
    };
    assert_eq!(
        emitted(1) - emitted(0),
        4,
        "the sound-off combo must drop exactly the 4-byte bsr.w drain line"
    );
}

// ---- sound_api (tranche-5 port #2) ----------------------------------------

/// The one-region map both sound_api probes place into. Base and extent read
/// `pins` (repin regenerates them from the build's own listing), so neither can
/// be left behind by a cartridge re-layout.
fn sound_api_map_toml() -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"sound_api\"\n\
         lma_base = {base:#x}\n\
         size = {size:#x}\n\
         kind = \"rom\"\n",
        base = pins::SOUND_API.plain_base,
        size = pins::SOUND_API.plain_len,
    )
}

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
    let (main, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse: {pdiags:?}");
    // Prepend the shared engine.z80_bus templates (stop_z80/start_z80 moved
    // there at the t19 step-6 sweep) — sound_api no longer lowers standalone.
    let z80_src = std::fs::read_to_string(aeon_dir().join("engine/z80_bus.emp"))
        .expect("z80_bus.emp must exist beside sound_api.emp");
    let (z80_file, zdiags) = parse_str(&z80_src);
    assert!(zdiags.iter().all(|d| d.level != Level::Error), "z80_bus parse: {zdiags:?}");
    // + engine.irq (sr_masked adopted at the t21 step-6 sweep).
    let irq_src = std::fs::read_to_string(aeon_dir().join("engine/irq.emp"))
        .expect("irq.emp must exist at the engine root");
    let (irq_file, idiags) = parse_str(&irq_src);
    assert!(idiags.iter().all(|d| d.level != Level::Error), "irq parse: {idiags:?}");
    // + engine.sound_constants (sound_api `use`s it for the slot addresses +
    // command values; the authority folds them in this standalone lower).
    let snd_src = std::fs::read_to_string(aeon_dir().join("engine/sound/sound_constants.emp"))
        .expect("sound_constants.emp must exist beside sound_api.emp");
    let (snd_file, sdiags) = parse_str(&snd_src);
    assert!(sdiags.iter().all(|d| d.level != Level::Error), "sound_constants parse: {sdiags:?}");
    // + engine.types (sound_api `use`s SongId/SfxId — the `extern SFXID_RING_*:
    // SfxId` typed references resolve their newtype here; a pure-types module,
    // zero bytes, so prepending it is region-neutral).
    let types_src = std::fs::read_to_string(aeon_dir().join("engine/system/types.emp"))
        .expect("types.emp must exist under engine/system");
    let (types_file, tdiags) = parse_str(&types_src);
    assert!(tdiags.iter().all(|d| d.level != Level::Error), "types parse: {tdiags:?}");
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items: types_file
            .items
            .into_iter()
            .chain(snd_file.items)
            .chain(z80_file.items)
            .chain(irq_file.items)
            .chain(main.items)
            .collect(),
        docs: main.docs.clone(),
    };
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            // DEBUG must be DEFINED (house convention) — 0 elides sound_api's
            // song-id/ring-full asserts (retro-fix batch 2), so #SONG_COUNT is
            // never referenced and needs no synthetic symbol on this probe path.
            // Z80_RAM (engine.constants) is the base of SND_Z80_BASE.
            defines: vec![("DEBUG".to_string(), 0), ("Z80_RAM".to_string(), 0xA0_0000)],
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

/// (d) single-authority genuineness — the two collect-ring SFX ids are TYPED
/// EXTERNs (`extern SFXID_RING_*: SfxId`), so engine.sound_api holds NO local copy
/// of the value: the id crosses the seam once, from the game authority's harvested
/// EquSym. Value desync is therefore impossible BY CONSTRUCTION — there is nothing
/// to doctor. The property that replaces the retired drift guard: with no mirror to
/// silently satisfy the reference, a MISSING authority symbol is LOUD (an
/// unresolved link), never a stale fallback. The AS-side truth here OMITS
/// SFXID_RING_RIGHT/LEFT, so the link must fail naming them.
#[test]
fn typed_extern_has_no_mirror_so_a_missing_authority_is_loud() {
    let Some(src) = sound_api_src() else { return };
    let (module, _asserts, diags) = lower_sound_api(&src);
    assert!(diags.iter().all(|d| d.level != Level::Error), "lower: {diags:?}");

    // Full AS-side truth EXCEPT the two collect-ring SFX ids — the game authority
    // the typed externs bind against. With no local mirror, resolution must fail
    // on them (a genuine desync, surfaced loud).
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
    let map_toml = sound_api_map_toml();
    let map = sigil_link::load_map(&map_toml).expect("map must load");
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

    // Resolution must FAIL on the absent authority symbols — no local mirror
    // silently stands in. Whether the miss surfaces at layout resolve or link, it
    // must name SFXID_RING_RIGHT/LEFT.
    let failure = match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
        Err(diags) => diags,
        Ok(resolved) => match sigil_link::link(&resolved, &SymbolTable::new()) {
            Err(diags) => diags,
            Ok(_) => Vec::new(),
        },
    };
    assert!(
        failure
            .iter()
            .any(|d| d.message.contains("SFXID_RING_RIGHT") || d.message.contains("SFXID_RING_LEFT")),
        "a missing authority must be loud (unresolved), got: {failure:?}"
    );
}

/// (e) a misspelled sound-constant in a slot equ is LOUD — never a silent wrong
/// address. The slot addresses read the authority (engine.sound_constants) by
/// bare name now, so a misspelling is an unknown-name LOWER error (caught by
/// `resolves()` via the lower diagnostics), not a dangling link. Non-vacuity: the
/// SAME composition first resolves the UNDOCTORED source cleanly, so the failure
/// is provably the one misspelled name.
#[test]
fn misspelled_extern_slot_is_loud() {
    let Some(src) = sound_api_src() else { return };

    fn resolves(src: &str) -> bool {
        let (module, _asserts, diags) = lower_sound_api(src);
        if diags.iter().any(|d| d.level == Level::Error) {
            return false;
        }
        let mut sections = module.sections;
        let map_toml = sound_api_map_toml();
        let map = sigil_link::load_map(&map_toml).expect("map must load");
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
    let doctored = src.replace("SND_Z80_BASE + SND_REQ_MUSIC", "SND_Z80_BASE + SND_REQ_MUSICC");
    assert_ne!(src, doctored, "the probe must actually doctor the source");
    assert!(
        !resolves(&doctored),
        "the misspelled sound constant must fail loudly while every correct name resolves"
    );
}
