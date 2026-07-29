//! t37 — the REAL `sound_sfx.emp` port: the Phase-5a SFX engine (steal + per-frame
//! interpreter — the FOURTH resident Z80 CODE file, the Out-heavy struct-prefix
//! mirror that REUSES the sequencer's ModUpdate/Sequencer_Channel), proven by the
//! SCALE-1 WINDOWED oracle (the sound_psg_port / sound_sequencer_port template).
//!
//! `sound_sfx.asm` is the 2nd of 5 includes inside z80_sound_driver.asm's
//! `cpu z80 / phase 0` blob (sequencer -> **sfx** -> fm -> psg -> tables). Its
//! phase-0 window is the PSG t32 CASE: SHAPE-VARIANT base $0CD7 (plain) vs $0D55
//! (debug) — a +$7E shift from the SEQUENCER's 16 internal `if DEBUG == 1` blocks
//! that precede this window — but sfx has NO internal debug blocks of its own, so
//! its window LENGTH is 1516 B ($5EC) in BOTH shapes. The shapes differ only in the
//! base embedded in internal jp/call targets + the +$7E-shifted fm/psg external call
//! targets (the two sequencer procs ModUpdate/Sequencer_Channel + the two driver
//! procs SndDrv_SetBank/Snd_RouteClassFlags PRECEDE the sequencer's debug growth, so
//! they are shape-INVARIANT). The oracle proves BOTH shapes: the .emp compiled at
//! the shape's vma, byte-compared against `sound_sfx.asm` assembled through sigil's
//! own AS front-end at the same phase, with the comptime constants as `-D` defines
//! and the cross-seam symbols (banked SfxBlobWinTab + external call targets) as equ
//! carriers. The `.asm` stays CANONICAL (this scale does NOT wire into build.sh —
//! the seam sub-tranche owns whole-ROM placement).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_sfx_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The sfx window length (fm_start - sfx_start), shape-INVARIANT (sfx has no own
/// __DEBUG__ blocks): $12C3-$0CD7 = $1341-$0D55 = $5EC = 1516 B.
const SFX_WINDOW_LEN: usize = 0x5EC;

#[derive(Clone, Copy)]
enum Shape {
    /// The plain (DEBUG=0) shipped ROM: sfx window base $0CD7.
    Plain,
    /// The debug (DEBUG=1) shipped ROM: the sequencer's +$7E __DEBUG__ growth shifts
    /// sfx to $0D55 (sfx's OWN layout is unchanged — no internal debug blocks).
    Debug,
}

impl Shape {
    fn is_debug(self) -> bool {
        matches!(self, Shape::Debug)
    }
    /// The phase-0 window base of sfx in this shape (from the asl listings:
    /// s4.lst `Sfx_QueueEntryPtr: 0CD7`, s4.debug.lst `Sfx_QueueEntryPtr: 0D55`).
    fn vma(self) -> i64 {
        match self {
            Shape::Plain => 0x0CD7,
            Shape::Debug => 0x0D55,
        }
    }
}

/// The comptime constants sfx folds at assemble time (sound_constants.asm equs +
/// game config). Fed to the .emp as `-D` defines and to the AS twin as equs.
/// Shape-INVARIANT. The in-file SND_SFX_DISP_* / SFX_SLOT_NONE derive from these on
/// BOTH sides (the .emp `const` block + the .asm body), so only the roots are here.
/// `DEBUG` is appended per shape by `compile_emp` (harmless — sfx has no `if DEBUG`).
fn const_seam() -> Vec<(&'static str, i64)> {
    vec![
        // SeqChannel / SfxChannel field offsets (shared prefix + sx_* appended).
        ("sc_stream_ptr", 0x00), ("sc_dur_count", 0x04), ("sc_dur_default", 0x05),
        ("sc_volume", 0x08), ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_last_pan", 0x24), ("sc_base_freq", 0x35), ("sc_noise_mode", 0x39),
        ("sx_priority", 0x39), ("sx_patch_base", 0x3B), ("sx_saved_route", 0x3D),
        ("sx_kind", 0x3F), ("sx_gain", 0x40), ("sx_duck", 0x41), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C), ("SfxChannel_len", 0x44),
        // SCF_* flag bit numbers + the chromatic mask.
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC", 0x80),
        // SFXEL_* voice kinds.
        ("SFXEL_NONE", 0), ("SFXEL_FM", 1), ("SFXEL_PSG", 2), ("SFXEL_NOISE", 3),
        // SfxHeader field offsets + per-channel record offsets.
        ("SFXH_PRIORITY", 0), ("SFXH_FLAGS", 1), ("SFXH_CHCOUNT", 2), ("SFXH_GAIN", 3),
        ("SFXH_DUCK", 4), ("SFXH_CAP", 5), ("SFXH_CHANNELS", 8),
        ("SFXHC_ROUTE", 0), ("SFXHC_CMD_HI", 2), ("SFXHC_CMD_LO", 3),
        ("SFXHC_VOICE_HI", 4), ("SFXHC_VOICE_LO", 5), ("SFXHC_LEN", 6),
        // SHF_* header flags.
        ("SHF_CONTINUOUS_B", 0), ("SHF_CONTINUOUS", 1),
        // CHROUTE_* routes + count.
        ("CHROUTE_FM3", 2), ("CHROUTE_FM4", 3), ("CHROUTE_FM5", 4), ("CHROUTE_PSG1", 6),
        ("CHROUTE_PSG2", 7), ("CHROUTE_PSG3", 8), ("CHROUTE_PSGN", 9), ("CHROUTE_COUNT", 0x0B),
        // SFX_* tuning + game ids.
        ("SFX_VOICE_COUNT", 7), ("SFX_DUCK_RAMP_STEP", 4), ("SFX_EXTEND_FRAMES", 0x0A),
        ("SFX_QUEUE_DEPTH", 3), ("SFX_ID_BASE", 0x33), ("SFX_TABLE_LEN", 0x87),
        ("SFX_BLOB_BANK", 0x0B), ("SFXID_REV_LOOP", 0xAB),
        // SND_SFX_* RAM (SND_SFX_DUCK_TARGET roots the in-file SND_SFX_DISP_* block).
        ("SND_SFX_CHANNELS", 0x1D00), ("SND_SFX_QUEUE", 0x1EDC), ("SND_SFX_QUEUE_CNT", 0x1EE4),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SFX_DUCK_TARGET", 0x1EE6), ("SND_REQ_BASE", 0x1F00),
        // SND_SEQ_* RAM.
        ("SND_SEQ_CHCOUNT", 0x1A01), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHANNELS", 0x1A08),
        // Misc RAM / PSG port + value.
        ("Snd_SpindashRev", 0x1CA5), ("SND_Z80_PSG", 0x7F11), ("SND_PSG_SILENCE_T3", 0xDF),
    ]
}

/// The cross-seam LINK symbols sfx references as addresses: the banked $8000-window
/// SfxBlobWinTab (`ld de, SfxBlobWinTab` -> Value16Le), the sequencer/driver call
/// targets (shape-INVARIANT — before the sequencer's __DEBUG__ growth), and the
/// fm/psg writer call targets (shape-VARIANT — after the growth, +$7E in debug). Fed
/// to the .emp as equ carriers and to the AS twin as equs. `doctor` overrides ONE
/// symbol — the t24 byte-gate positive control.
fn link_seam(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let shift = if shape.is_debug() { 0x7E } else { 0 };
    let after = |v: i64| v + shift; // fm/psg procs emitted after the sequencer window
    let mut out: Vec<(String, i64)> = vec![
        // SHAPE-INVARIANT (precede the sequencer's debug blocks / RAM / banked table):
        ("ModUpdate", 0x0713), ("Sequencer_Channel", 0x095B),
        ("SndDrv_SetBank", 0x031D), ("Snd_RouteClassFlags", 0x0554),
        ("SfxBlobWinTab", 0x845F),
        // SHAPE-VARIANT (+$7E in debug — fm/psg writers after the sequencer growth):
        ("Fm_PatchLoad", after(0x1312)), ("Fm_NoteOff", after(0x161A)),
        ("Fm_SetVolume", after(0x13DE)), ("Fm_PatchPtr", after(0x12F5)),
        ("Fm_NoteOnFreqExact", after(0x15AA)),
        ("Psg_NoteOff", after(0x170D)), ("Psg_SetVolume", after(0x176D)),
        ("Psg_NoteOn", after(0x16B5)), ("Psg_Noise", after(0x17D2)),
    ]
    .into_iter()
    .map(|(n, v)| (n.to_string(), v))
    .collect();
    if let Some((dn, dv)) = doctor {
        for (n, v) in out.iter_mut() {
            if n == dn {
                *v = dv;
            }
        }
    }
    out
}

/// Compile sound_sfx.emp at the shape's phase-0 window (the .emp declares
/// `vma:$0CD7`; for the debug shape we re-base the section's `vma_base` to $0D55).
/// Constants -> `-D` defines; link symbols -> equ carriers. Returns the `sound_sfx`
/// section bytes.
fn compile_emp(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("sound_sfx.emp"))
        .unwrap_or_else(|e| panic!("read sound_sfx.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_sfx.emp parse errors: {pdiags:?}"
    );
    let mut defines: Vec<(String, i128)> =
        const_seam().into_iter().map(|(n, v)| (n.to_string(), v as i128)).collect();
    defines.push(("DEBUG".to_string(), if shape.is_debug() { 1 } else { 0 }));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_sfx.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"sound_sfx\"\nlma_base = 0x0CD7\nsize = 0x800\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    for sec in sections.iter_mut() {
        if sec.name == "sound_sfx" {
            sec.vma_base = Some(shape.vma() as u32);
        }
    }
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    let pairs: Vec<(String, String)> =
        link_seam(shape, doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of sfx's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("sound_sfx").expect("linked sound_sfx").bytes.clone()
}

/// The AS twin oracle: assemble sound_sfx.asm through sigil's AS front-end at the
/// shape's `phase`, with ALL cross-seam symbols (constants + link addresses) as equs,
/// and return the emitted Z80 code bytes.
fn as_twin_bytes(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = std::fs::read_to_string(aeon.join("engine/sound/sound_sfx.asm"))
        .unwrap_or_else(|e| panic!("read sound_sfx.asm: {e}"));
    let mut seam = String::from("cpu 68000\n");
    for (name, val) in const_seam() {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    for (name, val) in link_seam(shape, doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    // Leading `0` so an h-suffix hex whose first digit is a letter (e.g. $CD7 ->
    // `0CD7h`) parses as a number, not a symbol (the AS hex-literal quirk).
    seam.push_str(&format!("cpu z80\nphase 0{:X}h\n", shape.vma()));
    let src = format!("{seam}{body}\ndephase\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
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
        .find(|s| !s.bytes.is_empty())
        .map(|s| s.bytes.clone())
        .unwrap_or_default()
}

fn assert_bytes_match(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch ({} vs {})",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(6);
        let hi = (i + 10).min(candidate.len());
        panic!(
            "{what}: first diff at {i:#x}\n  emp[{lo:#x}..{hi:#x}]: {:02x?}\n  twin[{lo:#x}..{hi:#x}]: {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

fn skip_if_missing() -> bool {
    let aeon = aeon_dir();
    if !aeon.join("engine/sound/sound_sfx.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE windowed byte gate (PLAIN shape): sound_sfx.emp == sound_sfx.asm at the
/// $0CD7 window. sfx is 1516 bytes ($5EC, $0CD7..$12C3). Proves the whole
/// transcription incl. the Out-heavy contract set, the game-config `if SFXID_REV_LOOP
/// >= 0` block, and the parenthesized `(ix+(field+k))` displacements.
#[test]
fn sound_sfx_matches_as_twin_plain() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, None);
    let twin = as_twin_bytes(Shape::Plain, None);
    assert!(!twin.is_empty(), "the AS twin must emit the sfx code");
    assert_eq!(twin.len(), SFX_WINDOW_LEN, "sfx is 1516 bytes ($0CD7..$12C3)");
    assert_bytes_match(&emp, &twin, "sound_sfx.emp vs sound_sfx.asm (plain $0CD7)");
}

/// THE windowed byte gate (DEBUG shape): sound_sfx.emp == sound_sfx.asm at the
/// $0D55 window (the sequencer's +$7E __DEBUG__ growth shifts the base). SAME length
/// as plain (sfx has no own debug blocks); the internal jp/call targets + the fm/psg
/// external targets differ — proving the .emp tracks the shape's window.
#[test]
fn sound_sfx_matches_as_twin_debug() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Debug, None);
    let twin = as_twin_bytes(Shape::Debug, None);
    assert!(!twin.is_empty(), "the AS twin must emit the sfx code");
    assert_eq!(twin.len(), SFX_WINDOW_LEN, "sfx is 1516 bytes (shape-invariant length)");
    assert_bytes_match(&emp, &twin, "sound_sfx.emp vs sound_sfx.asm (debug $0D55)");
}

/// The two shapes MUST differ (shape-variance evidence): the base embedded in the
/// internal jp/call targets + the +$7E fm/psg external targets make the plain and
/// debug byte images NOT equal — the reason both must be gated.
#[test]
fn plain_and_debug_shapes_differ() {
    if skip_if_missing() {
        return;
    }
    let plain = compile_emp(Shape::Plain, None);
    let debug = compile_emp(Shape::Debug, None);
    assert_ne!(plain, debug, "sfx is shape-variant — the two windows must produce different bytes");
}

/// Positive control (byte-gate non-triviality, t24): the UNDOCTORED .emp must DIVERGE
/// from a twin assembled with ONE doctored cross-seam address (a moved SfxBlobWinTab
/// changes every `ld de, SfxBlobWinTab` — SfxDispatch + Sfx_BeginSound). Proves the
/// comparison detects a difference, not a vacuous match. Pins-derived: the reference
/// window is the twin RE-ASSEMBLED each run, so a re-pin cannot re-stale it.
#[test]
fn emp_diverges_from_doctored_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, None);
    let twin = as_twin_bytes(Shape::Plain, Some(("SfxBlobWinTab", 0x8ABC)));
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// Doctoring the SAME symbol on BOTH sides keeps them equal — the abs16 call fixups
/// resolve to the shared address identically (the cell tracks the linked symbol, not
/// a frozen literal).
#[test]
fn doctored_both_sides_stay_equal() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, Some(("ModUpdate", 0x1234)));
    let twin = as_twin_bytes(Shape::Plain, Some(("ModUpdate", 0x1234)));
    assert_bytes_match(&emp, &twin, "sound_sfx.emp vs twin (both ModUpdate=$1234)");
}
