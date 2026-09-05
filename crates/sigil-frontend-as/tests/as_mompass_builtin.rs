//! `MOMPASS` is 1 on the first iteration of the fixpoint and 2 on every later
//! one, and the reasons it is not a running count are measurements, not taste.
//!
//! ## What asl's `MOMPASS` is
//!
//! asl's own `MOMPASS` is its 1-based pass counter, readable as an ordinary
//! value, and it is NOT bounded by 2. A file with no forward reference runs one
//! pass and `dc.b MOMPASS` emits `01`. Add a forward reference and it runs two
//! and emits `02`. Make an operand GROW between passes (an absolute address
//! that turns out not to fit in a word) and it runs three and emits `03`:
//!
//! ```text
//!     cpu 68000
//!     padding off
//!     supmode on
//!     org 0
//!     dc.w    MOMPASS
//!     move.w  d0,Sym
//! Later:
//!     dc.w    Later
//! Sym equ     $123456
//!     end
//!
//!        6/       0 : 0003                    dc.w MOMPASS
//!        7/       2 : 33C0 0012 3456          move.w  d0,Sym
//!        3 passes    0 errors    exit 0
//! ```
//!
//! ## Why sigil cannot report that number
//!
//! sigil's iteration count is measurably not asl's pass count. Over six probes
//! the two differ in three of six, in BOTH directions, and on the s2disasm root
//! asl takes 2 passes where sigil takes 4:
//!
//! ```text
//!   probe                              asl passes   sigil iterations
//!   no forward reference                    1              2
//!   one forward reference                   2              2
//!   forward `:=`                            2              3
//!   chained forward `:=`                    2              3
//!   absolute address that fits `.w`         2              2
//!   absolute address that grows to `.l`     3              3
//!   s2disasm s2.asm                         2              4
//! ```
//!
//! A running count would therefore make `if MOMPASS=2` at `s2.asm:91270` answer
//! FALSE where asl answers TRUE, for a reason that is a property of sigil's
//! fixpoint rather than of the program.
//!
//! ## What is portable
//!
//! The distinction the number is used for, which the corpus writes down at
//! `s2.constants.asm:972`: "Avoid undefined symbol errors by checking only
//! after the first pass." Reporting 1 then 2 makes that distinction exact and
//! decides all three idioms the corpora contain (`=1`, `>1`, `=2`) the way a
//! 2-pass asl decides them. Across s2disasm, s1disasm and skdisasm no `MOMPASS`
//! comparison names a pass number above 2, and aeon names `MOMPASS` in no file.
//!
//! ## Reference
//!
//! Every expected value below comes from `/home/volence/sonic_hacks/sonic_hack/
//! tools/as/asl`, md5 `61e672562465725a8c102288a7da9098`, invoked
//! `-xx -n -A -L -U -E -i .`, exit status checked and quoted at each test.
//! s2disasm's own asl (md5 `0dee1f98e6480a4783d27ffd8b90896f`) was not run for
//! any value here.

use sigil_frontend_as::{assemble_root_located, Options};

/// Assemble `body` as a real named file and hand back the linked bytes or the
/// diagnostics, whichever the run produced.
fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
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
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    }
}

/// The head every probe below shares, and the single forward reference that
/// makes asl run more than one pass. Without the forward reference asl would
/// run ONE pass, and a one-pass asl file is the shape sigil provably cannot
/// match (see `one_pass_asl_file_is_a_known_divergence`).
const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const FWD: &str = "\tdc.w Later-*\nLater:\n";

/// `dc.b MOMPASS` on the pass whose bytes are kept.
///
/// asl, probe `m_val`, exit 0, `2 passes`, `0 errors`:
///
/// ```text
///        4/       0 : 02                      dc.b MOMPASS
///        5/       1 : 11                      dc.b $11
///        6/       2 : 0002                    dc.w Later-*
/// ```
#[test]
fn mompass_reads_as_two_on_the_pass_that_emits() {
    let src = format!("{HEAD}\tdc.b MOMPASS\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(bytes(&src), vec![0x02, 0x11, 0x00, 0x02]);
}

/// `if MOMPASS=1` is FALSE on the pass whose bytes are kept, so the block it
/// guards is NOT assembled. This is the corpus's most common idiom: nine of
/// s2disasm's twelve sites, and every one of them guards a `message`,
/// `warning` or `fatal` rather than an emission.
///
/// asl, probe `m_eq1`, exit 0, `3 passes`, `0 errors`, bytes `11 00 02`. asl
/// needs three passes here for a reason worth reading: the guarded `dc.b $AA`
/// IS emitted on pass 1, which moves everything after it, so a further pass is
/// forced, and on that pass `MOMPASS` is no longer 1 and the byte is gone.
#[test]
fn mompass_eq_one_is_false_on_the_pass_that_emits() {
    let src = format!("{HEAD}\tif MOMPASS=1\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(bytes(&src), vec![0x11, 0x00, 0x02]);
}

/// `if MOMPASS > 1` is TRUE, so the block it guards IS assembled. This is
/// `s2.constants.asm:972`, whose own comment states the intent: "Avoid
/// undefined symbol errors by checking only after the first pass."
///
/// asl, probe `m_gt1`, exit 0, `3 passes`, `0 errors`, bytes `AA 11 00 02`.
#[test]
fn mompass_gt_one_is_true_on_the_pass_that_emits() {
    let src = format!("{HEAD}\tif MOMPASS>1\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(bytes(&src), vec![0xAA, 0x11, 0x00, 0x02]);
}

/// A pass number sigil never reports decides FALSE rather than refusing. No
/// corpus site names one (the census across s2disasm, s1disasm and skdisasm
/// finds only `=1`, `==1`, `>1` and `=2`), and answering FALSE is what a
/// 2-pass asl answers.
///
/// asl, probe `m_eq3`, exit 0, `2 passes`, `0 errors`, bytes `11 00 02`. Two
/// passes and not three because the guarded byte is never emitted, so the
/// layout never moves.
#[test]
fn mompass_eq_three_is_false() {
    let src = format!("{HEAD}\tif MOMPASS=3\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(bytes(&src), vec![0x11, 0x00, 0x02]);
}

/// The compound shape the corpus actually writes, at `s2.asm:88574`,
/// `s2.macros.asm:119` and `s2.macros.asm:224`: a real condition ANDed with
/// `MOMPASS=1`, guarding a `message`. Pinned as bytes because the `&&` has to
/// short-circuit to the same verdict as the bare form.
///
/// asl, probe `m_cmpd`, exit 0, `3 passes`, `0 errors`, bytes `11 00 02`.
#[test]
fn mompass_in_the_compound_corpus_condition() {
    let src = format!(
        "{HEAD}Z = 3\nN = 4\n\tif (Z<>N)&&(MOMPASS=1)\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x11, 0x00, 0x02]);
}

/// The refusal this parcel retires. Before `MOMPASS` had a value, the same
/// source produced
/// `error: unresolved if condition: \`MOMPASS\` has no value, so this condition
/// cannot decide whether the code it guards is assembled` and exit 1, at seven
/// distinct s2disasm positions. It must now produce bytes and no diagnostic
/// mentioning `MOMPASS`.
#[test]
fn mompass_no_longer_refuses_as_an_unresolved_condition() {
    let src = format!("{HEAD}\tif MOMPASS=1\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    match assemble(&src) {
        Ok(_) => {}
        Err(d) => panic!("MOMPASS still refuses: {d:?}"),
    }
}

/// `MOMPASS` outranks the symbol table, the rule `TRUE`, `FALSE` and `MOMCPU`
/// already follow. asl, probe `m_redef`, exit 0, `2 passes`, `0 errors`: it
/// accepts the line `MOMPASS = 7` without a word and `dc.b MOMPASS` still emits
/// `02`. (It is looser here than with `TRUE` and `MOMCPU`, which it refuses
/// outright with `error #2035`.) sigil matches the reported value: 2, not the 7
/// the source asked for.
#[test]
fn a_source_definition_does_not_displace_the_builtin() {
    let src = format!("{HEAD}MOMPASS = 7\n\tdc.b MOMPASS\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(bytes(&src)[0], 0x02, "a source `MOMPASS = 7` must not win");
}

// ---------------------------------------------------------------------------
// The two divergences from asl, asserted rather than left to be discovered.
// Both are stated in the note at docs/superpowers/notes/2026-09-05-as-mompass.md
// and booked in the gap ledger. Neither has any corpus population: every one of
// the twelve s2disasm sites, and every s1disasm and skdisasm site, guards a
// `message`, `warning`, `fatal` or a `:=` feeding one, and none reads MOMPASS
// as a value.
// ---------------------------------------------------------------------------

/// DIVERGENCE, and one no definition of `MOMPASS` could avoid. asl assembles a
/// file with no forward reference in ONE pass, so `dc.b MOMPASS` emits `01`
/// (probe `pA`, exit 0, `1 pass`, `0 errors`). sigil emits `02`: convergence
/// here requires `pass > 0`, so the iteration that emits bytes is never the
/// first one, and there is no value `MOMPASS` could take that would make it
/// report 1 on the pass that emits.
#[test]
fn one_pass_asl_file_is_a_known_divergence() {
    let src = format!("{HEAD}\tdc.b MOMPASS\n\tend\n");
    assert_eq!(
        bytes(&src),
        vec![0x02],
        "asl emits 01 here (1 pass); sigil emits 02 and cannot emit 01"
    );
}

/// The one shape that decides the design, and the reason it is a saturation
/// rather than a running count. It mirrors `s2.asm:91270`: an `if MOMPASS=2`
/// whose body emits NO bytes (it sets a `:=` that a later `dc.b` reads), in a
/// file whose iteration count is set by a forward `:=` chain rather than by
/// MOMPASS itself. So the guard cannot perturb the layout, and sigil's
/// iteration count exceeds asl's pass count for a reason unrelated to MOMPASS.
/// That is exactly the corpus situation: asl takes 2 passes on `s2.asm` where
/// sigil takes 4.
///
/// asl, probe `m_flag2`, exit 0, `2 passes`, `0 errors`:
///
/// ```text
///        9/       0 : AA                      dc.b FLAG
///       10/       1 : 11                      dc.b $11
///       11/       2 : 03                      dc.b V
///       12/       3 : EE                  W:  dc.b $EE
/// ```
///
/// Measured, not argued: the same source built with `pass as i64 + 1` in place
/// of the saturation emits `00 11 03 EE`, which is a byte divergence from asl
/// at the corpus's own `=2` shape. The saturation emits asl's bytes.
#[test]
fn mompass_eq_two_decides_the_corpus_shape_the_way_asl_does() {
    let src = format!(
        "{HEAD}FLAG := 0\nV := W\n\tif MOMPASS=2\nFLAG := $AA\n\tendif\n\
         \tdc.b FLAG\n\tdc.b $11\n\tdc.b V\nW:\tdc.b $EE\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0xAA, 0x11, 0x03, 0xEE]);
}

/// DIVERGENCE. `if MOMPASS=<n>` guarding a body that EMITS is self
/// destabilising under asl: emitting moves the layout, which forces another
/// pass, on which `MOMPASS` is no longer `<n>`, so asl settles with the body
/// OUT. asl, probe `m_eq2`, exit 0, `4 passes`, `0 errors`, bytes `11 00 02`.
/// sigil saturates at 2, so the condition stays TRUE and the fixpoint is stable
/// with the body IN: `AA 11 00 02`.
///
/// A running count WOULD match asl here, and that was measured rather than
/// assumed: built with `pass as i64 + 1` this same source emits asl's `11 00
/// 02`. The saturation is chosen anyway, because the shape it gets right
/// instead (`mompass_eq_two_decides_the_corpus_shape_the_way_asl_does`) is the
/// one the corpus contains and this one is not.
///
/// The corpus's one `=2` site, `s2.asm:91270`, guards a `message` and emits
/// nothing, so it is decided identically by both.
#[test]
fn mompass_eq_two_guarding_an_emission_diverges_from_asl() {
    let src = format!("{HEAD}\tif MOMPASS=2\n\tdc.b $AA\n\tendif\n\tdc.b $11\n{FWD}\tend\n");
    assert_eq!(
        bytes(&src),
        vec![0xAA, 0x11, 0x00, 0x02],
        "asl emits 11 00 02 here (4 passes, body out); sigil keeps the body in"
    );
}
