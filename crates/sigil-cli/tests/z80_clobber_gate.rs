//! `sigil build`'s contract closure gate refuses a `(cpu: z80)` proc that
//! under-declares its TRANSITIVE clobbers (A5). Before, the Z80 closure
//! (`[call.clobbers-incomplete]`) ran only in a reference-dependent CI test over
//! the five resident sound modules; the build itself had no Z80 row at any
//! warning level, so an under-claim built green (reviewer B's e8, measured exit 0).
//!
//! The Z80 sub-machine reached only through a computed `ex (sp),hl` dispatch is
//! excluded BY ANNOTATION, `@allow("clobbers.computed-dispatch", "<reason>")`,
//! never by name.
//!
//! The real binary over a self-contained fixture tree (one Z80 module and an empty
//! game map), the same harness as `clobber_gate_location.rs`. THE EXIT CODE
//! CARRIES NO SIGNAL HERE: the gate's frozen baselines are sized to aeon's corpus
//! and fail over a tree this small too, so every assertion reads the refusal TEXT,
//! with the gate's own verdict line as the control that the gate ran at all.

use std::path::Path;
use std::process::Command;

/// `Audit_Q` declares what it writes. `Audit_P` declares `a, f` but calls
/// `Audit_Q`, which brings `b`: one transitive firing, carried by `Audit_Q`.
const Z80: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    pub proc Audit_Q () clobbers(a, b, f) {
        ld      b, 0
        ld      a, 0
        ret
    }
    pub proc Audit_P () clobbers(a, f) {
        call    Audit_Q
        ret
    }
}
";

/// The same tree with `Audit_P` declared reached only through a computed dispatch.
const Z80_ALLOWED: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    pub proc Audit_Q () clobbers(a, b, f) {
        ld      b, 0
        ld      a, 0
        ret
    }
    @allow(\"clobbers.computed-dispatch\", \"fixture\")
    pub proc Audit_P () clobbers(a, f) {
        call    Audit_Q
        ret
    }
}
";

const HEADER: &str = "error: [call.clobbers-incomplete] (Z80) closure firings";
const VERDICT: &str = "error: contract closure gate FAILED.";

fn tree(root: &Path, src: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(root.join("engine")).unwrap();
    std::fs::create_dir_all(root.join("games/sonic4")).unwrap();
    let f = root.join("engine/z80fix.emp");
    std::fs::write(&f, src).unwrap();
    std::fs::write(root.join("games/sonic4/map.toml"), "").unwrap();
    f
}

/// `sigil build --native --game sonic4` over `root`, returning stderr.
fn build_stderr(root: &Path, contracts_off: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon"]).arg(root).args(["--native", "--game", "sonic4", "-o"]);
    cmd.arg(root.join("out.bin"));
    if contracts_off {
        cmd.env("SIGIL_CONTRACTS", "0");
    } else {
        cmd.env_remove("SIGIL_CONTRACTS");
    }
    let out = cmd.output().expect("run sigil build");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `file:line:col` of the first occurrence of `needle` in `src`: 1-based line,
/// 1-based column of the needle itself. A proc declaration's span starts at its
/// `proc` keyword (after any `pub`), so the needle is `"proc NAME"`.
fn loc(file: &Path, src: &str, needle: &str) -> String {
    let (i, col) = src
        .lines()
        .enumerate()
        .find_map(|(i, l)| l.find(needle).map(|c| (i, c + 1)))
        .unwrap_or_else(|| panic!("fixture has no line containing {needle:?}"));
    format!("{}:{}:{col}", file.display(), i + 1)
}

#[test]
fn the_build_gate_refuses_a_z80_clobber_underclaim_with_its_location() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tree(tmp.path(), Z80);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains(
            "error: [call.clobbers-incomplete] (Z80) closure firings, 1 firing(s); this family \
             is zero-firing by contract:"
        ),
        "the gate must refuse the Z80 closure family with its count, got:\n{err}"
    );
    let p = format!(
        "  {}: [call.clobbers-incomplete] `Audit_P` destroys `b` through `Audit_Q`, which its \
         clobbers(...) does not declare",
        loc(&f, Z80, "proc Audit_P")
    );
    assert!(err.contains(&p), "missing refusal line\n{p}\ngot:\n{err}");
    // The honest callee is not refused.
    assert!(!err.contains("`Audit_Q` destroys"), "Audit_Q is honest, got:\n{err}");
}

/// An `@allow("clobbers.computed-dispatch", …)` proc is out of the direct-call
/// closure's scope: the row is absent, while the gate still ran (its verdict line).
#[test]
fn a_computed_dispatch_allow_excludes_the_proc() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80_ALLOWED);
    let err = build_stderr(tmp.path(), false);
    assert!(err.contains(VERDICT), "the control must reach the gate's verdict, got:\n{err}");
    assert!(!err.contains(HEADER), "an excluded proc must not be refused, in:\n{err}");
}

/// CONTROL: `SIGIL_CONTRACTS=0` skips the gate before any Z80 refusal.
#[test]
fn sigil_contracts_0_still_skips_the_z80_family() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80);
    let err = build_stderr(tmp.path(), true);
    assert!(
        err.contains("warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)."),
        "the opt-out must say it skipped, got:\n{err}"
    );
    assert!(!err.contains(HEADER), "no Z80 refusal expected when skipped, in:\n{err}");
}

/// `--report contracts` prints the Z80 family with its count, the excluded count,
/// and the same carrier the refusal names.
#[test]
fn the_contract_report_prints_the_z80_family() {
    let report_of = |src: &str| {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path(), src);
        let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
            .args(["build", "--aeon"])
            .arg(tmp.path())
            .args(["--native", "--game", "sonic4", "--report", "contracts"])
            .env_remove("SIGIL_CONTRACTS")
            .output()
            .expect("run sigil build --report contracts");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let report = report_of(Z80);
    assert!(
        report.contains(
            "-- [call.clobbers-incomplete] (Z80) closure firings (1), 0 excluded by \
             @allow(\"clobbers.computed-dispatch\") --"
        ),
        "got:\n{report}"
    );
    let rows: Vec<&str> = report.lines().map(str::trim_end).collect();
    assert!(
        rows.contains(&format!("  {:<28} transitive b via Audit_Q", "Audit_P").as_str()),
        "got:\n{report}"
    );
    let report = report_of(Z80_ALLOWED);
    assert!(
        report.contains(
            "-- [call.clobbers-incomplete] (Z80) closure firings (0), 1 excluded by \
             @allow(\"clobbers.computed-dispatch\") --"
        ),
        "got:\n{report}"
    );
    assert!(report.contains("  EXCLUDED  Audit_P"), "got:\n{report}");
}

// ===========================================================================
// Fix round 1: the blindness rows, the remedy hint, and the hatch's bound.
// ===========================================================================

/// A register-relevant Z80 drop (`sizeof` of a type that does not exist).
const Z80_DROP: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    pub proc Audit_D () clobbers(a, f) {
        ld      a, sizeof(NoSuchType)
        ret
    }
}
";

/// A Z80 call into nothing: a hole in the closure.
const Z80_HOLE: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    pub proc Audit_H () clobbers(a, f) {
        call    Nowhere_Z80
        ret
    }
}
";

/// An annotated proc that an unannotated proc `call`s: the hatch's bound.
const Z80_CALLED: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    @allow(\"clobbers.computed-dispatch\", \"fixture\")
    pub proc Audit_T () clobbers(a, f) {
        ld      a, 0
        ret
    }
    pub proc Audit_C () clobbers(a, f) {
        call    Audit_T
        ret
    }
}
";

/// An annotated proc an unannotated proc only JUMPS to: allowed.
const Z80_JUMPED: &str = "module engine.z80fix (cpu: z80)
section z80fix (cpu: z80) {
    @allow(\"clobbers.computed-dispatch\", \"fixture\")
    pub proc Audit_T () clobbers(a, f) {
        ld      a, 0
        ret
    }
    pub proc Audit_J () clobbers(a, f) {
        jp      Audit_T
    }
}
";

#[test]
fn a_register_relevant_z80_drop_is_refused_as_blindness() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80_DROP);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains(
            "error: the Z80 clobber closure DROPPED 1 instruction(s), so its \
             [call.clobbers-incomplete] row is under-approximating."
        ),
        "got:\n{err}"
    );
    assert!(err.contains("  Audit_D: 1"), "the dropping proc is named, got:\n{err}");
    assert!(err.contains("error: contract closure gate FAILED, the analysis is blind."), "got:\n{err}");
}

#[test]
fn a_z80_hole_is_refused_as_blindness() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80_HOLE);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains(
            "error: the Z80 clobber closure has 1 unresolved callee(s), whose effect it cannot charge:"
        ),
        "got:\n{err}"
    );
    assert!(err.contains("  Nowhere_Z80\n"), "the hole is named, got:\n{err}");
    assert!(err.contains("error: contract closure gate FAILED, the analysis is blind."), "got:\n{err}");
}

/// `Audit_P` has no direct caller, so the refusal adds the remedy hint; `Audit_Q`
/// (called by `Audit_P`) is not named in it.
#[test]
fn a_firing_with_no_direct_caller_carries_the_dispatch_hint() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80);
    let err = build_stderr(tmp.path(), false);
    let hint = err.lines().find(|l| l.starts_with("  hint: ")).unwrap_or_else(|| panic!("no hint, got:\n{err}"));
    assert!(hint.starts_with("  hint: `Audit_P` has no direct caller in the Z80 corpus."), "{hint}");
    assert!(hint.contains("`@allow(\"clobbers.computed-dispatch\", \"<reason>\")`"), "{hint}");
    assert!(!hint.contains("Audit_Q"), "{hint}");
}

#[test]
fn an_annotated_proc_with_a_direct_caller_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tree(tmp.path(), Z80_CALLED);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains(
            "error: [clobbers.computed-dispatch-called] firings, 1 firing(s); this family is \
             zero-firing by contract:"
        ),
        "got:\n{err}"
    );
    let line = format!(
        "  {}: [clobbers.computed-dispatch-called] `Audit_T` declares \
         @allow(\"clobbers.computed-dispatch\") but `Audit_C` calls it directly",
        loc(&f, Z80_CALLED, "proc Audit_T")
    );
    assert!(err.contains(&line), "missing\n{line}\ngot:\n{err}");
}

#[test]
fn an_annotated_proc_reached_only_by_a_jump_is_allowed() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), Z80_JUMPED);
    let err = build_stderr(tmp.path(), false);
    assert!(err.contains(VERDICT), "the control must reach the gate's verdict, got:\n{err}");
    assert!(!err.contains("[clobbers.computed-dispatch-called]"), "got:\n{err}");
}

/// The same Z80 proc name declared in two modules: one node would overwrite the other.
#[test]
fn a_z80_name_declared_twice_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tree(tmp.path(), Z80);
    let second = tmp.path().join("engine/z80fix2.emp");
    let src2 = "module engine.z80fix2 (cpu: z80)
section z80fix2 (cpu: z80) {
    pub proc Audit_Q () clobbers(a, b, f) {
        ld      b, 0
        ret
    }
}
";
    std::fs::write(&second, src2).unwrap();
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains("error: Z80 name collisions, 1 firing(s); this family is zero-firing by contract:"),
        "got:\n{err}"
    );
    let a = format!("  {}: `Audit_Q` is declared twice", loc(&first, Z80, "proc Audit_Q"));
    let b = format!("  {}: `Audit_Q` is declared twice", loc(&second, src2, "proc Audit_Q"));
    assert!(err.contains(&a) || err.contains(&b), "missing a located collision line, got:\n{err}");
}
