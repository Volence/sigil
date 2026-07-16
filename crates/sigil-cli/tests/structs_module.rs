//! Shared struct module (row 1051 micro-batch, item 1) — `engine/structs.emp`.
//!
//! `engine/structs.emp` is a TYPE-ONLY module (Act + Sec twins, zero emitted
//! bytes), so there is no region to byte-compare. Its gate is the drift wall:
//! lower the real file, supply the `Act_*`/`Sec_*` field equs (structs.asm's
//! generated layout), and assert every per-field `offsetof(S, f) ==
//! extern("S_f")` guard + the `sizeof == *_len` totals PASS. A doctored equ
//! must fire its guard (the negative probe).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test structs_module
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_harness::test_support;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

/// Lower `engine/structs.emp` and return its captured link asserts (the drift
/// wall), plus the placed equ sections to resolve them against.
fn lower_structs(equs: &[(&str, &str)]) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>) {
    let path = aeon_dir().join("engine/structs.emp");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "structs.emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon_dir().join("engine")),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "structs.emp lower errors: {ldiags:?}"
    );

    let mut sections = module.sections;
    let mut eqs = test_support::assemble_equ_pairs(equs);
    for sec in &mut eqs {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(eqs);
    (sections, module.link_asserts)
}

fn guard_diags(
    sections: &[Section],
    asserts: &[sigil_ir::LinkAssert],
) -> Vec<sigil_span::Diagnostic> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), asserts)
}

#[test]
fn per_field_drift_wall_passes() {
    if std::env::var("AEON_DIR").is_err() && !Path::new("/home/volence/sonic_hacks/aeon").exists() {
        eprintln!("skip: AEON_DIR unset");
        return;
    }
    let equs = test_support::act_sec_field_equs();
    let (sections, asserts) = lower_structs(&equs);
    // 34 per-field guards + 2 sizeof guards = 36; all must pass.
    assert!(asserts.len() >= 36, "expected >=36 drift guards, got {}", asserts.len());
    let diags = guard_diags(&sections, &asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "structs.emp drift wall must all PASS: {diags:?}"
    );
}

#[test]
fn doctored_field_offset_fires_its_guard() {
    if std::env::var("AEON_DIR").is_err() && !Path::new("/home/volence/sonic_hacks/aeon").exists() {
        eprintln!("skip: AEON_DIR unset");
        return;
    }
    // Swap Sec.sec_bg_layout to a wrong offset — the per-field guard must fire
    // (the case a sizeof-only guard would miss for a same-size neighbour).
    let mut equs = test_support::act_sec_field_equs();
    for pair in &mut equs {
        if pair.0 == "Sec_sec_bg_layout" {
            pair.1 = "$18"; // truth is $1C
        }
    }
    let (sections, asserts) = lower_structs(&equs);
    let diags = guard_diags(&sections, &asserts);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error && d.message.contains("sec_bg_layout")),
        "doctored Sec.sec_bg_layout must fire its drift guard, got: {diags:?}"
    );
}
