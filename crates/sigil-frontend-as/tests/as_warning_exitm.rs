//! The half of `warning`/`exitm` that byte vectors cannot hold: the DIAGNOSTICS.
//!
//! `tests/asl_snippets.rs` gates every shape whose evidence is an image, from
//! asl-minted goldens. It is silent about three things that are the whole point
//! of these two directives:
//!
//!   - a `warning` that fires and is then DROPPED emits identical bytes to one
//!     that never fired, so only an assertion over the returned warn tier can
//!     tell them apart;
//!   - a source asl REFUSES produces no image to compare at all;
//!   - a warning that gets promoted to an error would fail the run — again
//!     invisible to a gate that only reads bytes on the success path.
//!
//! Expectations are derived from asl 1.42 Beta [Bld 212], probes committed under
//! `docs/superpowers/notes/2026-09-04-as-warning-exitm-probes/` with each cell's
//! exit code and verbatim message. Where sigil's wording differs from asl's, the
//! test asserts SIGIL's wording and the probe note records asl's — the two
//! assemblers are not required to phrase a refusal the same way, only to refuse
//! the same programs.

use sigil_frontend_as::{assemble, Options};
use sigil_span::{Diagnostic, Level};

/// Assemble, expecting SUCCESS, and hand back the warn-tier diagnostics.
///
/// Through the FILE entry point, which is the one that carries warnings out and
/// the one a real user reaches: the string form (`assemble`) returns the module
/// alone, so a warning is invisible there by construction.
fn warnings(src: &str) -> Vec<Diagnostic> {
    let dir = tmpdir("warn");
    let path = dir.join("root.asm");
    std::fs::write(&path, src).expect("write probe");
    let a = sigil_frontend_as::assemble_root_located_warned(&path, &Options::default())
        .unwrap_or_else(|f| {
            panic!(
                "expected a SUCCESSFUL assembly, got {:?}",
                f.diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });
    std::fs::remove_dir_all(&dir).ok();
    a.warnings
}

/// Assemble, expecting REFUSAL, and hand back the diagnostic messages.
fn refusal(src: &str) -> Vec<String> {
    let diags = assemble(src, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled"));
    diags.into_iter().map(|d| d.message).collect()
}

/// A per-test scratch directory. Named with the clock and the thread id so two
/// tests running in parallel — cargo's default — cannot land on the same path
/// and delete each other's root mid-assembly. Same `temp_dir` the snippet vector
/// generator already uses for its asl scratch.
fn tmpdir(tag: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir()
        .join("sigil_as_warning_exitm")
        .join(format!("{tag}-{n}-{:?}", std::thread::current().id()));
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// A `warning` fires at the WARN tier and the assembly SUCCEEDS.
///
/// asl, probe `w1`: `> > > w1.asm(5): warning: hello from warning`, exit 0, the
/// summary reading `0 errors / 1 warning`, and the bytes either side untouched.
///
/// MUST FAIL if the directive is dropped on the floor (zero warnings), if it is
/// raised at the error tier (the assembly would refuse and `warnings` panics),
/// or if the author's text stops reaching the message.
#[test]
fn warning_fires_at_the_warn_tier_and_the_run_still_succeeds() {
    let w = warnings("\tcpu 68000\n\tpadding off\n\tphase 0\n\tdc.b $11\n\twarning \"hello from warning\"\n\tdc.b $22\n");
    assert_eq!(w.len(), 1, "exactly one warning, got {w:?}");
    assert_eq!(w[0].level, Level::Warning, "warn tier, not an error");
    assert!(
        w[0].message.contains("hello from warning"),
        "the author's own text must survive into the message, got {:?}",
        w[0].message
    );
}

/// The `[as.warning]` id prefix is what the build's warn-tier tally and
/// `sigil-cli/tests/warn_tier_corpus.rs`'s register key on: `BuildWarning` reads
/// the id out of a leading `[...]` group and a message without one is counted as
/// `unclassified`.
///
/// MUST FAIL if the prefix is dropped or renamed, which would make every
/// author-written warning unclassifiable in the tally.
#[test]
fn warning_carries_the_as_warning_lint_id() {
    let w = warnings("\tcpu 68000\n\tpadding off\n\tphase 0\n\twarning \"anything\"\n");
    assert!(
        w[0].message.starts_with("[as.warning] "),
        "expected the `[as.warning]` id prefix, got {:?}",
        w[0].message
    );
}

/// `\{expr}` interpolates in a `warning` exactly as it does in
/// `error`/`fatal`/`message` — `s2.sounddriver.asm(301)` and five of the six
/// s1disasm sites write one.
///
/// The VALUE asserted here is sigil's, and sigil's differs from asl's: asl folds
/// `\{expr}` to HEX (probe `x1`, `v equ 42` → `2A`) and sigil's `interp_text`
/// renders decimal. That divergence is older than this directive, is shared by
/// `error`/`fatal`/`message`, and reaches BYTES through `str_env` (probe `x2`) —
/// so it is recorded in the probe note and NOT changed here. This test pins what
/// sigil does today so the fix, when it comes, has to walk past a red line
/// rather than silently re-render every message.
#[test]
fn warning_interpolates_its_message() {
    let w = warnings("\tcpu 68000\n\tpadding off\n\tphase 0\nval equ 42\n\twarning \"val is \\{val} here\"\n");
    assert_eq!(w[0].message, "[as.warning] val is 42 here");
}

/// A `warning` in a FALSE conditional arm is not reached (asl, probe `w5`).
///
/// MUST FAIL if the directive is evaluated during block SCANNING rather than
/// execution — the failure mode that makes a guarded diagnostic fire on every
/// build.
#[test]
fn warning_in_a_false_arm_does_not_fire() {
    let w = warnings(
        "\tcpu 68000\n\tpadding off\n\tphase 0\n\tif 0\n\twarning \"must not fire\"\n\tendif\n\tdc.b $11\n",
    );
    assert!(w.is_empty(), "a warning under a false `if` must not fire, got {w:?}");
}

/// `warning` needs a quoted message. asl refuses all three malformed spellings:
/// a bare word (`w6`, which it tries to read as a SYMBOL), no operand (`w7`) and
/// two operands (`w8`), the latter two as `wrong number of operands`.
///
/// Sigil refuses the first two. The two-operand form is a KNOWN GAP — sigil takes
/// the first string and ignores the rest, exactly as its `error`/`fatal`/`message`
/// siblings already do — and the gap is asserted here rather than left to be
/// discovered, so closing it changes a test rather than surprising a reader.
#[test]
fn warning_without_a_quoted_message_is_refused() {
    for src in [
        "\tcpu 68000\n\tpadding off\n\tphase 0\n\twarning\n",
        "\tcpu 68000\n\tpadding off\n\tphase 0\n\twarning bareword\n",
    ] {
        let msgs = refusal(src);
        assert!(
            msgs.iter().any(|m| m.contains("`warning` needs a quoted message")),
            "expected the quoted-message refusal, got {msgs:?}"
        );
    }
    // The known gap, pinned: two operands are ACCEPTED and the first wins.
    let w = warnings("\tcpu 68000\n\tpadding off\n\tphase 0\n\twarning \"a\",\"b\"\n");
    assert_eq!(w[0].message, "[as.warning] a", "known gap vs asl's `wrong number of operands`");
}

/// `exitm` with no expansion around it is an error, and the run does not
/// silently continue as if it had exited something (asl, probe `e5`:
/// `error: EXITM not called from within macro`).
///
/// MUST FAIL if `exitm` at top level is accepted — which, with the flag set and
/// nobody to clear it, would silently truncate the REST OF THE FILE.
#[test]
fn exitm_outside_every_expansion_is_refused() {
    let msgs = refusal("\tcpu 68000\n\tpadding off\n\tphase 0\n\tdc.b $11\n\texitm\n\tdc.b $22\n");
    assert!(
        msgs.iter().any(|m| m.contains("`exitm` outside a macro expansion")),
        "expected the outside-an-expansion refusal, got {msgs:?}"
    );
}

/// `exitm` takes no operand: asl reports `wrong number of operands` AND does not
/// perform the exit (probe `e10` — the macro's trailing `dc.b $A1` still lands).
///
/// MUST FAIL if a malformed `exitm` exits anyway, which would end an expansion
/// early on a typo and emit a short, plausible-looking program.
#[test]
fn exitm_with_an_operand_is_refused() {
    let msgs = refusal(concat!(
        "\tcpu 68000\n\tpadding off\n\tphase 0\n",
        "m10 macro\n\tdc.b $A0\n\texitm 1,2,junk\n\tdc.b $A1\n\tendm\n",
        "\tm10\n",
    ));
    assert!(
        msgs.iter().any(|m| m.contains("`exitm` takes no operand")),
        "expected the no-operand refusal, got {msgs:?}"
    );
}

/// An `include` is not an expansion and it HIDES the ones around it: asl refuses
/// an `exitm` written in an included file even when the `include` line sits in a
/// macro body, and neither the include nor the macro stops (probe `e14`,
/// `e14inc.asm(2): error: EXITM not called from within macro`).
///
/// MUST FAIL if the expansion count is left standing across an `include` — the
/// program would then assemble, and an `exitm` in a shared header would silently
/// truncate whichever macro happened to include it.
#[test]
fn exitm_in_an_included_file_is_refused_even_inside_a_macro() {
    let dir = tmpdir("exitm-include");
    std::fs::write(dir.join("inc.asm"), "\tdc.b $E0\n\texitm\n\tdc.b $E1\n").expect("write include");
    let root = dir.join("root.asm");
    std::fs::write(
        &root,
        concat!(
            "\tcpu 68000\n\tpadding off\n\tphase 0\n",
            "m14 macro\n\tdc.b $A0\n\tinclude \"inc.asm\"\n\tdc.b $A1\n\tendm\n",
            "\tm14\n",
        ),
    )
    .expect("write root");
    let failure = sigil_frontend_as::assemble_root_located(&root, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("an `exitm` inside an included file must be refused"));
    let msgs: Vec<&String> = failure.diags.iter().map(|d| &d.message).collect();
    assert!(
        msgs.iter().any(|m| m.contains("`exitm` outside a macro expansion")),
        "expected the outside-an-expansion refusal from the INCLUDED file, got {msgs:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The line that CLOSES an exitm'd `if` arm is never read, so a label on it binds
/// nothing (asl, probe `e18`: `dc.l LC` after it is `symbol undefined`).
///
/// MUST FAIL if `exec_if` binds the closer's label unconditionally, which would
/// place a symbol at whatever PC the arm stopped on — a defined-but-wrong address
/// rather than a missing one, which is the harder failure to notice.
///
/// The refusal lands at LINK time, not in the front end: `dc.l LC` becomes a
/// symbolic fixup the assembler is happy to defer, and it is the linker that
/// finds nothing to resolve it against. So this drives the same
/// front-end→link composition `asl_snippets.rs` uses, and asserts on the LINKER's
/// diagnostic — a front-end-only assertion here would pass with the label bound
/// and never notice.
#[test]
fn exitm_leaves_the_closer_label_unbound() {
    let src = concat!(
        "\tcpu 68000\n\tpadding off\n\tphase 0\n",
        "\trept 2\n\tdc.b $C0\n\tif 1\n\texitm\nLC:\tendif\n\tendr\n",
        "\tdc.l LC\n",
    );
    let module = assemble(src, &Options::default()).expect("assemble");
    let diags = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new())
        .err()
        .unwrap_or_else(|| panic!("`LC` must not resolve — asl never reads the closer it sits on"));
    let msgs: Vec<&String> = diags.iter().map(|d| &d.message).collect();
    assert!(
        msgs.iter().any(|m| m.contains("LC")),
        "expected `LC` to be the unresolved symbol, got {msgs:?}"
    );
}

/// COVERAGE, NOT A GATE — and deliberately so.
///
/// asl SEGFAULTS on an `exitm` inside an `irp`/`irpc` (exit 139, core dumped),
/// at top level and nested in a macro alike (probes `e8`, `e11`). There is no
/// reference answer for this cell at any nesting, so nothing here is DERIVED
/// from asl: sigil treats `irp`/`irpc` as the expansion frame `rept` and `while`
/// are, because that is the only reading consistent with its two siblings.
///
/// This test states what sigil does so the choice cannot drift unnoticed. It is
/// not evidence that the choice is right, and it must not be quoted as such.
#[test]
fn exitm_in_an_irp_ends_the_irp_no_asl_oracle_exists() {
    let src = concat!(
        "\tcpu 68000\n\tpadding off\n\tphase 0\n",
        "m8 macro\n\tdc.b $A0\n\tirp x,1,2,3\n\tdc.b $C0\n\texitm\n\tdc.b $C1\n\tendm\n\tdc.b $A1\n\tendm\n",
        "\tm8\n\tdc.b $FF\n",
    );
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked =
        sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(
        sigil_link::flatten(&linked, 0x00),
        vec![0xA0, 0xC0, 0xA1, 0xFF],
        "sigil's UNVERIFIED choice: one irp iteration, then the enclosing macro continues"
    );
}
