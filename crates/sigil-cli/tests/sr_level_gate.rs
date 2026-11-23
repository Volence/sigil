//! End-to-end CLI contract gate + `--report contracts` tests for whole-program
//! 68000 interrupt-level propagation and Rule 3 (`with <ctx> { }` level hold,
//! step A piece 2b Task 4).

use std::path::Path;
use std::process::Command;

const INTS_OFF: &str = "context ints_off {
    acquire = asm { move.w sr, -(sp)
                    ori.w #$0700, sr }
    release = asm { move.w (sp)+, sr }
}
";

fn write_two_file_tree(root: &Path, caller_src: &str, callee_src: &str) {
    sigil_harness::test_support::write_project_fixture(root, "g");
    std::fs::write(root.join("engine/caller.emp"), caller_src).unwrap();
    std::fs::write(root.join("engine/callee.emp"), callee_src).unwrap();
}

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
fn cross_file_sr_mask_and_rule3_violations_stop_sigil_build_with_location() {
    let tmp = tempfile::tempdir().unwrap();
    let caller = format!(
        "module engine.caller\n{INTS_OFF}\
         pub proc Preserver () clobbers(d0) preserves(sr.mask) {{
             jbsr    Masker
             rts
         }}
         pub proc Bracketed () clobbers(d0) {{
             with ints_off {{
                 jbsr    Unmasker
             }}
             rts
         }}
         "
    );
    let callee = "module engine.callee
pub proc Masker () clobbers(sr) {
    move.w  #$2700, sr
    rts
}
pub proc Unmasker () clobbers(sr) {
    move.w  #$2000, sr
    rts
}
";
    write_two_file_tree(tmp.path(), &caller, callee);

    let out_bin = tmp.path().join("out.bin");
    let out = run_sigil(tmp.path(), &["-o", out_bin.to_str().unwrap()]);
    assert!(!out.status.success(), "expected contract gate failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error: [proc.preserves-sr-unbalanced]/[proc.with-sr-level-broken] firings, 2 firing(s)"),
        "expected sr_level gate header with 2 firings:\n{stderr}"
    );
    assert!(
        stderr.contains("[proc.preserves-sr-unbalanced] `Preserver`"),
        "expected Preserver firing:\n{stderr}"
    );
    assert!(
        stderr.contains("[proc.with-sr-level-broken] `Bracketed`"),
        "expected Bracketed firing naming Unmasker:\n{stderr}"
    );
    assert!(
        stderr.contains("engine/caller.emp:"),
        "expected located path in caller.emp:\n{stderr}"
    );
}

#[test]
fn clean_cross_file_sr_and_continuation_pass_gate_and_appear_in_contracts_report() {
    let tmp = tempfile::tempdir().unwrap();
    let caller = format!(
        "module engine.caller\n{INTS_OFF}\
         pub proc Preserver () clobbers(d0) preserves(sr.mask) {{
             with ints_off {{
                 jbsr    SafeCritical
             }}
             jbsr    Resume
             rts
         }}
         "
    );
    let callee = "module engine.callee
pub proc SafeCritical () clobbers(d0) preserves(sr.mask) {
    move.w  sr, -(sp)
    ori.w   #$0700, sr
    moveq   #1, d0
    move.w  (sp)+, sr
    rts
}
@continuation
pub proc Resume () clobbers(d0) {
    moveq   #0, d0
    rte
}
";
    write_two_file_tree(tmp.path(), &caller, callee);

    let rep = run_sigil(tmp.path(), &["--report", "contracts"]);
    assert!(
        rep.status.success(),
        "expected --report contracts success:\n{}",
        String::from_utf8_lossy(&rep.stderr)
    );
    let stdout = String::from_utf8_lossy(&rep.stdout);
    assert!(
        stdout.contains("-- [sr_level] interrupt-level firings (0), trusted @continuation (1): --"),
        "expected sr_level summary line in report:\n{stdout}"
    );
    assert!(
        stdout.contains("Resume"),
        "expected Resume listed under trusted @continuation:\n{stdout}"
    );
}
