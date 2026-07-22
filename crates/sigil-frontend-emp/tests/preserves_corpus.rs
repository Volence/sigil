//! Contract-grammar v2 §5 — the verified-`preserves` analysis run over the REAL
//! aeon corpus: the six G1-residue procs, each checked against its residue
//! register. This is the G3 pre-retrofit checkpoint measurement, pinned.
//!
//! Prediction (G1 residue table): the FIVE local-preservation procs verify by
//! their own save/restore (individual-push a0; mid-body movem d1); Load_Object's
//! a0 does NOT verify LOCALLY — it never touches a0 itself and only clears
//! TRANSITIVELY once AllocDynamic declares+verifies `preserves(a0)` (the closure
//! subtraction, not local preservation). A local NotPreserved for Load_Object is
//! therefore CORRECT, not a failure — exactly the nuance the checkpoint surfaces.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`).
//! Under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS so this checkpoint pin runs
//! in the standard strict invocation, not silently skip.

use sigil_frontend_emp::ast::Item;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::preserves::{verify_preserved, PreserveStatus};
use sigil_frontend_emp::value::Reg;
use sigil_ir::backend::Cpu;
use std::path::PathBuf;

/// Evaluate one named proc from `file_rel` and return its residue register's
/// preserve status.
fn residue_status(aeon: &PathBuf, file_rel: &str, proc: &str, reg: Reg) -> PreserveStatus {
    let src = std::fs::read_to_string(aeon.join(file_rel))
        .unwrap_or_else(|e| panic!("read {file_rel}: {e}"));
    let (file, _diags) = parse_str(&src);
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == proc => Some(p),
            _ => None,
        })
        .unwrap_or_else(|| panic!("proc {proc} not found in {file_rel}"));
    let (buf, _d, _n) =
        eval_proc_body(&file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, &[]);
    let buf = buf.unwrap_or_else(|| panic!("no codebuf for {proc}"));
    verify_preserved(&buf.items, &[reg]).remove(&reg).unwrap()
}

#[test]
fn residue_procs_verify_as_predicted() {
    // House reference-gate pattern (repin_pins/mt_port, c5505f8): default the
    // sibling aeon tree; under SIGIL_STRICT_GATE a missing reference hard-fails so
    // this checkpoint pin actually runs under the standard strict invocation.
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return;
    }

    // (file, proc, residue reg, expected LOCAL status)
    let cases: &[(&str, &str, Reg, PreserveStatus)] = &[
        // individual-push a0 (branch-straddling) — verify locally.
        ("engine/objects/core.emp", "AllocDynamic", Reg::A0, PreserveStatus::Verified),
        // individual-push a0 with a (sp) peek — verify locally.
        ("engine/objects/entity_window.emp", "Collected_ParkSlot", Reg::A0, PreserveStatus::Verified),
        ("engine/objects/entity_window.emp", "Collected_UnparkSlot", Reg::A0, PreserveStatus::Verified),
        // mid-body movem d1 around Collected_FindSlot — verify locally.
        ("engine/objects/entity_window.emp", "Collected_CheckRing", Reg::D1, PreserveStatus::Verified),
        ("engine/objects/entity_window.emp", "Killed_CheckObject", Reg::D1, PreserveStatus::Verified),
        // inherited a0 — Load_Object never touches a0; clears TRANSITIVELY, not
        // locally. Local NotPreserved is correct.
        ("engine/objects/load_object.emp", "Load_Object", Reg::A0, PreserveStatus::NotPreserved),
    ];

    let mut report = String::from("\n== §5 verified-preserves over the 6 residue procs ==\n");
    let mut mismatches = Vec::new();
    for (file, proc, reg, expect) in cases {
        let got = residue_status(&aeon, file, proc, *reg);
        report.push_str(&format!("  {proc:<24} {reg:?}  -> {got:?}\n"));
        if got != *expect {
            mismatches.push(format!("{proc} {reg:?}: expected {expect:?}, got {got:?}"));
        }
    }
    eprintln!("{report}");
    assert!(mismatches.is_empty(), "residue verification drifted:\n{}", mismatches.join("\n"));
}
