//! t40 — the REAL `z80_sound_driver.emp` port: the Z80-autonomous DAC streaming
//! driver, the TOP of the sound stack and the LAST resident Z80 CODE file. Proven
//! by the SCALE-1 WINDOWED oracle (the sound_sequencer_port template applied to the
//! blob's FRONT), BOTH shapes.
//!
//! `z80_sound_driver.asm` DEFINES the phase-0 blob origin ($0000): it `save/cpu z80/
//! phase 0`s, then at its tail `include`s the sequencer/sfx/fm/psg engine + tables.
//! Its OWN window is blob offset $0000..$0565 = 1381 B, the SAME SIZE in BOTH shapes
//! (zero internal __DEBUG__ content). But it is NOT byte-identical across shapes:
//! five `call` sites target callees that live AFTER the sequencer's +$7E debug
//! growth (Sequencer_StopAll / Sfx_StopAll / SfxDispatch all shift +$7E in debug), so
//! those call operands differ plain vs debug. Sequencer_Frame ($0565, the sequencer
//! base) is the one shape-INVARIANT external target. The oracle therefore feeds
//! per-shape link addresses and gates BOTH shapes (the two windows are NOT equal).
//!
//! The twin assembles the driver BODY ONLY: `z80_sound_driver.asm` sliced from after
//! its `phase 0` framing to before the first engine `include` (so the even-pad +
//! budget `fatal` + the included engine are excluded), at the $0000 phase, with all
//! cross-file constants + the 4 external call targets as equ carriers. The `.asm`
//! stays CANONICAL (this scale does NOT wire into build.sh — the seam sub-tranche
//! owns whole-ROM placement).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test z80_sound_driver_port
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

#[derive(Clone, Copy)]
enum Shape {
    /// The plain (DEBUG=0) shipped ROM.
    Plain,
    /// The debug (DEBUG=1) shipped ROM: the sequencer's 16 internal __DEBUG__ blocks
    /// add +$7E DOWNSTREAM of the driver, shifting three of the driver's call targets.
    Debug,
}

impl Shape {
    fn is_debug(self) -> bool {
        matches!(self, Shape::Debug)
    }
    /// The driver window is $0000..$0565 = 1381 B in BOTH shapes (SAME SIZE; zero
    /// internal __DEBUG__ content). What differs is the BYTES at the shape-variant
    /// call sites, not the length.
    fn window_len(self) -> usize {
        0x0565 // 1381 B, both shapes
    }
}

/// The comptime constants the driver folds at assemble time: the SND_* RAM/reg map,
/// the SeqChannel/song-header field offsets, the CHROUTE/SCF enums, the Timer-A
/// constants, and the banked DacSampleTable window ptr. Fed to the .emp as `-D`
/// defines and to the AS twin as equs. Shape-INVARIANT. Internal driver labels are
/// EXCLUDED (they resolve within the body). Extracted from s4.lst's symbol table.
fn const_seam() -> Vec<(&'static str, i64)> {
    vec![
        ("CHROUTE_COUNT", 0xb), ("CHROUTE_DAC", 0xa), ("CHROUTE_FM6", 0x5),
        ("CHROUTE_PSG1", 0x6), ("DAC_SAMPLE_COUNT", 0xa), ("DacSampleTable", 0x85ad),
        ("DacSample_ds_length", 0x5), ("DacSample_ds_ptr", 0x3), ("SCF_ACTIVE", 0x1),
        ("SCF_IS_DAC", 0x10), ("SCF_IS_FM", 0x4), ("SCF_IS_PSG", 0x8), ("SCF_KEYED_B", 0x1),
        ("SHC_CMD_HI", 0x1), ("SHC_CMD_LO", 0x2), ("SHC_LEN", 0x5), ("SHC_MOD_HI", 0x3),
        ("SHC_MOD_LO", 0x4), ("SHC_ROUTE", 0x0), ("SH_CHANNELS", 0x6), ("SH_CHCOUNT", 0x3),
        ("SH_F_FM6_ADAPTIVE", 0x4), ("SH_PITCHTAB_HI", 0x4), ("SH_PITCHTAB_LO", 0x5),
        ("SH_TEMPO", 0x1), ("SH_TEMPO_MOD", 0x2), ("SND_ALIVE_MARKER", 0x5a),
        ("SND_CTRL_DMA_ACTIVE", 0x1f04), ("SND_CUR_BANK", 0x18f3), ("SND_DAC_PHASE", 0x18f0),
        ("SND_ENGINE_TABLE_BANK", 0xb), ("SND_FADE_CMD_IN", 0x2), ("SND_FADE_DELAY", 0x1),
        ("SND_FADE_DELAY_CTR", 0x1cce), ("SND_FADE_DIRTY", 0x1ccf), ("SND_FADE_SILENCE", 0x7f),
        ("SND_FADE_TARGET", 0x1ccd), ("SND_FM6_ADAPTIVE", 0x18fc),
        ("SND_FM6_CHAN_PTR", 0x18fa), ("SND_FM_KEYON_OPMASK", 0xf0),
        ("SND_MASTER_FADE", 0x1ccc), ("SND_MUSIC_PARAM_BANK", 0x1ca6),
        ("SND_MUSIC_PARAM_FLAGS", 0x1ca9), ("SND_MUSIC_PARAM_PATCHPTR", 0x1caa),
        ("SND_MUSIC_PARAM_PTR", 0x1ca7), ("SND_MUSIC_STOP", 0xff), ("SND_REG_DAC_DATA", 0x2a),
        ("SND_REG_DAC_ENABLE", 0x2b), ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_LFO", 0x22),
        ("SND_REG_LR_AMS_FMS", 0xb4), ("SND_REG_TIMER_A_HI", 0x24),
        ("SND_REG_TIMER_A_LO", 0x25), ("SND_REG_TIMER_CTRL", 0x27), ("SND_REQ_FADE", 0x1f05),
        ("SND_REQ_MUSIC", 0x1f02), ("SND_REQ_PING", 0x1f00), ("SND_REQ_SAMPLE", 0x1f01),
        ("SND_REQ_SFX", 0x1f03), ("SND_REQ_TEMPO", 0x1f06), ("SND_RING_BASE", 0x1900),
        ("SND_RING_LEAD_PRIME", 0x80), ("SND_RING_LEAD_TARGET", 0xc8), ("SND_RING_PAGE", 0x19),
        ("SND_RING_RD", 0x18f4), ("SND_RING_WR", 0x18f5), ("SND_ROM_BANK", 0x18f2),
        ("SND_ROM_LEN", 0x18f8), ("SND_ROM_PTR", 0x18f6), ("SND_SEQ_ACTIVE", 0x1a04),
        ("SND_SEQ_BADOP", 0x1a05), ("SND_SEQ_BASE", 0x1a00), ("SND_SEQ_CHANNELS", 0x1a08),
        ("SND_SEQ_CHCOUNT", 0x1a01), ("SND_SEQ_END", 0x1c9c), ("SND_SEQ_PATCHTAB", 0x1a02),
        ("SND_SEQ_TEMPO", 0x1a00), ("SND_SEQ_TEMPO_MOD", 0x1a07), ("SND_SEQ_TRACE_WR", 0x1a06),
        ("SND_SFX_QUEUE_CNT", 0x1ee4), ("SND_SONG_BANK", 0x18f1), ("SND_STACK_TOP", 0x1ffe),
        ("SND_STAT_ACK_COUNT", 0x1f12), ("SND_STAT_ALIVE", 0x1f10),
        ("SND_STAT_DAC_ACTIVE", 0x1f14), ("SND_STAT_PING_ECHO", 0x1f11),
        ("SND_STAT_TICK", 0x1f13), ("SND_TEMPO_BASE", 0x1cd2), ("SND_TEMPO_CUR", 0x1cd0),
        ("SND_TEMPO_RESTORE", 0xff), ("SND_TEMPO_TARGET", 0x1cd1),
        ("SND_TIMERA_CTRL_PROGRAM", 0x5), ("SND_TIMERA_CTRL_REARM", 0x15),
        ("SND_TIMERA_N", 0x89), ("SND_TIMERA_OVF_MASK", 0x1), ("SND_Z80_BANKREG", 0x6000),
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001), ("SND_Z80_YM_A2", 0x4002),
        ("SND_Z80_YM_A3", 0x4003), ("SeqChannel_len", 0x3c), ("Snd_PitchTabPtr", 0x1ca3),
        ("Snd_SongBase", 0x1ca1), ("Snd_SpindashRev", 0x1ca5), ("sc_detune", 0x3a),
        ("sc_dur_count", 0x4), ("sc_dur_default", 0x5), ("sc_flags", 0xa),
        ("sc_last_patch", 0x7), ("sc_macro_active", 0x3b), ("sc_mod_ctrl", 0x2a),
        ("sc_mod_ptr", 0x2), ("sc_noise_mode", 0x39), ("sc_note", 0x9),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_psgenv", 0x27),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_pt_count", 0x13),
        ("sc_route", 0xb), ("sc_stream_ptr", 0x0), ("sc_tempo_accum", 0x12),
        ("sc_tempo_mod", 0x11), ("sc_volume", 0x8),
    ]
}

/// The cross-file resident CALL targets the driver references as addresses (the 4
/// windowed-oracle-boundary externs). Fed to the .emp as equ carriers and to the AS
/// twin as equs. Sequencer_Frame ($0565, the sequencer base) is shape-INVARIANT; the
/// other three live AFTER the sequencer's +$7E debug growth, so they shift +$7E in
/// the debug shape — which is exactly WHY the driver window is not shape-identical.
/// `doctor` overrides ONE symbol (const or link) — the t24 byte-gate positive control.
fn link_seam(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let shift = if shape.is_debug() { 0x7E } else { 0 };
    let after = |v: i64| v + shift; // targets emitted after the sequencer's debug growth
    let mut out: Vec<(String, i64)> = vec![
        ("Sequencer_Frame", 0x0565), // shape-INVARIANT (the sequencer base)
        ("Sequencer_StopAll", after(0x0CB2)),
        ("Sfx_StopAll", after(0x11AA)),
        ("SfxDispatch", after(0x0E5D)),
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

/// const_seam with an optional single-symbol doctor override applied.
fn const_seam_doctored(doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    const_seam()
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v,
            };
            (n.to_string(), v)
        })
        .collect()
}

/// Slice the driver BODY out of z80_sound_driver.asm: from after the `phase 0`
/// framing directive to before the first engine `include`, so the even-pad + budget
/// `fatal` + the included engine are excluded and only $0000..$0565 is emitted.
fn driver_body(aeon: &std::path::Path) -> String {
    let src = std::fs::read_to_string(aeon.join("engine/sound/z80_sound_driver.asm"))
        .unwrap_or_else(|e| panic!("read z80_sound_driver.asm: {e}"));
    let lines: Vec<&str> = src.lines().collect();
    let phase_idx = lines
        .iter()
        .position(|l| l.trim() == "phase 0")
        .expect("driver `phase 0` framing directive");
    let incl_idx = lines
        .iter()
        .position(|l| l.contains("include \"engine/sound/"))
        .expect("driver first engine include");
    assert!(phase_idx < incl_idx, "phase framing precedes the includes");
    lines[phase_idx + 1..incl_idx].join("\n")
}

/// Compile z80_sound_driver.emp at the $0000 window with the shape's link addresses.
/// Constants -> `-D` defines; the 4 external call targets -> equ carriers. Returns the
/// `z80_sound_driver` section bytes (1381 B).
fn compile_emp(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("z80_sound_driver.emp"))
        .unwrap_or_else(|e| panic!("read z80_sound_driver.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "z80_sound_driver.emp parse errors: {pdiags:?}"
    );
    let defines: Vec<(String, i128)> =
        const_seam_doctored(doctor).into_iter().map(|(n, v)| (n, v as i128)).collect();
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "z80_sound_driver.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"z80_sound_driver\"\nlma_base = 0x0000\nsize = 0x600\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    for sec in sections.iter_mut() {
        if sec.name == "z80_sound_driver" {
            sec.vma_base = Some(0x0000);
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
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of the driver's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("z80_sound_driver").expect("linked z80_sound_driver").bytes.clone()
}

/// The AS twin oracle: assemble the driver body through sigil's AS front-end at the
/// $0000 phase, with ALL cross-file symbols (constants + the 4 call targets) as equs,
/// and return the emitted Z80 code bytes. The debug shape shifts the three
/// after-sequencer call targets +$7E (the driver has no `ifdef __DEBUG__` of its own).
fn as_twin_bytes(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = driver_body(&aeon);
    let mut seam = String::from("cpu 68000\n");
    for (name, val) in const_seam_doctored(doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    for (name, val) in link_seam(shape, doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    seam.push_str("cpu z80\nphase 0h\n");
    let src = format!("{seam}{body}\ndephase\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines: Vec::new(), ..AsOptions::default() };
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
    if !aeon.join("engine/sound/z80_sound_driver.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE windowed byte gate (PLAIN shape): z80_sound_driver.emp == the driver body of
/// z80_sound_driver.asm at the $0000 window, DEBUG=0. 1381 bytes ($0000..$0565).
/// Proves the whole transcription incl. the reset-vector zero-fill, the shadow-set
/// `exx` pairs, the `im 1` / `ld (nn),ix` / `ld ix,(nn)` forms, and the three
/// cycle-balance `ensure`s (self-gated to zero bytes).
#[test]
fn z80_sound_driver_matches_as_twin_plain() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, None);
    let twin = as_twin_bytes(Shape::Plain, None);
    assert!(!twin.is_empty(), "the AS twin must emit the driver code");
    assert_eq!(twin.len(), Shape::Plain.window_len(), "driver window is 1381 B");
    assert_bytes_match(&emp, &twin, "z80_sound_driver.emp vs .asm (plain $0000, DEBUG=0)");
}

/// THE windowed byte gate (DEBUG shape): z80_sound_driver.emp == the driver body at
/// the $0000 window, DEBUG=1. SAME SIZE (1381 B) as plain, but the three
/// after-sequencer call targets shift +$7E, so five call-site operands differ. Proves
/// the .emp reproduces the debug-shape link addresses exactly.
#[test]
fn z80_sound_driver_matches_as_twin_debug() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Debug, None);
    let twin = as_twin_bytes(Shape::Debug, None);
    assert!(!twin.is_empty(), "the AS twin must emit the driver code");
    assert_eq!(twin.len(), Shape::Debug.window_len(), "driver window is 1381 B (both shapes)");
    assert_bytes_match(&emp, &twin, "z80_sound_driver.emp vs .asm (debug $0000, DEBUG=1)");
}

/// Both windows are the SAME SIZE (1381 B) — the driver's own code is shape-invariant
/// in length (zero internal __DEBUG__ content).
#[test]
fn both_shapes_same_size() {
    assert_eq!(
        Shape::Plain.window_len(),
        Shape::Debug.window_len(),
        "the driver window is 1381 B in both shapes (no internal debug growth)"
    );
}

/// The two shapes DIFFER IN BYTES (the correction to the step-0 premise): the driver
/// is NOT byte-identical across shapes. Five `call` sites target Sequencer_StopAll /
/// Sfx_StopAll / SfxDispatch, which shift +$7E in the debug shape — so the plain and
/// debug byte images are NOT equal, which is exactly why BOTH must be gated.
#[test]
fn plain_and_debug_shapes_differ() {
    if skip_if_missing() {
        return;
    }
    let plain = compile_emp(Shape::Plain, None);
    let debug = compile_emp(Shape::Debug, None);
    assert_ne!(
        plain, debug,
        "the driver references shape-variant call targets — the two windows differ"
    );
    // Exactly the five shape-variant call operands differ: Sequencer_StopAll
    // ($CB2->$D30, x2) and Sfx_StopAll ($11AA->$1228, x2) each move both address
    // bytes (4+4=8); SfxDispatch ($E5D->$EDB, x1) shares its high byte $0E, so only
    // its low byte differs (+1). 9 differing bytes total, no more.
    let diffs = plain.iter().zip(debug.iter()).filter(|(a, b)| a != b).count();
    assert_eq!(diffs, 9, "exactly the 5 shape-variant call-target operands differ (9 bytes)");
}

/// Positive control (byte-gate non-triviality, t24): the UNDOCTORED .emp must DIVERGE
/// from a twin assembled with ONE doctored cross-file address (a moved Sfx_StopAll
/// changes the `call Sfx_StopAll` operand). Proves the comparison detects a
/// difference, not a vacuous match. Pins-derived: the reference is the twin
/// RE-ASSEMBLED each run, so a re-pin cannot re-stale it.
#[test]
fn emp_diverges_from_doctored_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, None);
    let twin = as_twin_bytes(Shape::Plain, Some(("Sfx_StopAll", 0x1ABC)));
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// Positive control on a CONSTANT (t24, second axis): doctoring an absolute RAM
/// address (SND_STAT_TICK, referenced across the driver) on the twin only must
/// diverge from the undoctored .emp — proves the const seam is load-bearing, not
/// inert.
#[test]
fn emp_diverges_from_doctored_const_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, None);
    let twin = as_twin_bytes(Shape::Plain, Some(("SND_STAT_TICK", 0x1DED)));
    assert_ne!(emp, twin, "a moved SND_STAT_TICK must change the driver's abs-mem operands");
}

/// Doctoring the SAME symbol on BOTH sides keeps them equal — the imm16/abs16 fixups
/// resolve to the shared address identically (the cell tracks the linked symbol, not
/// a frozen literal).
#[test]
fn doctored_both_sides_stay_equal() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Shape::Plain, Some(("SfxDispatch", 0x1234)));
    let twin = as_twin_bytes(Shape::Plain, Some(("SfxDispatch", 0x1234)));
    assert_bytes_match(&emp, &twin, "z80_sound_driver.emp vs twin (both SfxDispatch=$1234)");
}
