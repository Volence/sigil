//! t33 — the REAL `sound_fm.emp` port: the YM2612 FM voice writer + patch load
//! (the INVARIANT-HEAVY half of the rung-2 CONTRACT model), proven by the SCALE-1
//! WINDOWED oracle (the sound_psg template applied to a 21-label / ~925-byte Z80
//! CODE file with deep push/pop LIFO nesting).
//!
//! `sound_fm.asm` is the 3rd of 5 includes inside z80_sound_driver.asm's
//! `cpu z80 / phase 0` blob (sequencer -> sfx -> fm -> psg -> tables). Its phase-0
//! window is SHAPE-VARIANT: base $12C3 (plain) vs $1341 (debug), because the
//! upstream includes grow under __DEBUG__ before fm (fm itself has no __DEBUG__,
//! so its OWN layout is shape-invariant — every inter-label delta is a constant
//! +$7E). Unlike psg, fm's ENTIRE external link seam is shape-INVARIANT: Mod_ReArm
//! ($86F) and all four banked tables sit before the __DEBUG__ growth, and
//! Snd_ChanClass — psg's shape-varying extern trust — is now DEFINED inside fm
//! (resolving via the section's own placement). So the two shapes differ ONLY in
//! the absolute base embedded in the internal call/jp targets. The oracle proves
//! BOTH shapes: the .emp compiled at the shape's vma, byte-compared against
//! `sound_fm.asm` assembled through sigil's own AS front-end at the same phase,
//! with the comptime constants as `-D` defines and the cross-seam symbols (banked
//! tables + Mod_ReArm) as equ carriers.
//! The `.asm` stays CANONICAL (this scale does NOT wire into build.sh — the seam
//! sub-tranche owns whole-ROM placement).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sound_fm_port
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
    /// The plain (no-__DEBUG__) shipped ROM: fm window at $12C3.
    Plain,
    /// The debug shipped ROM: upstream __DEBUG__ growth shifts fm to $1341 (+$7E).
    Debug,
}

impl Shape {
    /// The phase-0 window base of fm in this shape (from the asl listings:
    /// s4.lst `Fm_YmWrite: 12C3`, s4.debug.lst `Fm_YmWrite: 1341`).
    fn vma(self) -> i64 {
        match self {
            Shape::Plain => 0x12C3,
            Shape::Debug => 0x1341,
        }
    }
}

/// The comptime constants fm folds at assemble time (sound_constants.asm equs).
/// Fed to the .emp as `-D` defines and to the AS twin as equs. Shape-invariant.
fn const_seam() -> Vec<(&'static str, i64)> {
    vec![
        // SeqChannel / SfxChannel field offsets.
        ("sc_route", 0x0B), ("sc_patch", 0x06), ("sc_pan", 0x1B), ("sc_transpose", 0x1A),
        ("sc_detune", 0x3A), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_opbias", 0x1C),
        ("sc_flags", 0x0A), ("sc_fill_master", 0x25), ("sc_fill_count", 0x26),
        ("sc_env_cur", 0x28), ("sc_env_out", 0x29), ("sx_gain", 0x40), ("sx_patch_base", 0x3B),
        // YM2612 register numbers.
        ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_ALG_FB", 0xB0), ("SND_REG_LR_AMS_FMS", 0xB4),
        ("SND_REG_OP_DT_MUL", 0x30), ("SND_REG_OP_TL", 0x40), ("SND_REG_OP_RS_AR", 0x50),
        ("SND_REG_OP_AM_D1R", 0x60), ("SND_REG_OP_D2R", 0x70), ("SND_REG_OP_D1L_RR", 0x80),
        ("SND_REG_OP_SSG_EG", 0x90), ("SND_REG_FNUM_HI", 0xA4), ("SND_REG_FNUM_LO", 0xA0),
        ("SND_REG_KEY_ONOFF", 0x28),
        // YM2612 port addresses.
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_Z80_YM_A2", 0x4002), ("SND_Z80_YM_A3", 0x4003),
        // FM command / value / layout constants.
        ("SND_SFX_BASE", 0x1D00), ("SND_FM_TL_MAX", 0x7F), ("SND_FM_KEYON_OPMASK", 0xF0),
        ("CHROUTE_FM6", 5), ("FmPatch_len", 0x20), ("FmPatch_fp_tl", 6),
        ("SCF_KEYED_B", 1),
        // RegDelta reg_sel decode.
        ("REGDELTA_OP_MASK", 3), ("REGDELTA_GROUP_MASK", 0x0F),
        ("REGDELTA_GROUP_COUNT", 6), ("REGDELTA_GROUP_SHIFT", 2),
        // Pitch-table sizing + fnum block boundaries.
        ("PITCHTAB_MAX_IDX", 0x83), ("PITCHTAB_COUNT", 0x84), ("FMPITCH_MAX_IDX", 0x5E),
        ("FNUM_HI", 0x508), ("FNUM_LO", 0x284),
        // RAM address constants (used as absolute mem operands / pointer cells).
        ("SND_STAT_DAC_ACTIVE", 0x1F14), ("SND_MASTER_FADE", 0x1CCC),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SEQ_PATCHTAB", 0x1A02),
        ("Snd_PitchTabPtr", 0x1CA3),
        // The FM writer scratch base + span guard bound.
        ("SND_FM_SCRATCH", 0x1C9C), ("SND_FM_SCRATCH_LEN", 5),
    ]
}

/// The cross-seam LINK symbols fm references as addresses: the banked $8000-window
/// tables (`ld rr, Table` = symbolic imm16 -> Value16Le) and the one external
/// resident call target (`call Mod_ReArm` = symbolic abs16). Fed to the .emp as equ
/// carriers and to the AS twin as equs. ALL SHAPE-INVARIANT (before __DEBUG__
/// growth): fm defines Snd_ChanClass itself, so no seam symbol shifts by shape —
/// the shape-variance is entirely the section's own re-based internal call/jp
/// targets. `doctor` overrides ONE symbol — the t24 byte-gate positive control.
fn link_seam(doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let mut out: Vec<(String, i64)> = vec![
        ("FmPitchTableZ", 0x8000), ("LogVolumeLutZ", 0x817C), ("CarrierMaskTableZ", 0x827C),
        ("SndDefaultPitchTable", 0x8357), ("Mod_ReArm", 0x086F),
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

/// Compile sound_fm.emp at the shape's phase-0 window (the .emp declares
/// `vma:$12C3`; for the debug shape we re-base the section's `vma_base` to $1341).
/// Constants -> `-D` defines; link symbols -> equ carriers. Returns the
/// `sound_fm` section bytes.
fn compile_emp(shape: Shape, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("sound_fm.emp"))
        .unwrap_or_else(|e| panic!("read sound_fm.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "sound_fm.emp parse errors: {pdiags:?}"
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
        "sound_fm.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    // The module-level `const`/`ensure` items lower into an (empty) `text` section;
    // give it a throwaway region so place_sections is satisfied. The byte gate reads
    // only the `sound_fm` region.
    let map = "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"sound_fm\"\nlma_base = 0x12C3\nsize = 0x400\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    // Re-base the fm section's PC to the shape's window (the internal call/jp
    // absolute targets resolve against vma_base). The .emp default is $12C3.
    for sec in sections.iter_mut() {
        if sec.name == "sound_fm" {
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
        link_seam(doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of fm's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("sound_fm").expect("linked sound_fm").bytes.clone()
}

/// The blob's LMA base in the reference ROM, per shape (plain vs debug).
fn blob_lma(debug: bool) -> usize {
    if debug {
        0x3E2
    } else {
        0x3DE
    }
}

/// The reference-ROM slice this file's window is byte-gated against. fm's bytes live
/// in the shipped ROM at `blob_lma(shape) + shape.vma()` (the blob's LMA plus fm's
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

/// THE windowed byte gate (PLAIN shape): sound_fm.emp == the reference ROM slice at
/// the $12C3 window. fm is 925 bytes ($39D, $12C3..$1660). Proves the whole T1 +
/// rung-2 operand + contract model on a real 21-label Z80 file with deep push/pop LIFO
/// nesting (Fm_PatchOpGroup / Fm_PatchTlGroup) and the `push ix / pop hl|bc` move
/// idiom.
#[test]
fn sound_fm_matches_reference_plain() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, None);
    assert_eq!(emp.len(), 0x39D, "fm is 925 bytes ($12C3..$1660)");
    assert_bytes_match(&emp, &ref_tail[..emp.len()], "sound_fm.emp vs s4.bin slice (plain $12C3)");
}

/// THE windowed byte gate (DEBUG shape): sound_fm.emp == the reference ROM slice at
/// the $1341 window (upstream __DEBUG__ growth, +$7E). The internal call/jp targets
/// differ from the plain shape — proving the .emp tracks the shape's window, not a
/// frozen base.
#[test]
fn sound_fm_matches_reference_debug() {
    let Some(ref_tail) = reference_slice(Shape::Debug) else { return };
    let emp = compile_emp(Shape::Debug, None);
    assert_eq!(emp.len(), 0x39D, "fm is 925 bytes (shape-invariant length)");
    assert_bytes_match(
        &emp,
        &ref_tail[..emp.len()],
        "sound_fm.emp vs s4.debug.bin slice (debug $1341)",
    );
}

/// The two shapes MUST differ (shape-variance evidence): the internal call/jp
/// absolute targets embed the window base, so the plain and debug byte images are
/// NOT equal — the reason both must be gated. (fm's link seam is shape-invariant,
/// so this variance comes ENTIRELY from the re-based internal targets.)
#[test]
fn plain_and_debug_shapes_differ() {
    if skip_if_missing() {
        return;
    }
    let plain = compile_emp(Shape::Plain, None);
    let debug = compile_emp(Shape::Debug, None);
    assert_ne!(plain, debug, "fm is shape-variant — the two windows must produce different bytes");
}

/// Positive control (byte-gate non-triviality, t24): a DOCTORED .emp (ONE moved
/// cross-seam address — a shifted FmPitchTableZ changes every `ld de, FmPitchTableZ`:
/// Fm_NoteOn) must DIVERGE from the reference slice. Proves the comparison detects a
/// difference, not a vacuous match.
#[test]
fn emp_diverges_from_doctored_reference() {
    let Some(ref_tail) = reference_slice(Shape::Plain) else { return };
    let emp = compile_emp(Shape::Plain, Some(("FmPitchTableZ", 0x7ABC)));
    assert_ne!(
        emp.as_slice(),
        &ref_tail[..emp.len()],
        "the byte gate is vacuous if a doctored .emp still matches the reference"
    );
}
