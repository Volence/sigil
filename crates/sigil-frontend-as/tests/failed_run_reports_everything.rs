//! A run that is going to FAIL reports the diagnostics an ordinary pass finds,
//! not the quieter set the deferral ("bonus") pass produces.
//!
//! The bonus pass runs once after convergence with `defer_unresolved_jsr_jmp`
//! set, and both things it does are SUPPRESSIONS made in service of a LINK:
//!
//!   1. a `jsr`/`jmp` bare-symbol target still folding to `Poison` becomes a
//!      deferred `Fragment::JmpJsrSym` instead of an `unresolved symbol … in
//!      operand` error, and
//!   2. `keep_labels_symbolic()`, SEVEN call sites in `eval.rs`, in
//!      `directive_equate`, `directive_dc_w`, `directive_dc_l`, `lower_m68k`
//!      (twice), `try_defer_long_imm` and `fixup_target`, short-circuits a
//!      label-referencing operand BEFORE its fold, so a compound operand that
//!      folds to `Poison` raises nothing where an ordinary pass raises
//!      `unresolved long expression`.
//!
//! A run that already holds an error will never reach the link those
//! suppressions serve, so it takes the converged ordinary pass instead. The
//! gate is `!force_relocate && (poison.is_empty() || already_failed)` in
//! `eval::run_impl`; `force_relocate` is the first conjunct, so a CHAINED
//! (relocating) ROM build is untouched.
//!
//! `sonic_hack`'s `S4.asm` is the corpus root that measured the difference: it
//! reports 3080 diagnostics with the bonus pass and 3083 without, the three
//! being `code/engines/hud.asm(1408): unresolved symbol \`LoadLevelLayout\` in
//! operand` and `code/engines/art_data.asm(23)` and `(24)`, both `unresolved
//! long expression`. Those two are the `dc.l (plc1<<24)|art` and `dc.l
//! (plc2<<24)|map16x16` lines of its `levartptrs` macro, where the PLC id is an
//! `equ` that never resolves and the art pointer is a real label, so
//! `expr_refs_label` is true and the fold is `Poison`, which is exactly the
//! `keep_labels_symbolic` short-circuit. `levartptrs_shape_*` below is that
//! construct reduced to one file, and it moves the same three lines.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble one body and return `Err((line, message))` per diagnostic, or `Ok`
/// with the diagnostic-free module's section count.
fn run(body: &str) -> Result<usize, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => Ok(m.sections.len()),
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        l.rsplit_once('(')
                            .and_then(|(_, rest)| rest.split_once(')'))
                            .and_then(|(n, _)| n.parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

fn diags(body: &str) -> Vec<(u32, String)> {
    match run(body) {
        Ok(n) => panic!("expected a failing run, got Ok with {n} sections"),
        Err(d) => d,
    }
}

/// `line` carries a diagnostic whose message contains `needle`. Prints the whole
/// diagnostic set on failure, never a tail excerpt.
fn assert_reported(got: &[(u32, String)], line: u32, needle: &str) {
    assert!(
        got.iter().any(|(l, m)| *l == line && m.contains(needle)),
        "expected `{needle}` at line {line}; the run reported {} diagnostics:\n{}",
        got.len(),
        got.iter()
            .map(|(l, m)| format!("  ({l}) {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn assert_not_reported(got: &[(u32, String)], needle: &str) {
    assert!(
        !got.iter().any(|(_, m)| m.contains(needle)),
        "did NOT expect `{needle}`; the run reported {} diagnostics:\n{}",
        got.len(),
        got.iter()
            .map(|(l, m)| format!("  ({l}) {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The `levartptrs` construct: an `equ` chain that never resolves (`id()` of an
/// undefined pointer) shifted into a `dc.l` alongside a real art LABEL, plus
/// leftover poison (`jsr LoadLevelLayout`) to force the bonus-pass decision,
/// plus one unrelated error to make the run a FAILING one.
///
/// Line map, used by every assertion below, so a body edit that shifts a line
/// fails loudly rather than silently testing a different line:
///   7  `dc.l (plc1<<24)|art`          -> unresolved long expression
///   8  `dc.l (plc2<<24)|map16x16`     -> unresolved long expression
///   9  `dc.l (palette<<24)|map128x128` -> CLEAN (the palette id resolves)
///  21  `jsr LoadLevelLayout`          -> unresolved symbol in operand
///  23  `moveq #$1FF,d0`               -> the unrelated error
const LEVARTPTRS_FAILING: &str = "\tcpu 68000\n\
id function ptr,(ptr-PLCPointers)/4\n\
PLCID_Ojz1 =\t\tid(PLCPtr_Ojz1)\n\
PLCID_Ojz2 =\t\tid(PLCPtr_Ojz2)\n\
PalID_OJZ =\t\t0\n\
levartptrs macro plc1,plc2,palette,art,map16x16,map128x128\n\
\tdc.l (plc1<<24)|art\n\
\tdc.l (plc2<<24)|map16x16\n\
\tdc.l (palette<<24)|map128x128\n\
    endm\n\
\torg 0\n\
LevelArtPointers:\n\
\tlevartptrs PLCID_Ojz1, PLCID_Ojz2, PalID_OJZ, Tiles_OJZ, Blocks_OJZ, Chunks_OJZ\n\
Tiles_OJZ:\n\
\tdc.w 0\n\
Blocks_OJZ:\n\
\tdc.w 0\n\
Chunks_OJZ:\n\
\tdc.w 0\n\
Hud:\n\
\tjsr LoadLevelLayout\n\
Bad:\n\
\tmoveq #$1FF,d0\n";

/// THE `sonic_hack` LINES. The two `unresolved long expression` the corpus root
/// gains are the `keep_labels_symbolic` suppression, and they are what the
/// owner's ruling on d-28 makes a failing run report.
#[test]
fn levartptrs_shape_reports_both_unresolved_long_expressions_on_a_failing_run() {
    let got = diags(LEVARTPTRS_FAILING);
    assert_reported(&got, 7, "unresolved long expression");
    assert_reported(&got, 8, "unresolved long expression");
}

/// The third `dc.l` of the same macro resolves, so it must stay clean: the gate
/// reports what an ordinary pass FINDS, it does not blanket-poison the macro.
#[test]
fn levartptrs_shape_leaves_the_resolvable_third_pointer_alone() {
    let got = diags(LEVARTPTRS_FAILING);
    assert!(
        !got.iter().any(|(l, _)| *l == 9),
        "line 9 resolves and must carry no diagnostic; got:\n{}",
        got.iter()
            .map(|(l, m)| format!("  ({l}) {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// THE DIRECTION THAT MUST NEVER BREAK. Skipping the bonus pass must not lose
/// the diagnostics the bonus pass was the one to raise: its leftover `poison`
/// is reported by the ordinary arm instead. Without this the failing run would
/// be QUIETER about `jsr LoadLevelLayout` than it was before, which is the one
/// outcome the nine-root measurement forbade.
#[test]
fn a_skipped_bonus_pass_still_reports_its_leftover_poison() {
    let got = diags(LEVARTPTRS_FAILING);
    assert_reported(&got, 21, "unresolved symbol `LoadLevelLayout` in operand");
}

/// The unrelated error that makes the run a failing one is itself still
/// reported: the gate adds, it does not replace.
#[test]
fn the_error_that_opened_the_gate_is_still_reported() {
    let got = diags(LEVARTPTRS_FAILING);
    assert_reported(&got, 23, "moveq data 511 does not fit in a signed byte");
}

/// THE GATE'S OTHER DIRECTION. The same file with the unrelated error removed
/// is NOT a failing run at the converged pass, so the bonus pass still runs and
/// still defers: `jsr LoadLevelLayout` becomes a `Fragment::JmpJsrSym` and the
/// front end returns `Ok`. This is the behaviour a cross-seam `.emp`/AS module
/// depends on, and it is what stops the gate from being "never run the bonus
/// pass".
#[test]
fn a_run_with_nothing_else_wrong_still_takes_the_bonus_pass() {
    // The macro is dropped too: its `unresolved long expression` is itself an
    // error at the converged pass and would open the gate on its own.
    let body = "\tcpu 68000\n\torg 0\nHud:\n\tjsr LoadLevelLayout\n";
    match run(body) {
        Ok(_) => {}
        Err(d) => panic!(
            "an otherwise-clean run must still defer its jsr target; got:\n{}",
            d.iter()
                .map(|(l, m)| format!("  ({l}) {m}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

/// A WARNING does not open the gate: `already_failed` is `Level::Error` (or a
/// carried `fatal`), not "any diagnostic". A run whose only complaint is a
/// warning still reaches the bonus pass and still defers.
#[test]
fn a_warning_alone_does_not_open_the_gate() {
    let body = "\tcpu 68000\n\torg 0\n\twarning \"just a warning\"\nHud:\n\tjsr LoadLevelLayout\n";
    match run(body) {
        Ok(_) => {}
        Err(d) => panic!(
            "a warning must not open the failed-run gate; got:\n{}",
            d.iter()
                .map(|(l, m)| format!("  ({l}) {m}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

/// CONTAINMENT, in the shape the measurement used: every diagnostic the run
/// produced with the bonus pass is still produced without it. The bonus-pass
/// set is not reachable from this crate's public API, so it is written down
/// here as the literal set the pre-gate binary printed on this exact body
/// (`sigil` at master `27db72a7`, md5 `02b1c129666d53cafb508d03d94c9507`):
/// one line, the `moveq`. The assertion is CONTAINMENT plus a printed
/// difference, never `new == old + 3`: two sets can differ by one member in
/// each direction and still total the same.
#[test]
fn the_new_diagnostic_set_contains_the_old_one() {
    let got = diags(LEVARTPTRS_FAILING);
    let old: &[(u32, &str)] = &[(23, "unsupported form: moveq data 511 does not fit in a signed byte")];
    let missing: Vec<_> = old
        .iter()
        .filter(|(l, m)| !got.iter().any(|(gl, gm)| gl == l && gm == m))
        .collect();
    assert!(
        missing.is_empty(),
        "the pre-gate diagnostic set is NOT contained in the post-gate one; missing {missing:?}\n\
         post-gate set:\n{}",
        got.iter()
            .map(|(l, m)| format!("  ({l}) {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let appeared: Vec<_> = got
        .iter()
        .filter(|(l, m)| !old.iter().any(|(ol, om)| ol == l && om == m))
        .map(|(l, m)| format!("({l}) {m}"))
        .collect();
    // Reported, not asserted by size: the identities are the finding.
    assert_eq!(
        appeared,
        vec![
            "(7) unresolved long expression".to_string(),
            "(8) unresolved long expression".to_string(),
            "(21) unresolved symbol `LoadLevelLayout` in operand".to_string(),
        ],
        "the set the gate ADDS changed"
    );
}

/// The gate is `already_failed`, not "there is poison": a poison-free failing
/// run never reached the bonus pass to begin with, and must be untouched.
#[test]
fn a_poison_free_failing_run_is_unchanged() {
    let body = "\tcpu 68000\n\torg 0\nBad:\n\tmoveq #$1FF,d0\n";
    let got = diags(body);
    assert_eq!(got.len(), 1, "got {got:?}");
    assert_reported(&got, 4, "moveq data 511 does not fit in a signed byte");
    assert_not_reported(&got, "unresolved");
}

/// THE WITNESS `451cb3e2` CITES, read from where it is committed:
/// `docs/superpowers/notes/2026-09-11-451cb3e2-witness/control.asm`. Nothing
/// else in it is wrong, so its only error is one the bonus pass would suppress,
/// and a module the front end used to return `Ok` is refused there instead,
/// with both lines named. The file is read rather than copied into this test so
/// the commit's pointer stays one anybody can follow.
#[test]
fn the_witness_451cb3e2_cites_is_refused_by_the_front_end() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-11-451cb3e2-witness/control.asm");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read the committed witness {}: {e}", path.display()));
    let got = diags(&body);
    assert_reported(&got, 4, "unresolved long expression");
    assert_reported(&got, 5, "unresolved symbol `NoSuchTarget` in operand");
    assert_eq!(got.len(), 2, "the witness draws exactly two diagnostics; got {got:?}");
}
