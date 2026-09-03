//! An assembly unit that never declares its processor is REFUSED, and the
//! refusal is scoped to the UNIT rather than to the file.
//!
//! ## What was silently wrong
//!
//! The front-end's `Options::initial_cpu` defaulted to `Cpu::Z80` — honest for
//! the Z80-only M0 build it was written for, and silently wrong afterwards. The
//! processor decides how the lexer reads `$`: as a hex prefix on 68000, as the
//! program counter on Z80. So a 68000 source with no `cpu` line did not fail to
//! target 68000 — it targeted Z80, and said nothing about it. The community
//! Sonic 2 disassembly had been assembling as a Z80 program on this path;
//! case-folding the `CPU` directive rescued that corpus only because it happens
//! to carry a `cpu` line at all.
//!
//! ## Why it refuses instead of warning
//!
//! A run that reports what it skipped still exits 0. A silent green is the
//! failure class this suite does not drop, and "the assembler picked a
//! processor for you" has no honest warning form: the bytes are already wrong
//! by the time anyone reads it.
//!
//! ## Why the scope is the unit, not the file
//!
//! This is the half whose absence would break a shipping build. Aeon's
//! `engine/debug/debugger.asm` carries no `cpu` directive and needs none: both
//! `games/*/game_root.asm` declare `cpu 68000` at line 15 and `include` it
//! afterwards, and `build.sh` assembles exactly one root per invocation. A
//! per-FILE refusal would fire on that include and break every aeon shape. The
//! declaration belongs to the unit, and `AsmState` is one state threaded
//! through the root and every file it splices — so a `cpu` line anywhere in the
//! unit satisfies it.
//!
//! ## Why these gates drive the CLI process
//!
//! `Options::initial_cpu` was always reachable, and every in-tree caller that
//! assembles a real program sets it. The undeclared caller is the shipped
//! `sigil <file.asm>` command, which takes `Options::default()` — a
//! library-level test on the option would have passed throughout while the
//! command silently mis-targeted. Two of the four below therefore assert on the
//! PROCESS. The remaining two pin the library contract the CLI rests on.

use sigil_frontend_as::{assemble, Options, CPU_UNDECLARED};
use sigil_ir::Cpu;
use std::process::Command;

/// The refusal's one distinguishing sentence, and the line it tells you to
/// write. Derived from the shipped constant rather than transcribed, so a
/// reworded message cannot leave this gate asserting on prose that no longer
/// exists.
fn refusal_head() -> &'static str {
    CPU_UNDECLARED
        .split_once(',')
        .expect("the refusal names what was not declared before its first comma")
        .0
}

/// A root with no `cpu` line is refused, naming what was not declared and
/// printing the exact line to write.
#[test]
fn an_undeclared_unit_is_refused_by_the_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("root.asm");
    std::fs::write(&root, "\tmove.w #$1234,d0\n").expect("write root.asm");

    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([root.to_str().unwrap(), "-o", dir.path().join("out.bin").to_str().unwrap()])
        .output()
        .expect("spawn sigil");

    assert!(
        !out.status.success(),
        "an undeclared unit must FAIL, not warn — a run that reports what it \
         skipped still exits 0. status: {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(refusal_head()),
        "the refusal must name what was not declared. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("`cpu 68000`") && stderr.contains("`cpu z80`"),
        "the refusal must print the exact line to write, for both processors. \
         stderr:\n{stderr}"
    );
    assert!(
        !dir.path().join("out.bin").exists(),
        "a refused unit must produce no output binary"
    );
}

/// THE GUARD ON THE HAZARD. A file with no `cpu` directive, included beneath a
/// root that has already declared, must NOT trip the refusal — this is aeon's
/// `debugger.asm` shape, and its absence is how this parcel breaks the shipping
/// build.
#[test]
fn an_included_file_beneath_a_declaring_root_does_not_trip_the_refusal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let part = dir.path().join("part.asm");
    let root = dir.path().join("root.asm");

    // The included file's own text carries NO processor line — the precondition
    // that makes this test about the hazard rather than about nothing.
    let part_src = "\tdc.w $1234\n\tmove.w #$5678,d0\n";
    assert!(
        !part_src.to_ascii_lowercase().contains("cpu"),
        "precondition: the included file declares no processor of its own"
    );
    std::fs::write(&part, part_src).expect("write part.asm");
    std::fs::write(&root, "\tcpu 68000\n\tinclude \"part.asm\"\n").expect("write root.asm");

    let out_bin = dir.path().join("out.bin");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([root.to_str().unwrap(), "-o", out_bin.to_str().unwrap()])
        .output()
        .expect("spawn sigil");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains(refusal_head()),
        "the refusal fired on an included file beneath a declaring root — the \
         scope has slipped from the unit to the file, which breaks every aeon \
         shape. stderr:\n{stderr}"
    );
    assert!(
        out.status.success(),
        "the unit is complete and declared; it must assemble. stderr:\n{stderr}"
    );
    assert!(out_bin.exists(), "the declared unit must produce its binary");
}

/// A CALLER that sets `initial_cpu` HAS declared: the source needs no directive
/// of its own. This is the sound stack's and the harness's shape.
#[test]
fn a_caller_setting_initial_cpu_has_declared() {
    let src = "\tmove.w #$1234,d0\n";
    assert!(
        !src.to_ascii_lowercase().contains("cpu"),
        "precondition: the source declares nothing; only the caller does"
    );
    let opts = Options { initial_cpu: Some(Cpu::M68000), ..Options::default() };
    let module = assemble(src, &opts).expect("a caller-declared unit assembles");
    assert!(
        module.sections.iter().any(|s| !s.fragments.is_empty()),
        "the caller-declared unit must actually have emitted its bytes"
    );

    // And the same source with NOTHING declared is the refusal — the control
    // that makes the line above mean the caller's declaration, not the source.
    let diags = assemble(src, &Options::default())
        .expect_err("nothing declared it: this must be refused");
    assert!(
        diags.iter().any(|d| d.message == CPU_UNDECLARED),
        "expected the undeclared-processor refusal, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// A unit that emits no bytes is still refused. It has no wrong bytes to show
/// for it, but every `$` in it was still lexed against a processor nobody named,
/// and its `equ` values carry that decision out to whoever consumes them.
#[test]
fn a_byteless_undeclared_unit_is_still_refused() {
    let diags = assemble("FOO equ 16\n", &Options::default())
        .expect_err("an undeclared unit is refused whether or not it emits");
    assert_eq!(
        diags.iter().filter(|d| d.message == CPU_UNDECLARED).count(),
        1,
        "the refusal is a property of the unit and is stated exactly once: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// The refusal is reported FIRST, ahead of the diagnostics it explains.
///
/// `FOO equ $10` is the shape that makes this testable: with no processor
/// declared the unit runs on the provisional one, where `$` lexes as the
/// program counter rather than a hex prefix, so the line mis-parses and raises
/// its own diagnostic before the end of the unit is reached. That mis-parse is
/// a SYMPTOM of the missing declaration, and a reader who meets it first is
/// being handed the consequence instead of the cause — which is exactly how
/// this class of failure stayed invisible.
#[test]
fn the_refusal_is_reported_before_the_diagnostics_it_explains() {
    let diags = sigil_frontend_as::assemble("FOO equ $10\n", &Options::default())
        .expect_err("nothing declared it: this must be refused");

    // The precondition that keeps the ordering assertion from being vacuous: a
    // lone diagnostic is trivially first. If the cascade ever stops happening,
    // this fails loudly rather than passing for the wrong reason.
    assert!(
        diags.len() >= 2,
        "this gate needs a diagnostic for the refusal to be ordered AHEAD of; \
         got only: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert_eq!(
        diags.first().map(|d| d.message.as_str()),
        Some(CPU_UNDECLARED),
        "the refusal must be reported FIRST — every other diagnostic an \
         undeclared unit produces is a consequence of it. Got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
