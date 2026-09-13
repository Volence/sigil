//! `sigil build`'s contract closure gate refuses a §6 flag-result firing:
//! `[call.flag-result-unused]` (a flag result abandoned on some path) and
//! `[call.result-invalid-path]` (a conditional register result read where its
//! guard says it is invalid), on both CPUs, each refusal with its location.
//!
//! The real binary over a self-contained fixture tree: one 68k module, one Z80
//! module and an empty game map are all the gate needs, because it runs before
//! anything else of the build. No reference tree is read, so this never skips.
//!
//! THE EXIT CODE CARRIES NO SIGNAL HERE, by construction. The gate's frozen
//! baselines are sized to aeon's corpus, and over a tree this small they report
//! every pinned row GONE and fail the gate too. So every assertion reads the
//! refusal TEXT, and two controls stand beside the firing tree: the same calls
//! written correctly reach the gate's verdict without a flag refusal, and
//! `SIGIL_CONTRACTS=0` skips the gate before any refusal.

use std::path::Path;
use std::process::Command;

/// A 68k caller abandoning `Queue`'s carry, a 68k caller reading `Alloc`'s
/// conditional `a1` on its invalid (carry-set) edge, and a 68k caller whose
/// `btst` redefines `Find`'s zero result before its `beq` (BTST writes Z and
/// leaves C, so only the zero model sees this one).
const M68K_FIRING: &str = "module engine.flagfix
extern proc Queue (d1) clobbers(d0) out(carry: dropped)
extern proc Alloc () clobbers(d0) out(a1 if cc)
extern proc Find () clobbers(d0) out(zero: found)
proc Caller () clobbers(d0-d1) {
    moveq #0, d1
    jbsr Queue
    moveq #0, d0
    rts
}
proc Reader () clobbers(d0-d1/a1) {
    jbsr Alloc
    bcs .fail
    move.w (a1), d0
    rts
.fail:
    move.w (a1), d1
    rts
}
proc ZCaller () clobbers(d0) {
    jbsr Find
    btst #0, d0
    beq .x
.x:
    rts
}
";

/// A Z80 caller abandoning `Resolve`'s carry (`scf` redefines it before `ret`),
/// and a Z80 caller whose `inc a` redefines `Lookup`'s zero result before its
/// `jr z` (8-bit INC writes Z and leaves C).
const Z80_FIRING: &str = "module engine.sndfix (cpu: z80)
extern proc Resolve () out(carry: missing)
extern proc Lookup () out(zero: absent)
proc SndCaller () {
    call Resolve
    scf
    ret
}
proc ZSndCaller () {
    call Lookup
    inc a
    jr z, .done
.done:
    ret
}
";

/// The same five calls written correctly: `@discards` on the drop, `a1` read
/// only on the valid edge, the Z80 carry consumed by `jr c`, and each zero
/// result read by its branch before the instruction that would redefine it.
const M68K_CLEAN: &str = "module engine.flagfix
extern proc Queue (d1) clobbers(d0) out(carry: dropped)
extern proc Alloc () clobbers(d0) out(a1 if cc)
extern proc Find () clobbers(d0) out(zero: found)
proc Caller () clobbers(d0-d1) {
    moveq #0, d1
    jbsr Queue @discards(dropped)
    moveq #0, d0
    rts
}
proc Reader () clobbers(d0-d1/a1) {
    jbsr Alloc
    bcs .fail
    move.w (a1), d0
.fail:
    rts
}
proc ZCaller () clobbers(d0) {
    jbsr Find
    beq .x
    btst #0, d0
.x:
    rts
}
";

const Z80_CLEAN: &str = "module engine.sndfix (cpu: z80)
extern proc Resolve () out(carry: missing)
extern proc Lookup () out(zero: absent)
proc SndCaller () {
    call Resolve
    jr c, .done
.done:
    ret
}
proc ZSndCaller () {
    call Lookup
    jr z, .done
    inc a
.done:
    ret
}
";

/// Write the fixture tree: the two modules and the empty game map the shape's
/// define merge requires.
fn tree(root: &Path, m68k: &str, z80: &str) {
    std::fs::create_dir_all(root.join("engine")).unwrap();
    std::fs::create_dir_all(root.join("games/sonic4")).unwrap();
    std::fs::write(root.join("engine/flagfix.emp"), m68k).unwrap();
    std::fs::write(root.join("engine/sndfix.emp"), z80).unwrap();
    std::fs::write(root.join("games/sonic4/map.toml"), "").unwrap();
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

/// Every flag-check kind the build walk produces, on both CPUs, stops the build
/// with the family header and one located line per firing, and each line says
/// what to do: the must-use lines name the declared result for `@discards`.
#[test]
fn the_build_gate_refuses_every_flag_result_firing_with_its_location() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), M68K_FIRING, Z80_FIRING);
    let err = build_stderr(tmp.path(), false);

    let m68k = tmp.path().join("engine/flagfix.emp");
    let z80 = tmp.path().join("engine/sndfix.emp");
    let expected = [
        format!(
            "  {}: [call.flag-result-unused] `Caller` calls `Queue` and abandons its `carry` \
             result `dropped` on some path",
            loc(&m68k, M68K_FIRING, "jbsr Queue")
        ),
        format!(
            "  {}: [call.result-invalid-path] `Reader` calls `Alloc`, whose `a1` result is \
             valid only where `cc` holds",
            loc(&m68k, M68K_FIRING, "jbsr Alloc")
        ),
        format!(
            "  {}: [call.flag-result-unused] `SndCaller` calls `Resolve` and abandons its \
             `carry` result `missing` on some path",
            loc(&z80, Z80_FIRING, "call Resolve")
        ),
        format!(
            "  {}: [call.flag-result-unused] `ZCaller` calls `Find` and abandons its `zero` \
             result `found` on some path",
            loc(&m68k, M68K_FIRING, "jbsr Find")
        ),
        format!(
            "  {}: [call.flag-result-unused] `ZSndCaller` calls `Lookup` and abandons its \
             `zero` result `absent` on some path",
            loc(&z80, Z80_FIRING, "call Lookup")
        ),
    ];
    assert!(
        err.contains(&format!(
            "error: [call.flag-result-unused]/[call.result-invalid-path], {} firing(s); \
             zero-firing by contract:",
            expected.len()
        )),
        "the gate must refuse the flag family with its count, got:\n{err}"
    );
    for line in &expected {
        assert!(err.contains(line.as_str()), "missing refusal line\n  {line}\ngot:\n{err}");
    }
    for name in ["dropped", "missing", "found", "absent"] {
        assert!(
            err.contains(&format!("or mark the call `@discards({name})` if dropping it is intended")),
            "the must-use refusal must name `@discards({name})`, got:\n{err}"
        );
    }
    assert!(
        err.contains("Read `a1` only on the `cc` path, or redefine it first"),
        "the invalid-path refusal must say what to do, got:\n{err}"
    );
}

/// CONTROL: the same calls written correctly reach the gate's verdict (the
/// baseline rows fail it, as over any tree this small) with no flag refusal. A
/// gate that refused every flag-result call, or matched on the callee alone,
/// would print the family here.
#[test]
fn correct_flag_result_calls_draw_no_flag_refusal() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), M68K_CLEAN, Z80_CLEAN);
    let err = build_stderr(tmp.path(), false);
    assert!(
        err.contains("error: contract closure gate FAILED."),
        "the control must reach the gate's verdict, or it shows nothing: {err}"
    );
    for rule in ["[call.flag-result-unused]", "[call.result-invalid-path]"] {
        assert!(!err.contains(rule), "no flag refusal expected, found {rule} in:\n{err}");
    }
}

/// CONTROL: `SIGIL_CONTRACTS=0` is still the emergency opt-out. It skips the gate,
/// saying so, before the analysis, so the firing tree draws no flag refusal.
#[test]
fn sigil_contracts_0_still_skips_the_flag_family() {
    let tmp = tempfile::tempdir().unwrap();
    tree(tmp.path(), M68K_FIRING, Z80_FIRING);
    let err = build_stderr(tmp.path(), true);
    assert!(
        err.contains("warning: contract closure gate SKIPPED (SIGIL_CONTRACTS=0)."),
        "the opt-out must say it skipped, got:\n{err}"
    );
    for rule in ["[call.flag-result-unused]", "[call.result-invalid-path]"] {
        assert!(!err.contains(rule), "a skipped gate refused {rule}:\n{err}");
    }
}
