//! An `if` whose condition does not evaluate is REFUSED, not read as false.
//!
//! ## What was silently wrong
//!
//! ```text
//!     if      Nowhere        ; Nowhere is never defined
//!     dc.l    $11111111
//!     endif
//!     dc.l    $22222222
//! ```
//!
//! assembled to `22 22 22 22`, exit 0, no diagnostic. With `Nowhere equ 0` it
//! assembled to `22 22 22 22`. **An undefined condition was byte-for-byte
//! indistinguishable from an explicit false**, so a typo in a condition name
//! deleted the code it guarded and the build succeeded. `eval_if_expr` ended in
//! `.unwrap_or(false)`, and `false` is a verdict.
//!
//! This is not a hypothetical shape. The corpus reaches it eleven times, at
//! eleven distinct source positions, all through conditions sigil cannot
//! evaluate: `MOMPASS`, an AS builtin sigil does not implement, guards seven of
//! them. Every one of those was a branch chosen with no basis, and the choice
//! emitted bytes.
//!
//! ## Why refusing does not fire on correct code
//!
//! The refusal is raised at the site on EVERY pass, and only the CONVERGED
//! pass's diagnostics are returned, the arrangement `rept` and `while` already
//! use for their own unresolved-count and unresolved-condition words. So a
//! legitimate FORWARD reference is untouched: sigil resolves `if Later` by
//! iterating to a fixpoint, and a name defined later in the file, in a file
//! included after the `if`, or by a later `set` has a value by the time the
//! returned pass runs.
//!
//! asl is STRICTER here, not looser. Its rule is `expression must be evaluatable
//! in first pass` and it refuses all four of those forward shapes with exit 2
//! (reference build md5 `61e672562465725a8c102288a7da9098`, exit status
//! checked). Refusing only what is still unresolved at convergence is therefore
//! strictly weaker than the reference and cannot red a program asl accepts.
//! Sigil deliberately keeps the extra tolerance; the tests below pin it.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble `body` as a real file and hand back either the linked bytes or the
/// diagnostics, whichever the run produced.
///
/// A real file rather than a string: `SourceMap::label` renders `file(line)`
/// only for a NAMED source, and a refusal that cannot say which line it is
/// about is the thing this parcel replaced.
fn assemble(body: &str) -> Result<Vec<u8>, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00))
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                // The 1-based line, from the map, so an expectation names a line
                // rather than a byte offset nobody can check by eye.
                let line = f
                    .sources
                    .label(d.primary)
                    .and_then(|l| {
                        l.rsplit_once('(')
                            .and_then(|(_, n)| n.trim_end_matches(')').parse().ok())
                    })
                    .unwrap_or(0);
                (line, d.message.clone())
            })
            .collect()),
    }
}

/// The head of the refusal, spelled out rather than imported, because the text
/// is the contract: a reworded message must red this file loudly rather than
/// keep matching on a fragment that survived the rewording.
const IF_HEAD: &str = "unresolved if condition: ";
const ELSEIF_HEAD: &str = "unresolved elseif condition: ";

fn errors(body: &str) -> Vec<(u32, String)> {
    match assemble(body) {
        Ok(bytes) => panic!(
            "expected a refusal, the unit assembled to {bytes:02X?} and said nothing \
             which is exactly the silent wrong answer this file exists to prevent"
        ),
        Err(d) => d,
    }
}

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected this to assemble, got {d:?}"),
    }
}

// ---- the fault -------------------------------------------------------------

#[test]
fn an_if_on_an_undefined_symbol_is_refused_and_names_it() {
    let diags = errors("\tcpu\t68000\n\tif\tNowhere\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n");
    let hit = diags
        .iter()
        .find(|(_, m)| m.starts_with(IF_HEAD))
        .unwrap_or_else(|| panic!("no `{IF_HEAD}` diagnostic in {diags:?}"));
    assert_eq!(hit.0, 2, "the refusal names the `if` line: {diags:?}");
    assert!(
        hit.1.contains("`Nowhere`"),
        "the refusal names the symbol with no value: {}",
        hit.1
    );
}

#[test]
fn an_elseif_on_an_undefined_symbol_is_refused_and_names_it() {
    let diags = errors(
        "\tcpu\t68000\n\tif\t0\n\tdc.l\t$11111111\n\telseif\tNowhere\n\tdc.l\t$33333333\n\tendif\n",
    );
    let hit = diags
        .iter()
        .find(|(_, m)| m.starts_with(ELSEIF_HEAD))
        .unwrap_or_else(|| panic!("no `{ELSEIF_HEAD}` diagnostic in {diags:?}"));
    assert_eq!(hit.0, 4, "the refusal names the `elseif` line: {diags:?}");
    assert!(hit.1.contains("`Nowhere`"), "names the symbol: {}", hit.1);
}

/// The discriminator the fault was invisible without: an undefined condition
/// and an explicit false produced the SAME bytes and the same exit status, so
/// nothing downstream could tell them apart. `if 0` must keep producing exactly
/// those bytes, silently.
#[test]
fn an_explicit_false_still_assembles_silently_to_the_same_bytes() {
    assert_eq!(
        bytes("\tcpu\t68000\n\tif\t0\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x22, 0x22, 0x22, 0x22]
    );
}

#[test]
fn an_explicit_true_still_assembles_both() {
    assert_eq!(
        bytes("\tcpu\t68000\n\tif\t1\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

// ---- the over-firing gates -------------------------------------------------
//
// Everything below is a shape the refusal must NOT fire on. A new refusal that
// fires on correct code is worse than no refusal, because the remedy people
// reach for is to weaken it.

/// A condition that is a defined constant.
#[test]
fn a_defined_constant_condition_does_not_fire() {
    assert_eq!(
        bytes("\tcpu\t68000\nK\tequ\t1\n\tif\tK\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// A condition that is a legitimate compound expression.
#[test]
fn a_compound_expression_condition_does_not_fire() {
    assert_eq!(
        bytes(
            "\tcpu\t68000\nK\tequ\t3\nJ\tequ\t4\n\tif\t(K*2)=6\n\tdc.l\t$11111111\n\tendif\n\
             \tdc.l\t$22222222\n"
        ),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// A FORWARD reference to an `equ` defined later in the same file. Sigil accepts
/// this by iterating to a fixpoint; asl refuses it (`expression must be
/// evaluatable in first pass`, exit 2). The tolerance is deliberate and this
/// pins it, so a refusal keyed on the first pass instead of on convergence
/// would red here.
#[test]
fn a_forward_equ_condition_does_not_fire() {
    assert_eq!(
        bytes("\tcpu\t68000\n\tif\tLater\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\nLater\tequ\t1\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// A FORWARD reference to a LABEL defined later in the same file.
#[test]
fn a_forward_label_condition_does_not_fire() {
    assert_eq!(
        bytes(
            "\tcpu\t68000\n\tif\tLater>0\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n\
             Later:\n\tdc.l\t$33333333\n"
        ),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22, 0x33, 0x33, 0x33, 0x33]
    );
}

/// A FORWARD reference to a name a later `set` defines.
#[test]
fn a_forward_set_condition_does_not_fire() {
    assert_eq!(
        bytes("\tcpu\t68000\n\tif\tLater\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\nLater\tset\t1\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// `ifdef`/`ifndef` on a name nothing defines is their whole point: not-defined
/// IS their answer, asl prints `=>UNDEFINED` and exits 0, and the refusal must
/// never reach them.
#[test]
fn ifdef_on_an_undefined_name_does_not_fire() {
    assert_eq!(
        bytes("\tcpu\t68000\n\tifdef\tNowhere\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x22, 0x22, 0x22, 0x22]
    );
    assert_eq!(
        bytes("\tcpu\t68000\n\tifndef\tNowhere\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// The string-comparison condition shape the corpus uses 25 times in s2disasm
/// alone (`if "__LABEL__"<>""`, `if MOMCPUNAME="Z80"`). It is decided before any
/// numeric fold is attempted and must stay decided.
#[test]
fn a_string_comparison_condition_does_not_fire() {
    assert_eq!(
        bytes(
            "\tcpu\t68000\n\tif\tMOMCPUNAME=\"Z80\"\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"
        ),
        vec![0x22, 0x22, 0x22, 0x22]
    );
    assert_eq!(
        bytes("\tcpu\t68000\n\tif\t\"a\"<>\"b\"\n\tdc.l\t$11111111\n\tendif\n\tdc.l\t$22222222\n"),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// An `elseif` AFTER a taken arm is never evaluated, and asl agrees (probe
/// `elseif_unreached`: `=>FALSE`, 0 errors, exit 0), so an undefined symbol
/// sitting there is not a refusal.
#[test]
fn an_elseif_after_a_taken_arm_is_not_evaluated() {
    assert_eq!(
        bytes(
            "\tcpu\t68000\n\tif\t1\n\tdc.l\t$11111111\n\telseif\tNowhere\n\tdc.l\t$33333333\n\
             \tendif\n\tdc.l\t$22222222\n"
        ),
        vec![0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x22, 0x22]
    );
}

/// An `if` nested inside a NOT-taken arm is never read, and asl agrees (probe
/// `if_in_skipped`: 0 errors, exit 0).
#[test]
fn an_if_inside_a_skipped_arm_is_not_evaluated() {
    assert_eq!(
        bytes(
            "\tcpu\t68000\n\tif\t0\n\tif\tNowhere\n\tdc.l\t$11111111\n\tendif\n\tendif\n\
             \tdc.l\t$22222222\n"
        ),
        vec![0x22, 0x22, 0x22, 0x22]
    );
}

/// One condition at one source position is one verdict. The corpus expands a
/// macro carrying `if MOMPASS=1` 81 times from one line, and reporting per
/// expansion buried the eleven distinct positions under 114 rows.
#[test]
fn one_condition_reached_many_times_is_reported_once() {
    let diags = errors(
        "\tcpu\t68000\nm\tmacro\n\tif\tNowhere\n\tdc.l\t$11111111\n\tendif\n\tendm\n\
         \tm\n\tm\n\tm\n\tm\n",
    );
    let hits = diags.iter().filter(|(_, m)| m.starts_with(IF_HEAD)).count();
    assert_eq!(hits, 1, "four expansions of one line, one verdict: {diags:?}");
}
