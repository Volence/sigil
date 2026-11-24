//! End-to-end CLI contract gate + `--report contracts` tests for step A piece 4:
//! - W6 blind 68000 calls/jumps (`jsr 4(a1)`, untargeted `jmp .tbl(pc, d0.w)`)
//!   stop `sigil build` and appear in `--report contracts` `[blind-transfers]`;
//! - W4 `requires(<ctx>)` on `ContractType` stops `sigil build` when unsatisfied
//!   and passes when satisfied;
//! - `baseline.toml` `format = 2` ratchets `clobber_unbounded` and `sr_level` in
//!   both directions (added and stale) while keeping concrete register firings
//!   strictly zero-firing;
//! - `format = 1` descriptions are refused by version before key checking.

use std::path::Path;
use std::process::Command;

fn run_sigil(root: &Path, extra_args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--project"])
        .arg(root.join("games/g"))
        .args(["--shape", "release"])
        .args(extra_args)
        .env_remove("AEON_DIR")
        .env_remove("SIGIL_CONTRACTS");
    cmd.output().expect("run sigil build")
}

#[test]
fn w6_blind_transfers_stop_build_and_appear_in_contracts_report() {
    let tmp = tempfile::tempdir().unwrap();
    sigil_harness::test_support::write_project_fixture(tmp.path(), "g");
    std::fs::write(
        tmp.path().join("engine/blind.emp"),
        "module engine.blind
pub proc DispCall () clobbers(d0) {
    jsr     4(a1)
    rts
}
pub proc JumpTbl () clobbers(d0) {
    jmp     .tbl(pc, d0.w)
.tbl:
    rts
}
",
    )
    .unwrap();

    let out_bin = tmp.path().join("out.bin");
    let build = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!build.status.success(), "expected contract gate failure on blind transfers");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("[proc.clobber-undeclared] `DispCall` reaches an indirect call"),
        "expected DispCall unbounded firing:\n{stderr}"
    );
    assert!(
        stderr.contains("[proc.clobber-undeclared] `JumpTbl` reaches an indirect call"),
        "expected JumpTbl unbounded firing:\n{stderr}"
    );

    let rep = run_sigil(tmp.path(), &["--report", "contracts"]);
    assert!(
        rep.status.success(),
        "expected --report contracts success:\n{}",
        String::from_utf8_lossy(&rep.stderr)
    );
    let stdout = String::from_utf8_lossy(&rep.stdout);
    assert!(
        stdout.contains("-- [blind-transfers] unseen 68000 calls/jumps (2): --"),
        "expected blind-transfers header with 2 entries:\n{stdout}"
    );
    assert!(
        stdout.contains("DispCall")
            && stdout.contains("untyped-call")
            && stdout.contains("engine/blind.emp:3:"),
        "expected DispCall untyped-call row with line location:\n{stdout}"
    );
    assert!(
        stdout.contains("JumpTbl")
            && stdout.contains("untargeted-jump")
            && stdout.contains("engine/blind.emp:7:"),
        "expected JumpTbl untargeted-jump row with line location:\n{stdout}"
    );
}

#[test]
fn w4_contract_type_requires_stops_unsatisfied_build_and_passes_when_satisfied() {
    let tmp = tempfile::tempdir().unwrap();
    sigil_harness::test_support::write_project_fixture(tmp.path(), "g");
    let header = "module engine.w4
context vblank {
    acquire = asm { nop }
    release = asm { nop }
}
type VbHook = proc () requires(vblank) clobbers(d0)
";
    // Unsatisfied: Caller does not hold `vblank`.
    std::fs::write(
        tmp.path().join("engine/w4.emp"),
        format!(
            "{header}pub proc Caller () clobbers(d0) {{\n    jsr (a0) as VbHook\n    rts\n}}\n"
        ),
    )
    .unwrap();

    let out_bin = tmp.path().join("out.bin");
    let bad = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!bad.status.success(), "expected [context.unsatisfied] to stop build");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("[context.unsatisfied]"),
        "expected [context.unsatisfied] refusal:\n{stderr}"
    );
    assert!(
        stderr.contains("as VbHook") && stderr.contains("vblank"),
        "expected callee `as VbHook` and context `vblank`:\n{stderr}"
    );

    // Satisfied: Caller declares `requires(vblank)`.
    std::fs::write(
        tmp.path().join("engine/w4.emp"),
        format!(
            "{header}pub proc Caller () requires(vblank) clobbers(d0) {{\n    jsr (a0) as VbHook\n    rts\n}}\n"
        ),
    )
    .unwrap();
    let ok = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    let ok_err = String::from_utf8_lossy(&ok.stderr);
    assert!(
        ok_err.contains("contract closure gate: passed ("),
        "expected satisfied W4 build to pass contract gate:\n{ok_err}"
    );
}

#[test]
fn format2_clobber_unbounded_and_sr_level_ratchets_enforce_both_directions() {
    let tmp = tempfile::tempdir().unwrap();
    sigil_harness::test_support::write_project_fixture(tmp.path(), "g");
    let base_src = "module engine.ratchet
pub proc BlindProc () clobbers(d0) preserves(sr.mask, d7.w) {
    jmp     .tbl(pc, d0.w)
.tbl:
    rts
}
";
    std::fs::write(tmp.path().join("engine/ratchet.emp"), base_src).unwrap();
    std::fs::write(
        tmp.path().join("engine/baseline.toml"),
        "format = 2\nclobber_unbounded = [\"BlindProc\"]\nsr_level = [\"BlindProc\"]\nword_preserve_unverifiable = [[\"BlindProc\", \"d7.w\"]]\n",
    )
    .unwrap();

    // 1. Exact baseline match passes the contract closure gate.
    let out_bin = tmp.path().join("out.bin");
    let ok = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    let ok_err = String::from_utf8_lossy(&ok.stderr);
    assert!(
        ok_err.contains("contract closure gate: passed ("),
        "expected exact format=2 baseline to pass:\n{ok_err}"
    );

    // 2. Concrete register under-declaration STILL fails even when `clobber_unbounded` is populated.
    std::fs::write(
        tmp.path().join("engine/ratchet.emp"),
        format!(
            "{base_src}pub proc ConcreteBad () clobbers(d0) {{\n    moveq #0, d2\n    rts\n}}\n"
        ),
    )
    .unwrap();
    let concrete = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!concrete.status.success(), "concrete clobber firing must still fail");
    let conc_err = String::from_utf8_lossy(&concrete.stderr);
    assert!(
        conc_err.contains("[proc.clobber-undeclared] `ConcreteBad` destroys `d2`"),
        "expected located ConcreteBad refusal:\n{conc_err}"
    );

    // 3. Added unbounded + sr_level + word_preserve_unverifiable proc fails with adjudication and location.
    // Name it `BlindProc_Extra` so its prefix matches `BlindProc` and exercises exact-key span matching.
    std::fs::write(
        tmp.path().join("engine/ratchet.emp"),
        format!(
            "{base_src}pub proc BlindProc_Extra () clobbers(d0) preserves(sr.mask, d7.w) {{\n    jmp (a0)\n}}\n"
        ),
    )
    .unwrap();
    let added = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!added.status.success(), "added unbounded/sr_level/word_preserve firing must fail");
    let added_err = String::from_utf8_lossy(&added.stderr);
    assert!(
        added_err.contains("[proc.clobber-undeclared] (unbounded) moved against the frozen baseline."),
        "expected clobber_unbounded adjudication:\n{added_err}"
    );
    assert!(
        added_err.contains("BlindProc_Extra (got 1, want 0)"),
        "expected BlindProc_Extra added row:\n{added_err}"
    );
    assert!(
        added_err.contains("[sr_level] moved against the frozen baseline."),
        "expected sr_level adjudication:\n{added_err}"
    );
    assert!(
        added_err.contains("[proc.preserves-unverifiable] moved against the frozen baseline."),
        "expected word_preserve_unverifiable adjudication:\n{added_err}"
    );
    assert!(
        added_err.contains("BlindProc_Extra :: preserves(d7.w) (got 1, want 0)"),
        "expected BlindProc_Extra word_preserve added row:\n{added_err}"
    );
    assert!(
        added_err.contains("engine/ratchet.emp:7:"),
        "expected located path on BlindProc_Extra's own line (not BlindProc's line 2):\n{added_err}"
    );

    // 4. Stale baseline rows fail in the GONE direction.
    std::fs::write(tmp.path().join("engine/ratchet.emp"), "module engine.ratchet\n").unwrap();
    let stale = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!stale.status.success(), "stale baseline rows must fail");
    let stale_err = String::from_utf8_lossy(&stale.stderr);
    assert!(
        stale_err.contains("BlindProc (got 0, want 1)"),
        "expected GONE row for BlindProc:\n{stale_err}"
    );
    assert!(
        stale_err.contains("BlindProc :: preserves(d7.w) (got 0, want 1)"),
        "expected GONE row for BlindProc word_preserve:\n{stale_err}"
    );
}

#[test]
fn format1_description_is_refused_with_version_error() {
    let tmp = tempfile::tempdir().unwrap();
    sigil_harness::test_support::write_project_fixture(tmp.path(), "g");
    std::fs::write(tmp.path().join("engine/baseline.toml"), "format = 1\n").unwrap();

    let out_bin = tmp.path().join("out.bin");
    let out = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!out.status.success(), "format = 1 baseline must be refused");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("format = 1, this reader accepts format = 2"),
        "expected format mismatch message:\n{stderr}"
    );
}
