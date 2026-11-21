//! `sigil build`'s contract closure gate refuses the three check families that
//! used to be computed and PRINTED (by `--report contracts`) but never stopped a
//! build: `[bus.*]` (Z80-bus machine state), `[branch.condition-constant]`, and
//! `[call.slot-type-mismatch]`/`[option.unguarded-use]`. Each refusal carries the
//! family header with its count and one located line per firing.
//!
//! The real binary over a self-contained fixture tree: one 68k module and an
//! empty game map are all the gate needs, because it runs before anything else
//! of the build. No reference tree is read, so this never skips.
//!
//! The fixture is a described project with empty contract baselines (the same harness as
//! `flag_result_gate.rs`, G0b Task 11), so the gate's verdict is the fixture's alone, and
//! two controls stand beside the firing tree: the same procs written cleanly PASS the
//! gate without any of the three headers, and `SIGIL_CONTRACTS=0` skips the
//! gate before any refusal.

use std::path::Path;
use std::process::Command;

/// `BusP`: a VDP write after an explicit bus release (the `[bus.vdp-write-unstopped]`
/// fixture of `sigil-frontend-emp/tests/z80_bus.rs`). `BranchP`: a constant-immediate
/// write between the `tst` and the `bne`, so the branch reads a forced `Z`
/// (`branch_const.rs`). `SwapP`: `TakeX` wants a `GridX` in `d2` and is handed a
/// `GridY` (`type_slice.rs`). The two port names are declared as the bus analysis
/// spells them.
const FIRING: &str = "module engine.printfix
pub const Z80_BUS_REQUEST = $A11100
pub const VDP_CTRL = $C00004
pub newtype GridX = u8
pub newtype GridY = u8
pub proc TakeX (d2: GridX) clobbers() { rts }
pub proc BusP () clobbers(d0) {
    move.w  #$0100, Z80_BUS_REQUEST
    move.w  #$0000, Z80_BUS_REQUEST
    move.w  d0, VDP_CTRL
    rts
}
pub proc BranchP () clobbers(d0/d1) {
.spin:
    tst.b   d0
    move.w  #$0000, d1
    bne     .spin
    rts
}
pub proc SwapP () clobbers(d0-d5) {
    move.l  d4, d2 as GridY
    jbsr    TakeX
    rts
}
";

/// The same three procs written cleanly: the VDP write inside a stop/start pair,
/// the `bne` reading the `tst`, and `TakeX` handed a `GridX`.
const CLEAN: &str = "module engine.printfix
pub const Z80_BUS_REQUEST = $A11100
pub const VDP_CTRL = $C00004
pub newtype GridX = u8
pub newtype GridY = u8
pub proc TakeX (d2: GridX) clobbers() { rts }
pub proc BusP () clobbers(d0) {
    move.w  #$0100, Z80_BUS_REQUEST
    move.w  d0, VDP_CTRL
    move.w  #$0000, Z80_BUS_REQUEST
    rts
}
pub proc BranchP () clobbers(d0/d1) {
.spin:
    move.w  #$0000, d1
    tst.b   d0
    bne     .spin
    rts
}
pub proc SwapP () clobbers(d0-d5) {
    move.l  d4, d2 as GridX
    jbsr    TakeX
    rts
}
";

const HEADERS: [&str; 3] = [
    "error: [bus.*] Z80-bus machine-state firings",
    "error: [branch.condition-constant] firings",
    "error: [call.slot-type-mismatch]/[option.unguarded-use] firings",
];

/// Write the fixture tree: the module and the empty game map the shape's define
/// merge requires.
fn tree(root: &Path, src: &str) {
    sigil_harness::test_support::write_project_fixture(root, "g");
    std::fs::write(root.join("engine/printfix.emp"), src).unwrap();
}

/// `sigil build --project <root>/games/g --shape release` over the fixture, returning stderr.
fn build_stderr(root: &Path, contracts_off: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--project"]).arg(root.join("games/g")).args(["--shape", "release", "-o"]).env_remove("AEON_DIR");
    cmd.arg(root.join("out.bin"));
    if contracts_off {
        cmd.env("SIGIL_CONTRACTS", "0");
    } else {
        cmd.env_remove("SIGIL_CONTRACTS");
    }
    let out = cmd.output().expect("run sigil build");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// `file:line:col` of the first line of `src` containing `needle`, the way the
/// gate renders a span: 1-based line, 1-based column of the instruction.
fn loc(file: &Path, src: &str, needle: &str) -> String {
    let (i, line) = src
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(needle))
        .unwrap_or_else(|| panic!("fixture has no line containing {needle:?}"));
    let col = line.len() - line.trim_start().len() + 1;
    format!("{}:{}:{col}", file.display(), i + 1)
}

/// Each of the three formerly print-only families stops the build with its
/// header, its count, and one located line per firing.
#[test]
fn the_build_gate_refuses_each_printed_family_with_its_location() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), FIRING);
    let err = build_stderr(tmp.path(), false);
    let f = tmp.path().join("engine/printfix.emp");

    assert!(
        err.contains(
            "error: [bus.*] Z80-bus machine-state firings, 1 firing(s); this family is \
             zero-firing by contract:"
        ),
        "the gate must refuse the bus family with its count, got:\n{err}"
    );
    let bus = format!(
        "  {}: [bus.vdp-write-unstopped] in `BusP`",
        loc(&f, FIRING, "move.w  d0, VDP_CTRL")
    );
    assert!(err.contains(&bus), "missing refusal line\n  {bus}\ngot:\n{err}");

    assert!(
        err.contains(
            "error: [branch.condition-constant] firings, 1 firing(s); this family is \
             zero-firing by contract:"
        ),
        "the gate must refuse the branch family with its count, got:\n{err}"
    );
    let branch = format!(
        "  {}: [branch.condition-constant] `BranchP` bne is statically decided (NEVER taken)",
        loc(&f, FIRING, "bne     .spin")
    );
    assert!(err.contains(&branch), "missing refusal line\n  {branch}\ngot:\n{err}");

    assert!(
        err.contains(
            "error: [call.slot-type-mismatch]/[option.unguarded-use] firings, 1 firing(s); \
             this family is zero-firing by contract:"
        ),
        "the gate must refuse the slot family with its count, got:\n{err}"
    );
    let slot = format!(
        "  {}: [call.slot-type-mismatch] `SwapP` calls `TakeX`: slot d2 expects GridX but \
         found GridY",
        loc(&f, FIRING, "jbsr    TakeX")
    );
    assert!(err.contains(&slot), "missing refusal line\n  {slot}\ngot:\n{err}");
}

/// CONTROL: the same procs written cleanly pass the gate (the fixture's baselines are
/// empty) with none of the three headers. A
/// gate that refused unconditionally would print them here.
#[test]
fn clean_shapes_draw_no_printed_family_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), CLEAN);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains("contract closure gate: passed ("),
        "the control must pass the gate, or it shows nothing: {err}"
    );
    for h in HEADERS {
        assert!(!err.contains(h), "no {h:?} refusal expected in:\n{err}");
    }
}

/// CONTROL: `SIGIL_CONTRACTS=0` is still the emergency opt-out. It skips the gate,
/// saying so, before the analysis, so the firing tree draws none of the headers.
#[test]
fn sigil_contracts_0_still_skips_the_printed_families() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), FIRING);
    let err = build_stderr(tmp.path(), true);
    assert!(
        err.contains("warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)."),
        "the opt-out must say it skipped, got:\n{err}"
    );
    for h in HEADERS {
        assert!(!err.contains(h), "no {h:?} refusal expected when skipped, in:\n{err}");
    }
}
