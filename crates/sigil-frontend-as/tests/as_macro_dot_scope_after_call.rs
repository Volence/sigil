//! A plain label written in a macro body is the `.`-local scope for everything
//! after it: the rest of that body, an enclosing body once the nested call
//! returns, and the caller once the expansion is over. The label itself stays
//! in its expansion; only the scope it opened travels.
//!
//! Row AS-MACRO-DOT-SCOPE-AFTER-CALL. Every fixture here IS a probe file asl
//! assembled: the source is read from
//! `docs/superpowers/notes/2026-09-12-as-macro-dot-scope-after-call/probes/` at
//! test time, so the text sigil is tested on cannot drift from the text the
//! oracle answered. Every expected byte string is that probe's `p2bin` output
//! from asl 1.42 Beta Bld 212 (`s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_ref.sh`'s `asl_run` with
//! `-xx -n -q -A -L -U -i .`), from a run that exited 0. Every expected refusal
//! is a probe asl refused: `#1010 symbol undefined` unless the test says
//! `#1000 symbol double defined`.
//!
//! Every probe sits at `org $100` behind a `$1111` word, and each candidate a
//! reference could bind carries its own word (`$5555` Base, `$6666` Base.x,
//! `$7777` a file-level Inner, `$8888` its `.x`), so the bytes say WHICH name
//! bound, not only that one did.

use sigil_frontend_as::{assemble, Options};
use std::path::PathBuf;

fn probe_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-12-as-macro-dot-scope-after-call/probes")
}

/// Assemble and link one probe. `Ok` is the image from `$100` on, `Err` the
/// messages of whichever stage refused it.
fn run(name: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = probe_dir();
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
    match run(name) {
        Ok(b) => assert_eq!(b, words(asl), "{name}: asl emits {asl}"),
        Err(m) => panic!("{name}: asl assembles it to {asl}, sigil refused: {m:?}"),
    }
}

#[track_caller]
fn refused(name: &str, fragment: &str) {
    match run(name) {
        Ok(b) => panic!("{name}: asl refuses it, sigil built {b:02x?}"),
        Err(m) => assert!(
            m.iter().any(|x| x.contains(fragment)),
            "{name}: refused, but no message mentions `{fragment}`: {m:?}"
        ),
    }
}

/// After a call whose body wrote `Inner:`, `.b := 2` is `Inner.b`, and
/// `Base.b`, the caller's own scope, does not exist. Every spelling of the body
/// label does it, the LAST one written wins, `Inner` itself is still out of
/// reach (`a06`), and the answer holds when an unrelated forward reference
/// forces more passes (`f01`).
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: before this row every `read_inner`
/// shape was refused and every `read_base` shape built, because the expansion
/// handed the caller's entry scope back on the way out.
#[test]
fn a_plain_label_in_a_macro_body_is_the_callers_dot_scope_after_the_call() {
    builds("a01_bind_read_inner", "1111 5555 2222 0002 4444");
    refused("a02_bind_read_base", "unresolved");
    builds("a04_label_read_inner", "1111 5555 2222 6666 0106 4444");
    refused("a05_label_read_base", "unresolved");
    builds("a07_twice", "1111 5555 2222 2222 0002 4444");
    builds("a08_no_base", "1111 2222 0002 4444");
    builds("c08_taken_if_read_inner", "1111 5555 2222 0002 4444");
    builds("f01_forced_passes_read_inner", "1111 010a 5555 2222 0002 9999 4444");
    for tag in ["colon_alone", "bare_alone", "bare_data", "indented_colon"] {
        builds(&format!("l_{tag}_read_inner"), "1111 5555 2222 0002 4444");
        refused(&format!("l_{tag}_read_base"), "unresolved");
    }
    builds("t01_two_read_second", "1111 5555 2222 2223 0002 4444");
    refused("t02_two_read_first", "unresolved");
    refused("t03_two_read_base", "unresolved");
}

/// The shapes the row was booked for: a `.x` whose meaning crosses the call.
/// Where both assemblers build, the bytes differ unless the scope moved, and
/// no diagnostic anywhere says so.
///
/// `s02`/`s03`: `.x` after the call is the FILE-level `Inner.x` (`$0104`,
/// `$010E`), not the caller's `Base.x`. `s06`: the `:=` twin, 9 not 3. `s07`,
/// `s08`: `defined(.x)` and `ifdef .x` are FALSE after the call, so the else
/// arm's `$BBBB` is assembled. `s12`: a second `.x:` after the call is a NEW
/// name (`Inner.x`), not a redefinition of `Base.x`. `s11`, `s13`: and it
/// collides with any other `Inner.x`, including one a second caller of the
/// same macro writes (`#1000`).
#[test]
fn a_dot_local_that_crosses_the_call_binds_the_name_asl_binds() {
    refused("s01_dot_before_read_after", "unresolved");
    builds("s02_dot_before_read_after_file_inner_before", "1111 7777 8888 5555 6666 2222 0104 4444");
    builds("s03_dot_before_read_after_file_inner_after", "1111 5555 6666 2222 010e 4444 7777 8888 4444");
    builds("s04_dot_before_read_base_qualified", "1111 5555 6666 2222 0104 4444");
    refused("s05_set_before_read_after", "unresolved");
    builds("s06_set_before_read_after_file_inner", "1111 7777 5555 2222 0009 4444");
    builds("s07_defined_after", "1111 5555 6666 2222 bbbb 4444");
    builds("s08_ifdef_after", "1111 5555 6666 2222 bbbb 4444");
    refused("s09_forward_ref_before_call", "unresolved");
    refused("s10_forward_ref_before_call_base_x_later", "unresolved");
    refused("s11_redefine_after_call_file_inner", "double defined");
    builds("s12_redefine_after_call_base_x", "1111 5555 6666 2222 6667 4444");
    refused("s13_two_callers_same_local", "double defined");
}

/// Inside the body, after its plain label, a value binding and a reference to
/// a `.`-local the body does not define both use that label (`v01`..`v04`),
/// while the same lines written BEFORE it use the caller's scope (`v05`,
/// `v06`). A `.lp:` the body wrote before its plain label is not reachable as
/// `.lp` after it (`v08`), and one written after it is (`v07`).
#[test]
fn inside_the_body_a_binding_or_reference_after_its_plain_label_uses_that_label() {
    builds("v01_body_bind_read_inner", "1111 5555 2222 0005 4444");
    refused("v02_body_bind_read_base", "unresolved");
    refused("v03_body_reads_caller_dot", "unresolved");
    builds("v04_body_reads_caller_dot_file_inner", "1111 7777 8888 5555 6666 2222 0104 4444");
    builds("v05_body_bind_before_label_read_base", "1111 5555 2222 0005 4444");
    builds("v06_body_reads_caller_dot_before_label", "1111 5555 6666 0104 2222 4444");
    builds("v07_body_own_dot_after_label", "1111 5555 2222 2223 0106 4444");
    refused("v08_body_dot_before_label_read_after", "unresolved");
}

/// A nested body's plain label moves the scope of the enclosing body for the
/// rest of it, and of the caller after the whole nest (`n01`, `n13`). The
/// enclosing body's own later label moves it again (`n04`). A `.`-label the
/// enclosing body writes after the nested call is qualified by the nested
/// label (`n12` reads it as `Inner.y`, `n11` cannot read it as `Outer.y`) but
/// stays in the enclosing expansion (`n09`), and one written BEFORE the nested
/// call is out of reach as `.lp` after it: refused (`n14`), or, with a
/// file-level `Inner.lp` present, that one (`n15`, `$0104`, silent before
/// this row).
#[test]
fn a_nested_body_label_moves_the_scope_of_every_body_it_returns_to() {
    builds("n01_inner_writes_read_inner", "1111 5555 2222 0002 4444");
    refused("n02_inner_writes_read_base", "unresolved");
    builds("n03_outer_writes_then_inner_plain", "1111 5555 3333 2222 0002 4444");
    builds("n04_inner_then_outer_label_read_outer2", "1111 5555 2222 3333 0002 4444");
    refused("n05_inner_then_outer_label_read_inner", "unresolved");
    builds("n06_outer_binds_after_inner_read_inner", "1111 5555 2222 0003 4444");
    refused("n07_outer_binds_after_inner_read_base", "unresolved");
    builds("n08_outer_label_after_inner_read_in_body", "1111 5555 2222 3333 0106 4444");
    refused("n09_outer_label_after_inner_read_inner_y_outside", "unresolved");
    refused("n10_outer_reads_caller_dot_after_inner", "unresolved");
    refused("n11_outer_label_then_inner_then_dot_read_outer_y", "unresolved");
    builds("n12_outer_label_then_inner_then_dot_read_inner_y", "1111 5555 3333 2222 3334 0108 4444");
    builds("n13_three_deep", "1111 5555 2222 0002 4444");
    refused("n14_outer_dot_before_inner_read_after", "unresolved");
    builds("n15_outer_dot_before_inner_read_after_file_inner", "1111 7777 8888 5555 3333 2222 0104 4444");
    builds("n16_inner_label_directive_outer_dot_read_qualified", "1111 5555 3333 0104 4444");
    builds("n17_outer_forward_dot_after_inner", "1111 5555 2222 6002 3333 3334 4444");
    builds("n18_outer_dot_after_inner_twice", "1111 5555 2222 3333 0106 2222 3333 010c 3335 4444");
}

/// A body label written inside a `rept` or `irp` iteration moves the scope the
/// same way, at file level and inside a macro body (`r08`: the last item's
/// substituted name). A `.y:` the macro body writes after such a loop stays in
/// the macro's expansion (`r11`) and reads back inside it (`r12`).
#[test]
fn a_body_label_inside_a_loop_iteration_moves_the_scope_too() {
    builds("r01_file_rept_read_inner", "1111 5555 2222 0002 4444");
    refused("r02_file_rept_read_base", "unresolved");
    builds("r03_macro_rept_read_inner", "1111 5555 2222 0002 4444");
    refused("r04_macro_rept_read_base", "unresolved");
    builds("r05_file_irp_read_inner", "1111 5555 2222 0002 4444");
    builds("r06_macro_irp_read_inner", "1111 5555 2222 0002 4444");
    refused("r07_macro_irp_read_base", "unresolved");
    builds("r08_macro_irp_var_label_read_last", "1111 5555 2222 2222 0002 4444");
    builds("r09_macro_rept2_read_inner", "1111 5555 2222 2222 0002 4444");
    refused("r10_file_rept_dot_before_read_after", "unresolved");
    refused("r11_macro_rept_label_then_dot_read_outside", "unresolved");
    builds("r12_macro_rept_label_then_dot_read_in_body", "1111 5555 2222 2223 0106 4444");
}

/// `{GLOBALSYMBOLS}` and `{INTLABEL}` do not change the rule. A
/// `__LABEL__:` body label is the invocation's label text (`g03`); a call
/// line's own label opens the scope first and the body's label replaces it
/// (`g07`, `g08`); a `{GLOBALSYMBOLS}` frame nested with a plain one, either
/// way round, carries the label out through both (`g10`..`g13`).
#[test]
fn the_option_twins_and_a_labelled_call_follow_the_same_rule() {
    builds("g01_globalsymbols_read_inner", "1111 5555 2222 0002 4444");
    refused("g02_globalsymbols_read_base", "unresolved");
    builds("g03_intlabel_label_read_tbl", "1111 5555 2222 0002 4444");
    refused("g04_intlabel_label_read_base", "unresolved");
    builds("g05_intlabel_inner_read_inner", "1111 5555 2222 0002 4444");
    refused("g06_intlabel_inner_read_tbl", "unresolved");
    builds("g07_call_label_body_inner_read_inner", "1111 5555 2222 0002 4444");
    refused("g08_call_label_body_inner_read_lbl", "unresolved");
    builds("g09_call_label_no_body_label_read_lbl", "1111 5555 2222 0002 4444");
    builds("g10_plain_outer_gs_inner_read_inner", "1111 5555 2222 0002 4444");
    builds("g11_gs_outer_plain_inner_read_inner", "1111 5555 2222 0002 4444");
    refused("g12_plain_outer_gs_inner_read_base", "unresolved");
    refused("g13_gs_outer_plain_inner_read_base", "unresolved");
}

/// The controls: a body that writes no plain label, or writes one on a line
/// that does not run (`c06` untaken `if`, `c07` after `exitm`), leaves the
/// caller's scope where it was; a `.q:` body label is not a scope (`c03`); the
/// `label` directive and `set` already opened the caller's scope (`c04`,
/// `c05`); and a `.`-local defined and read in one scope agrees whichever
/// scope it is (`a03`, `f02`).
#[test]
fn a_body_that_writes_no_plain_label_leaves_the_callers_scope_alone() {
    builds("a03_bind_read_dot", "1111 5555 2222 0002 4444");
    refused("a06_inner_itself", "unresolved");
    builds("c01_no_label_read_base", "1111 5555 2222 0002 4444");
    builds("c02_no_label_read_dot_before", "1111 5555 6666 2222 0104 4444");
    builds("c03_dot_label_body_read_base", "1111 5555 2222 0002 4444");
    builds("c04_label_directive_read_inner", "1111 5555 0002 4444");
    builds("c05_set_body_read_vv", "1111 5555 0002 4444");
    builds("c06_untaken_if_read_base", "1111 5555 0002 4444");
    builds("c07_exitm_before_label_read_base", "1111 5555 2222 0002 4444");
    builds("f02_forced_passes_dot_forward", "1111 010c 5555 2222 010a 6666 9999 4444");
}
