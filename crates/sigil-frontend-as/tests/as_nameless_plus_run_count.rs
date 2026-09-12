//! AS's nameless `+` RUNS. A `++` or `+++` definition names the forward slot
//! two or three past the counter and leaves the counter where it stands; only
//! a single `+` (or a `/`) advances it. A later single `+` that reaches the
//! run's slot is `symbol double defined`. A nameless name, definition or
//! reference, is one to three characters.
//!
//! Row AS-NAMELESS-PLUS-RUN-COUNT. Every fixture is a probe file asl assembled,
//! read at test time from
//! `docs/superpowers/notes/2026-09-12-as-nameless-plus-run-count/probes/`, so
//! the text sigil is tested on cannot drift from the text the oracle answered.
//! Every expected byte string is that probe's `p2bin` image from `$100` on,
//! from asl 1.42 Beta Bld 212 (`s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, through `asl_ref.sh`'s `asl_run` with
//! `-xx -n -q -A -L -U -i .`) on a run that exited 0. Those runs are recorded
//! in `asl-symbols.txt` beside the probes, with asl's own nameless symbol
//! names: `__forwN` is zero-based, so asl's `__forwN` is slot `N + 1` in
//! `src/nameless.rs`. Every expected refusal is a probe asl refused, and the
//! comment names asl's error.
//!
//! Every probe sits at `org $100` behind a `$1111` word and gives each
//! definition its own data word, so a reference's value names the line it
//! landed on and a refusal cannot hide as a zero.

use sigil_frontend_as::{assemble, Options};
use std::path::PathBuf;

fn notes() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/superpowers/notes")
}

/// This row's probes.
fn probes() -> PathBuf {
    notes().join("2026-09-12-as-nameless-plus-run-count/probes")
}

/// Assemble and link one probe. `Ok` is the image from `$100` on, `Err` the
/// messages of whichever stage refused it.
fn run_in(dir: PathBuf, name: &str) -> Result<Vec<u8>, Vec<String>> {
    let path = dir.join(format!("{name}.asm"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("probe {} unreadable: {e}", path.display()));
    let opts = Options { include_root: Some(dir), ..Default::default() };
    let module = match assemble(&text, &opts) {
        Ok(m) => m,
        Err(diags) => return Err(diags.into_iter().map(|d| d.message).collect()),
    };
    match sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()) {
        Ok(linked) => Ok(sigil_link::flatten(&linked, 0x00).unwrap()[0x100..].to_vec()),
        Err(diags) => Err(diags.into_iter().map(|d| d.message).collect()),
    }
}

fn words(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .flat_map(|w| {
            let v = u16::from_str_radix(w, 16).expect("hex word");
            [(v >> 8) as u8, v as u8]
        })
        .collect()
}

#[track_caller]
fn builds(name: &str, asl: &str) {
    match run_in(probes(), name) {
        Ok(b) => assert_eq!(b, words(asl), "{name}: asl emits {asl}"),
        Err(m) => panic!("{name}: asl assembles it to {asl}, sigil refused: {m:?}"),
    }
}

#[track_caller]
fn refused_in(dir: PathBuf, name: &str, fragment: &str) {
    match run_in(dir, name) {
        Ok(b) => panic!("{name}: asl refuses it, sigil built {b:02x?}"),
        Err(m) => assert!(
            m.iter().any(|x| x.contains(fragment)),
            "{name}: refused, but no message mentions `{fragment}`: {m:?}"
        ),
    }
}

#[track_caller]
fn refused(name: &str, fragment: &str) {
    refused_in(probes(), name, fragment);
}

/// A reference that spans a `+` run, at file level. asl's symbol table for the
/// run shape `+`, `++`, `+` at $104/$106/$108 reads `__forw0 = 104`,
/// `__forw2 = 106`, `__forw1 = 108`: the `++` is slot 3 and the `+` after it
/// slot 2. So from before the run `++` is the LAST `+` (`b02`, `$0108`) and
/// `+++` the `++` (`b03`, `$0106`); after a `++` a `++` reference lands behind
/// itself, on that `++` (`b06`, `b08` on the `++`'s own line, `b17` as a
/// `bra.s`, `60FC`).
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: a counter that advances by the
/// run's length puts the `+` after a `++` one slot further on, so `b02`,
/// `b06`, `b08`, `b09`, `b12`, `b13`, `b17` and `b18` name a slot nothing
/// defines and are refused, which is what sigil did before this row. The rest
/// are shapes both counters agree on, kept so the rule cannot drift on them.
#[test]
fn a_reference_spanning_a_plus_run_lands_where_asl_puts_it() {
    builds("b02_ref_pp_over_p_pp_p", "1111 0108 2222 3333 5555 4444");
    builds("b06_pp_ref_pp_p", "1111 2222 0102 3333 4444");
    builds("b08_pp_sameline_ref_pp", "1111 0102 3333 4444");
    builds("b09_bra_pp_over_run", "1111 6004 2222 3333 5555 4444");
    builds("b12_p_pp_ref_pp_p", "1111 2222 3333 0104 5555 4444");
    builds("b13_ref_p_over_ppp_p_p", "1111 0106 2222 3333 5555 4444");
    builds("b17_bra_pp_after_pp", "1111 2222 60fc 3333 4444");
    builds("b18_slash_pp_p_ref_pp", "1111 0108 2222 3333 5555 4444");
    builds("b01_ref_p_over_p_pp_p", "1111 0104 2222 3333 5555 4444");
    builds("b03_ref_ppp_over_p_pp_p", "1111 0106 2222 3333 5555 4444");
    builds("b05_pp_ref_p_p", "1111 2222 0106 3333 4444");
    builds("b07_pp_sameline_ref_p", "1111 0104 3333 4444");
    builds("b10_bra_ppp_over_run", "1111 6002 2222 3333 5555 4444");
    builds("b11_p_pp_ref_p_p", "1111 2222 3333 0108 5555 4444");
    builds("b14_ref_ppp_over_ppp_p_p", "1111 0104 2222 3333 5555 4444");
    builds("b19_bra_p_after_pp_nop", "1111 2222 6002 4e71 3333 4444");
    // Definitions only: a run and a single `+` that do NOT meet on a slot.
    builds("a05_ppp_pp", "1111 2222 3333 4444");
    builds("a06_pp_ppp", "1111 2222 3333 4444");
    builds("a08_p_p_pp_p", "1111 2222 3333 5555 6666 4444");
    builds("a11_slash_pp_p", "1111 2222 3333 5555 4444");
}

/// The same rule inside a macro body and a loop iteration, where the run's
/// slot is filed in the instance and the counter is still the shared one.
/// `c01` and `d02` land behind themselves on their own instance's `++`;
/// `c03`, `c04` and `d01` reach PAST a body's or an iteration's run to a
/// file-level `+`, because the run did not move the counter (`__forw0` is the
/// file-level `+` in all three); `c06` shows a body's slot 3 and a file-level
/// slot 3 are different symbols.
///
/// `c05` (asl `#1010 symbol undefined`) is the body's `++` named from after
/// the call, out of reach. `n21` (the macro-label-leak note's probe, the
/// shape this row was opened for) names `+++` before a body `++` and a
/// file-level `+`: slot 3 is the body's, so asl refuses it (`#1010 symbol
/// undefined`, its line 9); a counter that advances by the run's length put
/// the file-level `+` on slot 3 and built it.
#[test]
fn a_plus_run_in_a_macro_body_or_a_loop_iteration_leaves_the_counter_too() {
    builds("c01_body_pp_ref_pp", "1111 2222 0102 4444");
    builds("c02_body_ref_pp_over_run", "1111 0108 2222 3333 5555 4444");
    builds("c03_ref_p_body_pp_file_p", "1111 0106 2222 3333 4444");
    builds("c04_ref_p_body_ppp_twice", "1111 0108 2222 2222 3333 4444");
    builds("c06_body_p_pp_then_file", "1111 2222 3333 0108 5555 6666 4444");
    builds("d01_ref_p_rept2_pp_file_p", "1111 0108 2222 2222 3333 4444");
    builds("d02_rept2_pp_ref_pp", "1111 2222 0102 2222 0106 4444");
    builds("d03_ref_ppp_rept2_p_file_pp_p", "1111 010a 2222 2222 3333 5555 4444");
    builds("d04_rept2_ref_p_pp_p", "1111 0106 2222 3333 010c 2222 3333 4444");
    refused("c05_body_pp_then_file_ref", "nameless");
    refused_in(
        notes().join("2026-09-12-as-macro-label-leak/probes2"),
        "n21_plusplus_in_body",
        "nameless",
    );
}

/// Because a run leaves the counter standing, the single `+` definitions after
/// it count up onto the run's slot, and then two definitions name one symbol:
/// asl's `error #1000: symbol double defined`, on the line that reaches it.
/// `a01` (`+`, `++`, `+`, `+`: the fourth), `a02` (`++`, `++`), `a03`, `a04`,
/// `a12`; inside one macro expansion (`g01`), in each iteration of a `rept`
/// (`g03`, reported for both), where the second definition is a `/` (`g02`),
/// and where a body's `+` moves the shared counter onto a file-level run's
/// slot (`g04`).
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: a slot bound twice is silently
/// rebound to the later address, and the file builds. That is what happens
/// with the counter rule alone and no check.
#[test]
fn a_single_plus_that_reaches_a_runs_slot_is_symbol_double_defined() {
    for name in [
        "a01_p_pp_p_p",
        "a02_pp_pp",
        "a03_ppp_p_p_p",
        "a04_pp_p_p",
        "a12_p_pp_p_p_pp",
        "g01_body_p_pp_p_p",
        "g02_pp_slash_p",
        "g03_rept2_p_pp_p_p",
        "g04_pp_body_p_file_p",
    ] {
        refused(name, "symbol double defined");
    }
}

/// `++++` and `+++++` in column 1 are asl's `error #1020: invalid symbol name`
/// (`a07`, `f03`), exactly as `--` and `//` are (`e01`, `e02`, and `e03` in a
/// macro body). `+++` is a definition (`a05`, `b14`).
#[test]
fn a_nameless_definition_is_at_most_three_plus_signs() {
    for name in ["a07_pppp", "f03_ppppp_def", "e01_mm_def", "e02_ss_def", "e03_body_mm_def"] {
        refused(name, "not nameless labels");
    }
    builds("a05_ppp_pp", "1111 2222 3333 4444");
    builds("b14_ref_ppp_over_ppp_p_p", "1111 0104 2222 3333 5555 4444");
}

/// A reference of four or more signs is not a nameless name either: asl reads
/// the run as operators and answers `error #1110: wrong number of operands`,
/// even with a fourth definition in place for it to name (`f01` `dc.w ++++`
/// over four `+`, `f04` as a `bra.s`, `f02` `dc.w ----` under four `-`, `f08`
/// `dbf d0,----`, `f06` `-----1`; `b04` has no fourth slot). In front of an
/// operand the last sign is the operator, so `----1` is `(---) - 1`, `$0101`
/// (`f05`), and `++++1` is `(+++) + 1`, `$0109` (`f07`).
///
/// sigil's refusal is its expression parser's (`bad word expression`, `bad
/// operand expression`).
#[test]
fn a_nameless_reference_is_at_most_three_signs() {
    for name in [
        "f01_ref_pppp_over_four_p",
        "f02_four_m_ref_mmmm",
        "f04_bra_pppp_over_four_p",
        "f06_four_m_ref_mmmmm_minus_1",
        "f08_dbf_mmmm",
        "b04_ref_pppp_over_p_pp_p",
    ] {
        refused(name, "expression");
    }
    builds("f05_three_m_ref_mmm_minus_1", "1111 2222 3333 5555 0101 4444");
    builds("f07_ref_pppp_plus_1_over_three_p", "1111 0109 2222 3333 5555 4444");
}

/// asl gives a `/` a forward name only and counts `-` alone on its backward
/// counter (`a09`: `/`, `-` is `__forw0`, `__back0`; `e07`, `e08`, `e09`: a
/// `/` in a body or a loop leaves the next file-level `-` at `__back0`), yet a
/// backward reference reaches a `/` (`e04`, `e05`). sigil counts `-` and `/`
/// in one backward sequence; the names differ from asl's table and every
/// address below is asl's. `e06`, `e10` and `e11` (asl `#1010 symbol
/// undefined`) are a `--` whose second-nearest backward definition is a body's
/// or an iteration's, out of reach.
#[test]
fn a_backward_reference_reaches_a_slash_where_asl_resolves_it() {
    builds("e04_body_slash_minus_ref_mm", "1111 2222 3333 0102 4444");
    builds("e05_body_minus_slash_ref_mm", "1111 2222 3333 0102 4444");
    builds("e07_body_slash_twice_file_minus", "1111 2222 2222 3333 4444");
    builds("e08_rept_slash_file_minus", "1111 2222 3333 4444");
    builds("e09_body_slash_file_plus_minus", "1111 2222 3333 5555 0106 4444");
    builds("e12_body_slash_ref_p_file_p", "1111 2222 0106 3333 4444");
    builds("b15_minus_run_ref", "1111 2222 3333 5555 0102 4444");
    builds("a09_slash_minus", "1111 2222 3333 4444");
    builds("a10_minus_slash_minus", "1111 2222 3333 5555 4444");
    refused("e06_file_minus_body_slash_file_minus_ref_mm", "nameless");
    refused("e10_file_minus_rept_slash_file_minus_ref_mm", "nameless");
    refused("e11_body_minus_file_minus_ref_mm", "nameless");
}
