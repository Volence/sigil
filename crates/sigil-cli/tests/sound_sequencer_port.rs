//! t36 — the REAL `sound_sequencer.emp` port: the Z80 music-event-list INTERPRETER
//! (the biggest resident Z80 file; the tranche where the extern trust surface goes
//! to ZERO), proven by the SCALE-1 WINDOWED oracle (the sound_psg_port template
//! applied to a ~46-proc Z80 CODE file with INTERNAL __DEBUG__ divergence).
//!
//! `sound_sequencer.asm` is the FIRST of the resident z80_sound_driver.asm `phase 0`
//! includes (sequencer -> sfx -> fm -> psg -> tables). Its phase-0 window base is
//! $0565 in BOTH shapes (shape-INVARIANT — unlike psg/fm whose bases shift). What is
//! shape-VARIANT is the sequencer's OWN body: the debug shape carries 16
//! `if DEBUG == 1` blocks (the Seq_Trace body + 15 scattered `call Seq_Trace` sites),
//! for +$7E = 126 bytes (plain window $0565..$0CD7 = 1906 B; debug $0565..$0D55 =
//! 2032 B). This +$7E is the very upstream growth that shifted psg's base in t32 —
//! porting the sequencer, we now OWN it. The oracle proves BOTH shapes: the .emp
//! compiled with `DEBUG=0|1`, byte-compared against `sound_sequencer.asm` assembled
//! through sigil's own AS front-end at the same $0565 phase, with `__DEBUG__` defined
//! for the debug shape, the comptime constants as `-D` defines, and the cross-seam
//! symbols (banked table + external call targets) as equ carriers. The `.asm` stays
//! CANONICAL (this scale does NOT wire into build.sh — the seam sub-tranche owns
//! whole-ROM placement).
//!
//! The trampoline (`push hl` / `ex (sp),hl` / `ret` = computed dispatch through the
//! return address) emits `E3 C9` identically in both windows — its operand form
//! ($E3) is exercised + byte-gated here.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_sequencer_port
//! ```

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

#[derive(Clone, Copy)]
enum Shape {
    /// The plain (DEBUG=0) shipped ROM: sequencer window $0565..$0CD7 = 1906 B.
    Plain,
    /// The debug (DEBUG=1) shipped ROM: the 16 internal __DEBUG__ blocks add +$7E,
    /// window $0565..$0D55 = 2032 B. SAME base ($0565) as plain.
    Debug,
}

impl Shape {
    fn is_debug(self) -> bool {
        matches!(self, Shape::Debug)
    }
    /// The window size (end - $0565), from the s4.lst / s4.debug.lst listings.
    fn window_len(self) -> usize {
        match self {
            Shape::Plain => 0x0772, // 1906 B
            Shape::Debug => 0x07F0, // 2032 B
        }
    }
}

/// The comptime constants the sequencer folds at assemble time (SeqChannel field
/// offsets, flag bits, MEV/SEQEV/TAG/Ctl constants, SND_* RAM+reg addresses). Fed to
/// the .emp as `-D` defines and to the AS twin as equs. Shape-INVARIANT. `DEBUG` is
/// appended per shape by `compile_emp` (the .emp `if DEBUG == 1` gate).
fn const_seam() -> Vec<(&'static str, i64)> {
    vec![
        // SeqChannel field offsets.
        ("sc_stream_ptr", 0x00), ("sc_mod_ptr", 0x02), ("sc_dur_count", 0x04),
        ("sc_dur_default", 0x05), ("sc_patch", 0x06), ("sc_volume", 0x08),
        ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_loop_ptr", 0x0C), ("sc_repeat_ptr", 0x0E), ("sc_repeat_count", 0x10),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_pt_cursor", 0x14), ("sc_points", 0x15), ("sc_transpose", 0x1A),
        ("sc_pan", 0x1B), ("sc_opbias", 0x1C), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_last_pan", 0x24), ("sc_fill_master", 0x25),
        ("sc_fill_count", 0x26), ("sc_psgenv", 0x27), ("sc_psgenv_cur", 0x28),
        ("sc_psgenv_out", 0x29), ("sc_env", 0x27), ("sc_env_cur", 0x28),
        ("sc_env_out", 0x29), ("sc_mod_ctrl", 0x2A), ("sc_mod_wait", 0x2B),
        ("sc_mod_speed", 0x2C), ("sc_mod_delta", 0x2D), ("sc_mod_steps", 0x2E),
        ("sc_mod_speed_raw", 0x2F), ("sc_mod_step_raw", 0x30), ("sc_mod_wait_raw", 0x31),
        ("sc_mod_delta_raw", 0x32), ("sc_mod_accum", 0x33), ("sc_base_freq", 0x35),
        ("sc_last_freq", 0x37), ("sc_noise_mode", 0x39), ("sc_detune", 0x3A),
        ("sc_macro_active", 0x3B), ("sx_gain", 0x40), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C),
        // Flag bit numbers.
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_REKEY_B", 5), ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC_B", 7),
        // MEV opcode range.
        ("MEV_VOL", 0xE0), ("MEV_REST", 0x80), ("MEV_NOTE_BASE", 0x81),
        // Vol-env control bytes.
        ("PsgVolEnvCtl_Loop", 0x80), ("PsgVolEnvCtl_Sustain", 0x81), ("PsgVolEnvCtl_Rest", 0x83),
        ("FmVolEnvCtl_Loop", 0x80), ("FmVolEnvCtl_Sustain", 0x81), ("FmVolEnvCtl_Rest", 0x83),
        // Macro-stream tags.
        ("TAG_MAC_NEXT", 0xE0), ("TAG_MAC_REG", 0xE1), ("TAG_MAC_LOOP", 0xE2), ("TAG_MAC_END", 0xE3),
        // FM fnum band edges (for the block-boundary correction).
        ("FNUM_LO", 0x284), ("FNUM_HI", 0x508),
        // DEBUG trace event codes.
        ("SEQEV_NOTEON", 1), ("SEQEV_NOTEOFF", 2), ("SEQEV_VOL", 3), ("SEQEV_PATCH", 4),
        ("SEQEV_DAC", 5), ("SEQEV_LOOP", 6), ("SEQEV_JUMP", 7), ("SEQEV_END", 8),
        ("SEQEV_RPT_START", 9), ("SEQEV_RPT_END", 10),
        // PSG/YM register + value constants.
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_SILENCE_T3", 0xDF), ("CHROUTE_PSGN", 9),
        ("SND_REG_LFO", 0x22), ("SND_REG_TIMER_A_HI", 0x24), ("SND_REG_TIMER_CTRL", 0x27),
        ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_DAC_ENABLE", 0x2B),
        ("SND_FADE_DELAY", 1), ("SND_FADE_STEP", 2), ("SND_SEQ_TRACE_LEN", 0x20),
        // Z80-bus + RAM absolute addresses.
        ("SND_Z80_PSG", 0x7F11), ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_STAT_TICK", 0x1F13), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHCOUNT", 0x1A01),
        ("SND_SEQ_CHANNELS", 0x1A08), ("SND_SEQ_BADOP", 0x1A05), ("SND_SEQ_TRACE", 0x1CAC),
        ("SND_SEQ_TRACE_WR", 0x1A06), ("SND_TEMPO_CUR", 0x1CD0), ("SND_TEMPO_TARGET", 0x1CD1),
        ("SND_TEMPO_BASE", 0x1CD2), ("SND_MASTER_FADE", 0x1CCC), ("SND_FADE_TARGET", 0x1CCD),
        ("SND_FADE_DELAY_CTR", 0x1CCE), ("SND_FADE_DIRTY", 0x1CCF),
        ("Snd_SongBase", 0x1CA1), ("Snd_SpindashRev", 0x1CA5),
    ]
}

/// The cross-seam LINK symbols the sequencer references as addresses: the banked
/// $8000-window `SeqOpcodeTable` (`ld hl, SeqOpcodeTable` -> Value16Le) and the
/// external resident call/jp targets. Fed to the .emp as equ carriers and to the AS
/// twin as equs. The banked table + the two BEFORE-sequencer procs (Snd_StartSample,
/// Snd_DacLookup) are shape-INVARIANT; every AFTER-sequencer proc shifts +$7E in the
/// debug shape (the sequencer's own __DEBUG__ growth pushes them later). `doctor`
/// overrides ONE symbol — the t24 byte-gate positive control.
fn link_seam(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    // Plain-shape addresses (from s4.lst); AFTER-sequencer procs get +$7E in debug.
    let shift = if shape.is_debug() { 0x7E } else { 0 };
    let after = |v: i64| v + shift; // procs emitted after the sequencer window
    let mut out: Vec<(String, i64)> = vec![
        ("Sfx_Frame", after(0x0D2A)),
        ("Fm_FnumApplyDelta", after(0x154E)), ("Fm_WriteFreq", after(0x162C)),
        ("Fm_NoteFromTable", after(0x1533)), ("Fm_NoteOn", after(0x1590)),
        ("Fm_NoteOnFreq", after(0x159D)), ("Fm_NoteOff", after(0x161A)),
        ("Fm_SetVolume", after(0x13DE)), ("Fm_SetPan", after(0x14BE)),
        ("Fm_PatchPtr", after(0x12F5)), ("Fm_PatchLoad", after(0x1312)),
        ("Fm_RegDelta", after(0x14CD)), ("Fm_YmWrite", after(0x12C3)),
        ("Fm_ReparkDac", after(0x12D9)),
        ("Psg_SetVolume", after(0x176D)), ("Psg_NoteOn", after(0x16B5)),
        ("Psg_NoteOff", after(0x170D)), ("Psg_Noise", after(0x17D2)),
        ("Psg_ApplyMod", after(0x1729)), ("Psg_EmitDivisor", after(0x1734)),
        ("Psg_SilenceAll", after(0x1807)),
        ("PsgVolEnv_Resolve", after(0x1683)), ("FmVolEnv_Resolve", after(0x169C)),
        ("Snd_ChanClass", after(0x12EE)),
        // Shape-INVARIANT: BEFORE the sequencer, and the banked engine-table.
        ("Snd_StartSample", 0x02B7), ("Snd_DacLookup", 0x029E),
        ("SeqOpcodeTable", 0x856D),
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

/// Compile sound_sequencer.emp at the $0565 window with the shape's `DEBUG` define.
/// Constants -> `-D` defines; link symbols -> equ carriers. Returns the
/// `sound_sequencer` section bytes.
fn compile_emp(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("sound_sequencer.emp"))
        .unwrap_or_else(|e| panic!("read sound_sequencer.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_sequencer.emp parse errors: {pdiags:?}"
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
        "sound_sequencer.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"sound_sequencer\"\nlma_base = 0x0565\nsize = 0x800\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    for sec in sections.iter_mut() {
        if sec.name == "sound_sequencer" {
            sec.vma_base = Some(0x0565);
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
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of the sequencer's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("sound_sequencer").expect("linked sound_sequencer").bytes.clone()
}

/// The blob's LMA base in the reference ROM, per shape (plain vs debug).
fn blob_lma(debug: bool) -> usize {
    if debug {
        0x3E2
    } else {
        0x3DE
    }
}

/// The reference-ROM slice this file's window is byte-gated against. The sequencer's
/// bytes live in the shipped ROM at `blob_lma(shape) + $0565` (the blob's LMA plus the
/// sequencer's shape-INVARIANT phase-0 window base). Reads `s4.bin` (plain) /
/// `s4.debug.bin` (debug) from `AEON_DIR`; returns the ROM tail from that offset (the
/// caller compares the first `compile_emp.len()` bytes). Under `strict_gate` a missing
/// ROM panics; otherwise the test skips (`None`).
fn reference_slice(shape: Shape) -> Option<Vec<u8>> {
    let name = if shape.is_debug() { "s4.debug.bin" } else { "s4.bin" };
    let path = aeon_dir().join(name);
    let refrom = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference ROM missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            return None;
        }
    };
    let off = blob_lma(shape.is_debug()) + 0x0565;
    Some(refrom[off..].to_vec())
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
    if !aeon.join("s4.bin").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference ROM missing at {}", aeon.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE windowed byte gate (PLAIN shape): sound_sequencer.emp == the reference ROM
/// slice at the $0565 window, DEBUG=0. 1906 bytes ($0565..$0CD7). Proves the whole
/// transcription incl. the `ex (sp),hl` trampoline (`E3 C9`), with the 16
/// `if DEBUG == 1` blocks folded away.
#[test]
fn sound_sequencer_matches_reference_plain() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, None);
    assert_eq!(emp.len(), Shape::Plain.window_len(), "sequencer plain window is 1906 B");
    assert_bytes_match(
        &emp,
        &ref_tail[..emp.len()],
        "sound_sequencer.emp vs s4.bin slice (plain $0565, DEBUG=0)",
    );
}

/// THE windowed byte gate (DEBUG shape): sound_sequencer.emp == the reference ROM
/// slice at the $0565 window, DEBUG=1. 2032 bytes ($0565..$0D55) — +$7E over plain from
/// the 16 internal __DEBUG__ blocks (Seq_Trace body + 15 call sites). Same base as
/// plain; the external call targets shift +$7E. Proves the .emp `if DEBUG == 1` bodies
/// match the reference EXACTLY.
#[test]
fn sound_sequencer_matches_reference_debug() {
    let Some(ref_tail) = reference_slice(Shape::Debug) else { return };
    let emp = compile_emp(Shape::Debug, None);
    assert_eq!(emp.len(), Shape::Debug.window_len(), "sequencer debug window is 2032 B");
    assert_bytes_match(
        &emp,
        &ref_tail[..emp.len()],
        "sound_sequencer.emp vs s4.debug.bin slice (debug $0565, DEBUG=1)",
    );
}

/// The +$7E delta is EXACTLY the 16 internal __DEBUG__ blocks: debug window - plain
/// window == 126 bytes. Guards the shape-variance claim (the source of psg's t32 base
/// shift).
#[test]
fn debug_window_is_plain_plus_7e() {
    assert_eq!(
        Shape::Debug.window_len() - Shape::Plain.window_len(),
        0x7E,
        "the 16 internal __DEBUG__ blocks add exactly +$7E"
    );
}

/// The two shapes MUST differ (shape-variance evidence): the debug shape carries the
/// trace blocks AND its external call targets shift +$7E, so the plain and debug byte
/// images are NOT equal — the reason both must be gated.
#[test]
fn plain_and_debug_shapes_differ() {
    if skip_if_missing() {
        return;
    }
    let plain = compile_emp(Shape::Plain, None);
    let debug = compile_emp(Shape::Debug, None);
    assert_ne!(plain, debug, "the sequencer is shape-variant — the two windows differ");
}

/// Positive control (byte-gate non-triviality, t24): a DOCTORED .emp (ONE moved
/// cross-seam address — a shifted SeqOpcodeTable changes the `.coord` trampoline's `ld
/// hl, SeqOpcodeTable`) must DIVERGE from the reference slice. Proves the comparison
/// detects a difference, not a vacuous match.
#[test]
fn emp_diverges_from_doctored_reference() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, Some(("SeqOpcodeTable", 0x8ABC)));
    assert_ne!(
        emp.as_slice(),
        &ref_tail[..emp.len()],
        "the byte gate is vacuous if a doctored .emp still matches the reference"
    );
}
