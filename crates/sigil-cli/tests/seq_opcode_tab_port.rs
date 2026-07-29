//! t27 phase-2 lane B — the REAL `seq_opcode_tab.emp` port: the sequencer
//! coordination-opcode jump table (32 × `dc.w <resident-Z80-label>`), proven by
//! the WINDOWED oracle (the z80_init_port template applied to a banked data
//! table). Closes gap-ledger row ~825-831 (Z80 LE data unprobed at file scale)
//! for the symbolic-cell half: P1 (Z80 little-endian data) + P2 (a banked
//! `(cpu:z80, vma:$8000)` section whose cells are resident phase-0 addresses)
//! on the real file.
//!
//! `seq_opcode_tab.asm` is included inside main.asm's `cpu z80 / phase 08000h`
//! block — a CANONICAL banked head table, not a standalone assembly. The oracle
//! is the AS twin assembled through sigil's own AS front-end at `cpu z80`, with
//! the ~26 resident `Seq_Op_*`/`Seq_BadOpcode` handler addresses shared as equs
//! by both sides. The .emp compiled at `(cpu:z80, vma:$8000)` must byte-match.
//! The `.asm` stays canonical (this scale does NOT wire into build.sh).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seq_opcode_tab_port
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

/// The unique resident `Seq_Op_*`/`Seq_BadOpcode` handler labels the table cells
/// reference, each a distinct resident phase-0 Z80 address (≤ $FFFF, so the
/// `Value16Le` window check is satisfied). Shared by the .emp (as equ carriers
/// for its bareword `dc.w` cells) and the AS twin (as equs). `doctor` overrides
/// ONE symbol's value — the t24 positive control (it changes that cell's bytes).
fn seq_seam(doctor: Option<(&str, i64)>) -> Vec<(String, i64)> {
    let names = [
        "Seq_Op_Vol", "Seq_Op_Patch", "Seq_Op_Dac", "Seq_Op_NoteDur", "Seq_Op_Pan",
        "Seq_Op_RepeatStart", "Seq_Op_RepeatEnd", "Seq_Op_NoteRaw", "Seq_Op_PitchEnv",
        "Seq_Op_OpBias", "Seq_Op_RegDelta", "Seq_Op_PsgEnv", "Seq_Op_ModSet", "Seq_Op_NoteFill",
        "Seq_Op_LoopPoint", "Seq_Op_Jump", "Seq_Op_SpinRev", "Seq_BadOpcode", "Seq_Op_PsgNoise",
        "Seq_Op_Tempo", "Seq_Op_Lfo", "Seq_Op_Porta", "Seq_Op_Detune", "Seq_Op_RegWrite",
        "Seq_Op_Macro", "Seq_Op_End",
    ];
    names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            // Distinct resident addresses $0100, $0200, … (all < $8000).
            let base = 0x0100 + (i as i64) * 0x0100;
            let v = match doctor {
                Some((dn, dv)) if dn == *n => dv,
                _ => base,
            };
            (n.to_string(), v)
        })
        .collect()
}

/// Compile seq_opcode_tab.emp at `(cpu:z80, vma:$8000)`, resolve + link with the
/// resident symbols as equ carriers, return the `seq_opcode_tab` section bytes.
fn compile_emp(doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let dir = aeon.join("engine/sound");
    let src = std::fs::read_to_string(dir.join("seq_opcode_tab.emp"))
        .unwrap_or_else(|e| panic!("read seq_opcode_tab.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "seq_opcode_tab.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "seq_opcode_tab.emp lower errors: {:?}",
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    let map = "fill = 0x00\n\n[[region]]\nname = \"seq_opcode_tab\"\nlma_base = 0x8000\nsize = 0x100\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map).expect("map loads");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    // The resident symbols as equ carriers (the `dc.w <label>` link targets).
    let pairs: Vec<(String, String)> =
        seq_seam(doctor).into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000; // clear of the table's LMA
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    linked.section("seq_opcode_tab").expect("linked seq_opcode_tab").bytes.clone()
}

/// The AS twin oracle: assemble seq_opcode_tab.asm through sigil's AS front-end
/// at `cpu z80`, with the resident symbols as equs, and return the emitted bytes.
fn as_twin_bytes(doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let body = std::fs::read_to_string(aeon.join("engine/sound/seq_opcode_tab.asm"))
        .unwrap_or_else(|e| panic!("read seq_opcode_tab.asm: {e}"));
    // The resident symbols are defined as equs in the default (68000) mode,
    // where `$` hex parses; the table body then assembles under `cpu z80` /
    // `phase 08000h` (the real build's head placement — the cell values are
    // absolute equ references, so placement does not affect the bytes, but keep
    // it faithful). `dephase` closes the phased block.
    let mut seam = String::from("cpu 68000\n");
    for (name, val) in seq_seam(doctor) {
        seam.push_str(&format!("{name} = ${val:X}\n"));
    }
    seam.push_str("cpu z80\nphase 08000h\n");
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
    if !aeon.join("engine/sound/seq_opcode_tab.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return true;
    }
    false
}

/// THE windowed byte gate: seq_opcode_tab.emp == seq_opcode_tab.asm. Proves the
/// resident-Z80-address `Value16Le` value cell (phase-1(c)) on the real 32-entry
/// table — P1 (Z80 LE data) + P2 (banked section link values) at file scale.
#[test]
fn seq_opcode_tab_matches_as_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None);
    let twin = as_twin_bytes(None);
    assert!(!twin.is_empty(), "the AS twin must emit the opcode table");
    assert_eq!(twin.len(), 32 * 2, "the table is 32 dw entries (64 bytes)");
    assert_bytes_match(&emp, &twin, "seq_opcode_tab.emp vs seq_opcode_tab.asm");
}

/// Positive control (byte-gate non-triviality, t24): the UNDOCTORED .emp must
/// DIVERGE from a twin assembled with ONE doctored resident address — proving
/// the comparison genuinely detects a per-cell difference, not a vacuous match.
#[test]
fn emp_diverges_from_doctored_twin() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(None);
    let twin = as_twin_bytes(Some(("Seq_Op_Vol", 0x7ABC))); // one handler moved
    assert_ne!(emp, twin, "the byte gate is vacuous if the .emp matches a doctored twin");
}

/// Doctoring the SAME symbol on BOTH sides keeps them equal — the `Value16Le`
/// cell resolves to the shared address identically (the bareword cell tracks the
/// linked symbol, not a frozen literal).
#[test]
fn doctored_both_sides_stay_equal() {
    if skip_if_missing() {
        return;
    }
    let emp = compile_emp(Some(("Seq_Op_Vol", 0x7ABC)));
    let twin = as_twin_bytes(Some(("Seq_Op_Vol", 0x7ABC)));
    assert_bytes_match(&emp, &twin, "seq_opcode_tab.emp vs twin (both Seq_Op_Vol=$7ABC)");
}
