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
//! THIS CHECKPOINT RUNS PER SHIPPED SHAPE, from sigil-cli. The gate lives in
//! sigil-cli because that crate depends on `sigil-harness` — the owner of the shape
//! `-D` profiles — as well as `sigil-frontend-emp`. Each residue proc is evaluated
//! under every shipped shape's own `-D` set (`native::shape_defines`) with that
//! shape's bound L1 interface env: a save/restore inside a
//! `DEBUG`/`CRASH_REPORT`/`SOUND_*`-gated arm of one of these procs lowers only when
//! that shape turns the arm on, so the checkpoint prediction must hold under all
//! seven shapes — the pin is shape-STABLE, not an artifact of an empty define set.
//! The number of `(proc, shape)` residue evaluations is reported as the non-vacuity
//! witness.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`).
//! Under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS so this checkpoint pin runs
//! in the standard strict invocation, not silently skip.

use sigil_frontend_emp::ast::{File as EmpFile, Item};
use sigil_frontend_emp::contract::InterfaceEnv;
use sigil_frontend_emp::corpus_contracts::bind_corpus_interfaces;
use sigil_frontend_emp::eval::eval_proc_body;
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::preserves::{verify_preserved, CallPolicy, PreserveStatus};
use sigil_frontend_emp::value::Reg;
use sigil_harness::native;
use sigil_ir::backend::Cpu;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn emp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == ".worktrees") {
                continue;
            }
            emp_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// The reference tree, or `None` when it is absent and strict mode is off (under
/// `SIGIL_STRICT_GATE` a missing tree HARD-FAILS — the house reference-gate pattern).
fn aeon_dir() -> Option<PathBuf> {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    );
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

/// Evaluate the named proc from `file_rel` under `defines` + `iface_env` and return
/// its residue register's LOCAL preserve status. `noreturn` is the whole-corpus
/// `@noreturn` set, so a tail into a declared divergent handler is recognized as an
/// obligation-free exit (oracle parity).
fn residue_status(
    file: &EmpFile,
    proc: &str,
    reg: Reg,
    defines: &[(String, i128)],
    iface_env: &InterfaceEnv,
    noreturn: &BTreeSet<String>,
) -> PreserveStatus {
    let p = file
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == proc => Some(p),
            _ => None,
        })
        .unwrap_or_else(|| panic!("proc {proc} not found"));
    let (buf, _d, _n) =
        eval_proc_body(file, &p.name, &p.params, &p.body, p.span, 0, Cpu::M68000, defines, iface_env);
    let buf = buf.unwrap_or_else(|| panic!("no codebuf for {proc}"));
    verify_preserved(&buf.items, &[reg], CallPolicy::ClobberAll, p.falls_into.as_deref(), noreturn)
        .remove(&reg)
        .unwrap()
}

#[test]
fn residue_procs_verify_as_predicted() {
    let Some(aeon) = aeon_dir() else { return };

    // Parse the whole corpus once (the env binding and the whole-corpus `@noreturn`
    // set both need every file), and index each residue proc's home file by module
    // path relative to `aeon`.
    let mut paths = Vec::new();
    emp_files(&aeon.join("engine"), &mut paths);
    emp_files(&aeon.join("games"), &mut paths);
    paths.sort();
    let files: Vec<(PathBuf, EmpFile)> = paths
        .iter()
        .map(|p| (p.clone(), parse_str(&std::fs::read_to_string(p).unwrap()).0))
        .collect();
    let just_files: Vec<EmpFile> = files.iter().map(|(_, f)| f.clone()).collect();

    // Whole-corpus `@noreturn` set (a tail into another file's divergent handler is
    // an obligation-free exit).
    let noreturn: BTreeSet<String> = just_files
        .iter()
        .flat_map(|f| f.items.iter())
        .filter_map(|i| match i {
            Item::Proc(p) if p.is_noreturn() => Some(p.name.clone()),
            Item::ExternProc(e) if e.is_noreturn() => Some(e.name.clone()),
            _ => None,
        })
        .collect();

    let file_for = |rel: &str| -> &EmpFile {
        let want = aeon.join(rel);
        &files.iter().find(|(p, _)| *p == want).unwrap_or_else(|| panic!("file {rel} not in corpus")).1
    };

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

    let mut report = String::from("\n== §5 verified-preserves over the 6 residue procs (per shape) ==\n");
    let mut mismatches = Vec::new();
    let mut evals = 0usize;
    for (label, profile) in native::shipped_shapes() {
        let aeon = aeon_dir().expect("aeon tree present");
        let defines =
            native::shape_defines(&profile, &aeon).expect("shape defines");
        let (iface_env, bind_diags) =
            bind_corpus_interfaces(&just_files, &defines, profile.game_module_prefix());
        assert!(
            bind_diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "shape `{label}`: L1 bind errors: {bind_diags:?}"
        );
        for (file_rel, proc, reg, expect) in cases {
            let got = residue_status(file_for(file_rel), proc, *reg, &defines, &iface_env, &noreturn);
            evals += 1;
            report.push_str(&format!("  [{label:<12}] {proc:<22} {reg:?}  -> {got:?}\n"));
            if got != *expect {
                mismatches.push(format!("[{label}] {proc} {reg:?}: expected {expect:?}, got {got:?}"));
            }
        }
    }
    eprintln!("{report}");
    assert!(mismatches.is_empty(), "residue verification drifted:\n{}", mismatches.join("\n"));

    // NON-VACUOUS: every residue proc across every shipped shape was evaluated.
    let expected = cases.len() * native::shipped_shapes().len();
    assert_eq!(
        evals, expected,
        "expected {} residue procs across {} shapes, evaluated {evals}",
        cases.len(),
        native::shipped_shapes().len()
    );
}
