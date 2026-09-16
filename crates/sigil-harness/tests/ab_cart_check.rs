//! `golden/ab/cart_check.py` actually refuses, and `cargo test` is what executes that.
//!
//! The A/B instruments under `golden/ab/` are hand-run: no suite invokes them, so nothing
//! automated could ever go green or red on them. That is exactly why the cart check had to
//! become an instrument rather than a paragraph in `AB_PROTOCOL.md`, and it is why the
//! instrument's own refusal branches are gated from here. A check whose failing branch is
//! never executed is a check nobody has seen work, and a hand-run one has no CI and no
//! second reader to notice.
//!
//! The house pattern for gating a non-Rust concern from Rust is `boot_read_bound.rs`,
//! including its positive control, and this file follows it.
//!
//! **WHAT THESE CASES CAN AND CANNOT ESTABLISH.** They drive `verify_cart` against a FAKE
//! bus: scripted `status`, `memory_hash` and `read_memory` answers, no emulator, no socket,
//! no `oracle-aether` process. They establish that every refusal branch refuses and that the
//! passing branches pass. They do NOT establish that a real oracle server answers these
//! method spellings with these keys, nor that its `region` really reads `"cartridge ROM"`
//! for a flat image. That half is a live confirmation and is TAGGED for a foreground run,
//! not claimed here.
//!
//! **The expectations are not written down twice.** Each case's declared verdict lives in
//! the Python `CASES` registry and the runner compares against it; this file asserts the
//! runner's exit status. `the_rust_gate_runs_every_case_the_module_declares` derives the
//! population from `--list-cases`, so a case added in Python that no Rust test runs is a
//! red here rather than a silent hole.

use std::path::PathBuf;
use std::process::Command;

fn cart_check_py() -> PathBuf {
    sigil_harness::reference_dependence::workspace_root()
        .join("crates/sigil-harness/golden/ab/cart_check.py")
}

/// `(success, combined output)`. A `python3` that will not start is a hard failure, never a
/// skip: an unmeasurable gate reported as green is the defect this whole parcel is closing.
fn run(args: &[&str]) -> (bool, String) {
    let py = cart_check_py();
    assert!(py.is_file(), "the instrument is missing at {}", py.display());
    let out = Command::new("python3")
        .arg(&py)
        .args(args)
        // `-B` keeps a stale `__pycache__` from being what a run actually executes, which
        // would make a mutation look applied and green at the same time.
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .unwrap_or_else(|e| panic!("cannot run python3 {}: {e}", py.display()));
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// The case names the module declares, read from the module rather than copied here.
fn declared_cases() -> Vec<String> {
    let (ok, text) = run(&["--list-cases"]);
    assert!(ok, "--list-cases failed:\n{text}");
    let names: Vec<String> = text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    assert!(!names.is_empty(), "the module declares no cases, so this gate measures nothing");
    names
}

/// One case, by name. The case's own expectation (`pass` or `refuse`) lives in the module.
fn case(name: &str) {
    let (ok, text) = run(&["--self-test", name]);
    assert!(ok, "cart_check case `{name}` did not behave as the module declares:\n{text}");
    assert!(
        text.contains("0 MISMATCH"),
        "cart_check case `{name}` exited 0 without a clean summary line:\n{text}"
    );
}

// --- the passing branches --------------------------------------------------------------

#[test]
fn exact_match_passes() {
    case("exact_match_passes");
}

/// A server that does not serve `memory_hash` falls back to the documented byte sample.
#[test]
fn sampled_fallback_passes() {
    case("sampled_fallback_passes");
}

/// A caveat about something else is reported, not refused: §2.4 allows them, and a check
/// that reddened on every caveat is one a runner learns to wave through.
#[test]
fn unrelated_caveat_passes() {
    case("unrelated_caveat_passes");
}

/// **The coverage limit, held true rather than asserted in prose.** A one-byte edit placed
/// where no sample window looks PASSES the sampled route. That is not a defect in the check,
/// it is what a sample is, and this case is why a sampled green prints SAMPLE, NOT
/// WHOLE-IMAGE and why `CART_CHECK_REQUIRE_TOTAL=1` exists. The case below proves the total
/// route catches the same byte.
#[test]
fn one_byte_outside_every_window_escapes_the_sample() {
    case("one_byte_outside_every_window_escapes_the_sample");
}

// --- the refusal branches --------------------------------------------------------------

/// The case the server's own caveat is structurally blind to: a different file that has not
/// changed on disk. Same bytes at the other path, so only the path row can fire.
#[test]
fn wrong_rom_path_refuses() {
    case("wrong_rom_path_refuses");
}

#[test]
fn missing_rom_path_refuses() {
    case("missing_rom_path_refuses");
}

/// The row contract item 27 calls out as REQUIRED: the sizes match and the bytes differ.
#[test]
fn same_size_different_bytes_refuses() {
    case("same_size_different_bytes_refuses");
}

/// The same required row down the sampled route, with the edit inside a window.
#[test]
fn same_size_different_bytes_refuses_sampled() {
    case("same_size_different_bytes_refuses_sampled");
}

/// The filed incident's shape: the difference lives at the END of the image (166 bytes).
#[test]
fn tail_bytes_differ_refuses() {
    case("tail_bytes_differ_refuses");
}

#[test]
fn one_byte_outside_every_window_is_caught_by_the_total_hash() {
    case("one_byte_outside_every_window_is_caught_by_the_total_hash");
}

#[test]
fn stale_caveat_refuses() {
    case("stale_caveat_refuses");
}

#[test]
fn could_not_check_caveat_refuses() {
    case("could_not_check_caveat_refuses");
}

#[test]
fn missing_rom_bytes_refuses() {
    case("missing_rom_bytes_refuses");
}

#[test]
fn wrong_rom_bytes_refuses() {
    case("wrong_rom_bytes_refuses");
}

#[test]
fn read_memory_error_refuses() {
    case("read_memory_error_refuses");
}

#[test]
fn short_read_refuses() {
    case("short_read_refuses");
}

/// `"cartridge ROM bank 9"` begins with the same two words as the flat spelling, and under
/// it the bus address is NOT an image offset (contract M1, exact equality).
#[test]
fn banked_region_refuses() {
    case("banked_region_refuses");
}

#[test]
fn status_bus_error_refuses() {
    case("status_bus_error_refuses");
}

#[test]
fn unreadable_file_refuses() {
    case("unreadable_file_refuses");
}

/// Two REAL builds of the same engine, same byte count, different bytes. Corroboration on
/// real data, not the gate: the fixture lives in another lane's worktree, so the case skips
/// LOUDLY when it is gone rather than going red when somebody prunes a worktree.
#[test]
fn real_build_pair_refuses() {
    case("real_build_pair_refuses");
}

// --- the population, and the control ----------------------------------------------------

/// Every case the module declares is run by a test in this file, derived from `--list-cases`
/// rather than copied. Without it, a case added in Python would sit unexecuted and this
/// file would still be green.
#[test]
fn the_rust_gate_runs_every_case_the_module_declares() {
    let declared = declared_cases();
    let here = std::fs::read_to_string(
        sigil_harness::reference_dependence::workspace_root()
            .join("crates/sigil-harness/tests/ab_cart_check.rs"),
    )
    .expect("this test file must be readable to enumerate what it runs");
    let missing: Vec<&String> = declared
        .iter()
        .filter(|n| !here.contains(&format!("case(\"{n}\")")))
        .collect();
    assert!(
        missing.is_empty(),
        "cart_check.py declares cases no test in this file runs: {missing:?}\n\
         Add a `#[test]` calling `case(\"<name>\")` for each, so the case is executed and \
         named in the runner's output rather than only counted inside python."
    );
    println!("cart_check.py declares {} cases, all run here", declared.len());
}

/// Every declared case, in one run, reported as aggregate totals rather than a tail.
#[test]
fn the_whole_case_set_behaves_as_declared() {
    let (ok, text) = run(&["--self-test"]);
    assert!(ok, "the cart_check case set did not behave as declared:\n{text}");
    let summary = text
        .lines()
        .find(|l| l.starts_with("SELFTEST "))
        .unwrap_or_else(|| panic!("no SELFTEST summary line, so nothing was measured:\n{text}"))
        .to_string();
    assert!(summary.ends_with("0 MISMATCH"), "{summary}\n{text}");
    println!("{summary}");
}

/// **The positive control.** A `--self-test` that reported everything as expected no matter
/// what the module did would look identical to one that works, so the module declares
/// control cases whose stated expectation is deliberately wrong. Each MUST exit non-zero and
/// say MISMATCH. This is the only thing in the file that proves the failing path is live.
#[test]
fn the_self_test_runner_actually_goes_red() {
    let (ok, text) = run(&["--list-controls"]);
    assert!(ok, "--list-controls failed:\n{text}");
    let controls: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    assert!(
        !controls.is_empty(),
        "no control case, so nothing here proves this runner can go red"
    );
    for name in &controls {
        let (ok, text) = run(&["--self-test", name]);
        assert!(!ok, "control `{name}` exited 0; the runner cannot go red:\n{text}");
        assert!(
            text.contains("MISMATCH"),
            "control `{name}` failed without naming the mismatch:\n{text}"
        );
        println!("control {name}: went red as it must");
    }
}
