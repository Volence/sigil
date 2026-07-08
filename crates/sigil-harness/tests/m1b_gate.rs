//! M1.B acceptance gate: prove the linker's byte-mutating passes reproduce the
//! reference ROM checksum, that multi-section 68k linking is byte-correct, and
//! that emit_listing is parseable by BOTH live tools (s4budget + Oracle).
//! The full sha256 ROM identity is sub-project D; here we gate the pieces B owns.
//!
//! This file has TWO gate classes:
//!
//! - HERMETIC tests (`multi_section_jsr_and_branch_link_correctly`,
//!   `abs_l_jmp_flows_through_emit_rom`) depend on nothing outside this repo,
//!   ALWAYS run, and ARE the CI-enforced M1.B gate. GitHub CI checks out only
//!   this repo, so these are what a green CI badge actually proves.
//!
//! - REFERENCE-DEPENDENT tests (`header_checksum_reproduces_reference_rom_18e`
//!   vs `aeon/s4.bin`, `s4budget_parses_emit_listing`,
//!   `oracle_loadfromaslisting_resolves_emit_listing`) require the sibling
//!   `aeon`/`oracle` repos (and python3/g++). Those are absent in GitHub CI, so
//!   these tests SKIP (return green) there by default. A green GitHub CI does
//!   NOT prove the checksum or listing-fidelity claims.
//!
//! To enforce the reference gates, run them in an environment that HAS the
//! sibling repos with the strict override, which turns a missing reference into
//! a hard failure instead of a skip:
//!
//! ```text
//! SIGIL_STRICT_GATE=1 cargo test -p sigil-harness --test m1b_gate
//! ```

use std::path::PathBuf;
use std::process::Command;

use sigil_ir::map::{MemoryMap, Region, RegionKind};
use sigil_ir::{Cpu, DataFragment, Expr, Fixup, FixupKind, Fragment, Label, Section, SymbolTable, SymbolValue};
use sigil_span::{SourceId, Span};

fn aeon_dir() -> PathBuf {
    PathBuf::from(std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()))
}
fn oracle_gui_dir() -> PathBuf {
    PathBuf::from(std::env::var("ORACLE_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/oracle".into()))
        .join("linux-port/gui")
}
fn sp() -> Span { Span { source: SourceId(0), start: 0, end: 0 } }

/// Reference-dependent gates skip when the sibling aeon/oracle repos are absent
/// (the default, e.g. GitHub CI which checks out only this repo). Set
/// SIGIL_STRICT_GATE=1 for a pre-merge run in an environment that HAS the
/// references, to turn "missing reference" into a hard failure instead of a skip.
fn strict_gate() -> bool { std::env::var("SIGIL_STRICT_GATE").is_ok() }

#[test]
fn header_checksum_reproduces_reference_rom_18e() {
    let rom_path = aeon_dir().join("s4.bin");
    let Ok(mut rom) = std::fs::read(&rom_path) else {
        if strict_gate() { panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin"); }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    assert!(rom.len() > 0x200, "reference ROM too small");
    let stored = ((rom[0x18E] as u16) << 8) | rom[0x18F] as u16;
    // Zero the stored word before recompute to prove the algorithm derives it
    // (0x18E is outside the [0x200,EOF) sum range, so this doesn't affect the sum).
    rom[0x18E] = 0; rom[0x18F] = 0;
    sigil_link::apply_header_checksum(&mut rom);
    let got = ((rom[0x18E] as u16) << 8) | rom[0x18F] as u16;
    assert_eq!(got, stored, "checksum mismatch: got {got:#06X}, ref {stored:#06X}");
}

#[test]
fn multi_section_jsr_and_branch_link_correctly() {
    // code@0: jsr Target ; bra.w Done ; Done: nop.  target@0x100: Target: rts.
    let code = Section {
        name: "code".into(), cpu: Cpu::M68000, vma_base: None, lma: 0,
        labels: vec![Label { name: "Done".into(), offset: 8 }],
        fragments: vec![
            Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("Target".into()), span: sp() },
            Fragment::Data(DataFragment {
                bytes: vec![0x60, 0x00, 0x00, 0x00],
                fixups: vec![Fixup { kind: FixupKind::PcRelDisp16, offset: 2, target: Expr::Sym("Done".into()) }],
                span: sp(),
            }),
            Fragment::Data(DataFragment { bytes: vec![0x4E, 0x71], fixups: vec![], span: sp() }),
        ],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
    };
    let target = Section {
        name: "target".into(), cpu: Cpu::M68000, vma_base: None, lma: 0x100,
        labels: vec![Label { name: "Target".into(), offset: 0 }],
        fragments: vec![Fragment::Data(DataFragment { bytes: vec![0x4E, 0x75], fixups: vec![], span: sp() })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
    };
    let map = MemoryMap::new(
        vec![Region { name: "rom".into(), lma_base: 0, size: 0x1000, kind: RegionKind::Rom, vma_base: None }],
        0x00,
    );
    let resolved = sigil_link::resolve_layout(&[code, target], &SymbolTable::new(), true).unwrap();
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).unwrap();
    let c = &linked.section("code").unwrap().bytes;
    // jsr Target(0x100) → abs.w 4E B8 01 00; bra.w Done: op@4, extword@6, disp=8-6=2 → 60 00 00 02; nop 4E 71.
    assert_eq!(c, &vec![0x4E, 0xB8, 0x01, 0x00, 0x60, 0x00, 0x00, 0x02, 0x4E, 0x71]);
    // emit_rom places both sections and validates the region.
    let rom = sigil_link::emit_rom(&linked, &map).unwrap();
    assert_eq!(&rom[0x100..0x102], &[0x4E, 0x75]); // Target: rts at LMA 0x100
}

#[test]
fn abs_l_jmp_flows_through_emit_rom() {
    // jmp to a high (>0x7FFF) target → abs.l (4EF9 + 32-bit operand), placed via emit_rom.
    let mut stubs = SymbolTable::new();
    stubs.define("Hi", SymbolValue::Int(0x12_3456));
    let code = Section {
        name: "code".into(), cpu: Cpu::M68000, vma_base: None, lma: 0, labels: vec![],
        fragments: vec![Fragment::JmpJsrSym { is_jsr: false, target: Expr::Sym("Hi".into()), span: sp() }],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
    };
    let map = MemoryMap::new(
        vec![Region { name: "rom".into(), lma_base: 0, size: 0x1000, kind: RegionKind::Rom, vma_base: None }],
        0x00,
    );
    let resolved = sigil_link::resolve_layout(&[code], &stubs, true).unwrap();
    let linked = sigil_link::link(&resolved, &stubs).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap();
    // jmp abs.l 4EF9 00123456 at LMA 0.
    assert_eq!(&rom[0..6], &[0x4E, 0xF9, 0x00, 0x12, 0x34, 0x56]);
}

#[test]
fn s4budget_parses_emit_listing() {
    let lst = sigil_link::emit_listing(&[
        sigil_link::ListingSymbol { name: "Main".into(), value: 0x1000, is_equate: false, unused: false },
        sigil_link::ListingSymbol { name: "OBJ_len".into(), value: 0x40, is_equate: true, unused: false },
    ]);
    let dir = std::env::temp_dir().join("sigil_m1b_lst");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("s4.lst");
    std::fs::write(&path, &lst).unwrap();
    let script = aeon_dir().join("tools/s4budget.py");
    let rom = aeon_dir().join("s4.bin");
    if !script.is_file() || !rom.is_file() {
        if strict_gate() { panic!("SIGIL_STRICT_GATE set but reference missing: s4budget.py or ref ROM"); }
        eprintln!("skip: s4budget.py or reference ROM absent");
        return;
    }
    // POSITIONAL CLI: <listing> <rom> --summary.
    let out = Command::new("python3").arg(&script).arg(&path).arg(&rom).arg("--summary").output();
    match out {
        Ok(o) => {
            // This test proves s4budget PARSES our emit_listing format without
            // error (exit 0 + a `ROM:` line). Actual symbol EXTRACTION (exact
            // resolved values) is proven by the complementary
            // `oracle_loadfromaslisting_resolves_emit_listing` test below.
            // s4budget's --summary line is printed to stderr (see s4budget.py:608,
            // `print(format_summary(...), file=sys.stderr)`); scan the combined
            // output so this gate is stream-agnostic. Exit 0 + a `ROM:` line both
            // prove the tool parsed our emit_listing successfully.
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            assert!(o.status.success(), "s4budget failed: {stderr}");
            let combined = format!("{stdout}{stderr}");
            assert!(combined.contains("ROM:"), "unexpected s4budget output: {combined}");
        }
        Err(_) => {
            if strict_gate() { panic!("SIGIL_STRICT_GATE set but reference missing: python3"); }
            eprintln!("skip: python3 unavailable");
        }
    }
}

#[test]
fn oracle_loadfromaslisting_resolves_emit_listing() {
    // The real M1.d Oracle gate: compile a micro-harness against the actual
    // oracle Symbols.cpp and confirm it resolves symbols from our emit_listing.
    let lst = sigil_link::emit_listing(&[
        sigil_link::ListingSymbol { name: "Main".into(), value: 0x1000, is_equate: false, unused: false },
        sigil_link::ListingSymbol { name: "OBJ_len".into(), value: 0x40, is_equate: true, unused: false },
    ]);
    let gui = oracle_gui_dir();
    let symbols_cpp = gui.join("Symbols.cpp");
    if !symbols_cpp.is_file() {
        if strict_gate() { panic!("SIGIL_STRICT_GATE set but reference missing: oracle Symbols.cpp / g++"); }
        eprintln!("skip: oracle Symbols.cpp not found at {} (set ORACLE_DIR)", symbols_cpp.display());
        return;
    }
    // g++ availability check.
    if Command::new("g++").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        if strict_gate() { panic!("SIGIL_STRICT_GATE set but reference missing: oracle Symbols.cpp / g++"); }
        eprintln!("skip: g++ unavailable");
        return;
    }
    let dir = std::env::temp_dir().join("sigil_m1b_oracle");
    std::fs::create_dir_all(&dir).unwrap();
    let lst_path = dir.join("s4.lst");
    std::fs::write(&lst_path, &lst).unwrap();
    let probe = dir.join("probe.cpp");
    std::fs::write(&probe, r#"#include "Symbols.h"
#include <cstdio>
int main(int argc, char** argv){
  SymbolTable t;
  if(!t.LoadFromAsListing(argv[1])){ printf("LOAD_FAILED\n"); return 2; }
  uint32_t a=0; bool ok = t.Lookup("Main", a);
  if(!ok || a != 0x1000){ printf("BAD Main=%06X ok=%d\n", a, ok); return 3; }
  ok = t.Lookup("OBJ_len", a);
  if(!ok || a != 0x40){ printf("BAD OBJ_len=%06X ok=%d\n", a, ok); return 4; }
  printf("OK\n");
  return 0;
}
"#).unwrap();
    let bin = dir.join("probe");
    let build = Command::new("g++")
        .args(["-std=c++17", "-I"]).arg(&gui)
        .arg(&probe).arg(&symbols_cpp).arg("-o").arg(&bin)
        .output().expect("run g++");
    assert!(build.status.success(), "harness build failed: {}", String::from_utf8_lossy(&build.stderr));
    let run = Command::new(&bin).arg(&lst_path).output().expect("run probe");
    assert!(run.status.success(),
        "Oracle did not resolve symbols from emit_listing: {}{}",
        String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
}
