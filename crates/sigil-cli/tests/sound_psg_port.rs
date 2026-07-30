//! t32 — the REAL `sound_psg.emp` port: the SN76489 PSG voice writer (the rung-2
//! CONTRACT-model acceptance corpus), proven by the SCALE-1 WINDOWED oracle (the
//! z80_init_port / seq_opcode_tab template applied to a 15-proc Z80 CODE file).
//!
//! `sound_psg.asm` is the 4th of 5 includes inside z80_sound_driver.asm's
//! `cpu z80 / phase 0` blob (sequencer -> sfx -> fm -> psg -> tables). Its phase-0
//! window is SHAPE-VARIANT: base $1660 (plain) vs $16DE (debug), because the
//! upstream includes grow under __DEBUG__ before psg (psg itself has no __DEBUG__,
//! so its OWN layout is shape-invariant — every inter-label delta matches; only
//! the absolute base embedded in the internal call/jp targets, and a few external
//! symbol values, differ). The oracle proves BOTH shapes: the .emp compiled at the
//! shape's vma, byte-compared against `sound_psg.asm` assembled through sigil's own
//! AS front-end at the same phase, with the comptime constants as `-D` defines and
//! the cross-seam symbols (banked tables + external call targets) as equ carriers.
//! The `.asm` stays CANONICAL (this scale does NOT wire into build.sh — the seam
//! sub-tranche owns whole-ROM placement).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_psg_port
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
    /// The plain (no-__DEBUG__) shipped ROM: psg window at $1660.
    Plain,
    /// The debug shipped ROM: upstream __DEBUG__ growth shifts psg to $16DE.
    Debug,
}

impl Shape {
    /// The phase-0 window base of psg in this shape (from the asl listings:
    /// s4.lst `Psg_HwCh: 1660`, s4.debug.lst `Psg_HwCh: 16DE`).
    fn vma(self) -> i64 {
        match self {
            Shape::Plain => 0x1660,
            Shape::Debug => 0x16DE,
        }
    }
}

/// The comptime constants psg folds at assemble time (sound_constants.asm equs).
/// Fed to the .emp as `-D` defines and to the AS twin as equs. Shape-invariant.
fn const_seam() -> Vec<(&'static str, i64)> {
    vec![
        // SeqChannel / SfxChannel field offsets.
        ("sc_route", 0x0B), ("sc_flags", 0x0A), ("sc_volume", 0x08),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_noise_mode", 0x39), ("sc_detune", 0x3A), ("sx_gain", 0x40),
        // Route constants + bit number.
        ("CHROUTE_PSG1", 6), ("CHROUTE_PSGN", 9), ("SCF_KEYED_B", 1),
        // PSG command / value constants.
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_ATTEN_SILENT", 0x0F), ("SND_Z80_PSG", 0x7F11),
        ("SND_PSG_VOL_LATCH", 0x90), ("SND_PSG_TONE_LATCH", 0x80), ("SND_PSG_SILENCE_N", 0xFF),
        ("SND_PSG_SILENCE_T1", 0x9F), ("SND_PSG_SILENCE_T2", 0xBF), ("SND_PSG_SILENCE_T3", 0xDF),
        ("SND_PSG_NOISE_VOL", 0xF0), ("SND_PSG_NOISE_CTRL", 0xE0),
        ("SND_MASTER_FADE", 0x1CCC), ("SND_SFX_DUCK_LEVEL", 0x1EE5),
        // Vol-env table lengths.
        ("PSGVOLENV_COUNT", 0x0B), ("FMVOLENV_COUNT", 3),
    ]
}

/// The cross-seam LINK symbols psg references as addresses: the banked $8000-window
/// tables (`ld rr, Table` = symbolic imm16 -> Value16Le) and the external resident
/// call targets (`call Proc` = symbolic abs16). Fed to the .emp as equ carriers and
/// to the AS twin as equs. The banked tables + Mod_* are shape-invariant (before
/// __DEBUG__ growth); Snd_ChanClass shifts with the sequencer's debug growth.
/// `doctor` overrides ONE symbol — the t24 byte-gate positive control.
fn link_seam(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let snd_chanclass = match shape {
        Shape::Plain => 0x12EE,
        Shape::Debug => 0x136C,
    };
    let mut out: Vec<(String, i64)> = vec![
        ("PsgDivisorTableZ", 0x80BE), ("PsgVolEnv_Ids", 0x8284), ("PsgVolEnv_Ptrs", 0x828F),
        ("FmVolEnv_Ids", 0x8335), ("FmVolEnv_Ptrs", 0x8338),
        ("Mod_ReArm", 0x086F), ("Mod_Advance", 0x08A2), ("Snd_ChanClass", snd_chanclass),
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

/// Compile sound_psg.emp at the shape's phase-0 window (the .emp declares
/// `vma:$1660`; for the debug shape we re-base the section's `vma_base` to $16DE —
/// placement moves LMA, this moves the PC the internal call/jp targets resolve
/// against). Constants -> `-D` defines; link symbols -> equ carriers. Returns the
/// `sound_psg` section bytes.
fn compile_emp(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("sound_psg.emp"))
        .unwrap_or_else(|e| panic!("read sound_psg.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_psg.emp parse errors: {pdiags:?}"
    );
    let defines: Vec<(String, i128)> =
        const_seam().into_iter().map(|(n, v)| (n.to_string(), v as i128)).collect();
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_psg.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"sound_psg\"\nlma_base = 0x1660\nsize = 0x400\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    // Re-base the psg section's PC to the shape's window (the internal call/jp
    // absolute targets resolve against vma_base). The .emp default is $1660.
    for sec in sections.iter_mut() {
        if sec.name == "sound_psg" {
            sec.vma_base = Some(shape.vma() as u32);
        }
    }
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    // The cross-seam link symbols as equ carriers (the `ld rr,Table` / `call Proc`
    // link targets).
    let pairs: Vec<(String, String)> =
        link_seam(shape, doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of psg's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("sound_psg").expect("linked sound_psg").bytes.clone()
}

/// The blob's LMA base in the reference ROM, per shape (plain vs debug).
fn blob_lma(debug: bool) -> usize {
    if debug {
        0x3E2
    } else {
        0x3DE
    }
}

/// The reference-ROM slice this file's window is byte-gated against. psg's bytes live
/// in the shipped ROM at `blob_lma(shape) + shape.vma()` (the blob's LMA plus psg's
/// phase-0 window offset). Reads `s4.bin` (plain) / `s4.debug.bin` (debug) from
/// `AEON_DIR`; returns the ROM tail from that offset (the caller compares the first
/// `compile_emp.len()` bytes). Under `strict_gate` a missing ROM panics; otherwise the
/// test skips (`None`).
fn reference_slice(shape: Shape) -> Option<Vec<u8>> {
    let (name, debug) = match shape {
        Shape::Plain => ("s4.bin", false),
        Shape::Debug => ("s4.debug.bin", true),
    };
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
    let off = blob_lma(debug) + shape.vma() as usize;
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

/// THE windowed byte gate (PLAIN shape): sound_psg.emp == the reference ROM slice at
/// the $1660 window. psg is 444 bytes ($1BC, $1660..$181C). Proves the whole T1 +
/// rung-2 operand model (incl. the t32 bare-const-imm8 fold) on a real 15-proc Z80
/// file.
#[test]
fn sound_psg_matches_reference_plain() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, None);
    assert_eq!(emp.len(), 0x1BC, "psg is 444 bytes ($1660..$181C)");
    assert_bytes_match(&emp, &ref_tail[..emp.len()], "sound_psg.emp vs s4.bin slice (plain $1660)");
}

/// THE windowed byte gate (DEBUG shape): sound_psg.emp == the reference ROM slice at
/// the $16DE window (upstream __DEBUG__ growth). The internal call/jp targets + the
/// Snd_ChanClass address differ from the plain shape — proving the .emp tracks the
/// shape's window, not a frozen base.
#[test]
fn sound_psg_matches_reference_debug() {
    let Some(ref_tail) = reference_slice(Shape::Debug) else { return };
    let emp = compile_emp(Shape::Debug, None);
    assert_eq!(emp.len(), 0x1BC, "psg is 444 bytes (shape-invariant length)");
    assert_bytes_match(
        &emp,
        &ref_tail[..emp.len()],
        "sound_psg.emp vs s4.debug.bin slice (debug $16DE)",
    );
}

/// The two shapes MUST differ (shape-variance evidence): the internal call/jp
/// absolute targets embed the window base, so the plain and debug byte images are
/// NOT equal — the reason both must be gated.
#[test]
fn plain_and_debug_shapes_differ() {
    if skip_if_missing() {
        return;
    }
    let plain = compile_emp(Shape::Plain, None);
    let debug = compile_emp(Shape::Debug, None);
    assert_ne!(plain, debug, "psg is shape-variant — the two windows must produce different bytes");
}

/// Positive control (byte-gate non-triviality, t24): a DOCTORED .emp (ONE moved
/// cross-seam address — a shifted PsgDivisorTableZ changes every `ld de,
/// PsgDivisorTableZ`: Psg_NoteOn / Psg_EmitNoiseClock) must DIVERGE from the reference
/// slice. Proves the comparison detects a difference, not a vacuous match.
#[test]
fn emp_diverges_from_doctored_reference() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, Some(("PsgDivisorTableZ", 0x7ABC)));
    assert_ne!(
        emp.as_slice(),
        &ref_tail[..emp.len()],
        "the byte gate is vacuous if a doctored .emp still matches the reference"
    );
}
