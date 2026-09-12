//! A name written inside an expansion instance (a macro expansion, or one
//! iteration of a `rept` / `irp` / `irpc` / `while`) belongs to THAT instance,
//! whatever spelling put it there, and is out of reach once the instance ends.
//!
//! Row AS-MACRO-LABEL-LEAK. Every fixture here IS a probe file asl assembled:
//! the source is read from `docs/superpowers/notes/2026-09-12-as-macro-label-leak/`
//! at test time, so the text sigil is tested on cannot drift from the text the
//! oracle answered. Every expected byte string is that probe's `p2bin` output
//! from asl 1.42 Beta Bld 212 (`s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_ref.sh`'s `asl_run` with
//! `-xx -n -q -A -L -U -i .`), from a run that exited 0. Every expected refusal
//! is a probe asl refused, with `#1010 symbol undefined` unless noted.
//!
//! Every probe sits at `org $100` behind a `$1111` word, so a label that binds
//! reads as a non-zero address: a refusal cannot hide as a zero, and "resolves"
//! and "resolves to the other expansion" are different bytes.

use sigil_frontend_as::{assemble, Options};
use std::path::PathBuf;

fn probe_dir(batch: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/superpowers/notes/2026-09-12-as-macro-label-leak")
        .join(batch)
}

/// Assemble and link one probe. `Ok` is the image from `$100` on, `Err` the
/// messages of whichever stage refused it. The probe's own directory is the
/// include root, which is where its `.inc` companions live.
fn run(batch: &str, name: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = probe_dir(batch);
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
fn builds(batch: &str, name: &str, asl: &str) {
    match run(batch, name) {
        Ok(b) => assert_eq!(b, words(asl), "{batch}/{name}: asl emits {asl}"),
        Err(m) => panic!("{batch}/{name}: asl assembles it to {asl}, sigil refused: {m:?}"),
    }
}

#[track_caller]
fn refused(batch: &str, name: &str, fragment: &str) {
    match run(batch, name) {
        Ok(b) => panic!("{batch}/{name}: asl refuses it, sigil built {b:02x?}"),
        Err(m) => assert!(
            m.iter().any(|x| x.contains(fragment)),
            "{batch}/{name}: refused, but no message mentions `{fragment}`: {m:?}"
        ),
    }
}

/// A `-` or `/` written in a body is filed in that expansion, so a reference
/// after the expansion returned (or from a sibling, or from the outer body
/// once a nested one returned) names a slot nobody live can see.
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: every one of them built before
/// this row, to the body's own address, because the slot was global. `c08`
/// is the sharpest: an OUTER `-` sits one slot below the body's, so a rule that
/// merely skipped invisible slots would reach it and emit `$0102`; asl refuses.
#[test]
fn a_nameless_label_written_in_a_macro_body_is_out_of_reach_after_the_expansion() {
    refused("probes", "c01_minus_after", "nameless");
    refused("probes", "c04_slash_then_minus", "nameless");
    refused("probes", "c08_minus_outer_before_body", "nameless");
    refused("probes", "c15_minus_rept_in_macro", "nameless");
    refused("probes", "i03_include_nameless", "nameless");
    refused("probes2", "n03_outer_back_to_closed_inner", "nameless");
    refused("probes2", "n10_other_macro_later", "nameless");
    refused("probes2", "n20_if_in_body", "nameless");
}

/// The counters are global, so a reference made before a call names the slot
/// the body will fill, and that slot is the body's. The accepted pair shows the
/// counter really is shared: `n22`'s `++` and `n23`'s `--` skip the body's slot
/// and land on the outer definition beyond it.
#[test]
fn a_nameless_reference_made_before_a_call_cannot_reach_a_definition_in_the_body() {
    refused("probes", "c02_plus_before", "nameless");
    refused("probes", "c03_plus_before_outer_after", "nameless");
    refused("probes", "c05_plus_before_slash", "nameless");
    refused("probes", "c14_plus_before_twice", "nameless");
    refused("probes2", "n02_outer_fwd_to_inner_def", "nameless");
    refused("probes2", "n13_inner_fwd_past_closed_nested", "nameless");
    builds("probes2", "n22_fwd_outside_skips_body", "1111 0106 2222 3333 4444");
    builds("probes2", "n23_back_outside_skips_body", "1111 3333 2222 0102 4444");
}

/// The other direction is open: a body (or a nested expansion, or a loop inside
/// one) reaches a definition written outside it, before or after, and an
/// argument carrying a nameless reference means what it means where the body
/// evaluates it (`n12`: the body's own `-`, `$0104`, not the file's `$0102`).
#[test]
fn a_macro_body_reaches_nameless_definitions_outside_it_in_both_directions() {
    builds("probes", "c06_inside_reads_outer_minus", "1111 3333 0102 4444");
    builds("probes", "c07_inside_reads_outer_plus", "1111 0104 3333 4444");
    builds("probes", "c09_own_minus_control", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "n01_fwd_inside_own", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "n04_inner_back_to_outer_def", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "n05_inner_fwd_to_outer_def", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "n11_ref_in_arg", "1111 3333 0102 4444");
    builds("probes2", "n12_ref_in_arg_body_def", "1111 3333 2222 0104 4444");
    builds("probes2", "n16_back_from_nested_child", "1111 2222 0102 4444");
    builds("probes2", "n17_fwd_child_ref_parent_def", "1111 0104 2222 4444");
    builds("probes2", "n18_slash_in_body_back_inside", "1111 2222 0102 2222 0106 4444");
}

/// A loop iteration is an instance too, at file level as well as in a macro,
/// and a nameless label written on the loop's OWN line is outside the body
/// (`n09`, the corpus's `-\trept N` / `dbf d0,-` shape).
#[test]
fn a_nameless_label_in_a_loop_iteration_belongs_to_that_iteration() {
    refused("probes", "c10_minus_rept_file", "nameless");
    refused("probes", "c11_minus_irp_file", "nameless");
    refused("probes", "c12_minus_while_file", "nameless");
    builds("probes2", "n06_rept_fwd_in_iter", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "n07_rept_back_in_iter", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "n08_rept_reads_outer", "1111 3333 0102 0102 4444");
    builds("probes2", "n09_label_on_rept_line", "1111 7001 4e71 4e71 51c8 fffa 4444");
    builds("probes2", "n14_while_fwd_in_iter", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "n15_irp_back_in_iter", "1111 0001 0102 0002 0106 4444");
}

/// A label whose name comes from `__LABEL__`, `{expr}`, a parameter or
/// `ALLARGS` is still a label written in the body. The body text does not
/// spell the name, so a scan of the text cannot claim it; the label is filed
/// in the instance that writes it all the same.
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: `$0102`, the body's address,
/// which every one of them built to before this row.
#[test]
fn a_label_whose_name_the_body_does_not_spell_stays_in_its_expansion() {
    refused("probes", "g01_intlabel_suffix", "Aint_Blocks");
    refused("probes", "g02_intlabel_bare", "Aint");
    refused("probes", "g04_brace_global", "Lab7");
    refused("probes", "g05_brace_param", "Lab3");
    refused("probes", "g06_param_name_colon", "Foo");
    refused("probes", "g07_param_name_bare", "Foo");
    refused("probes", "g08_param_name_dot", ".foo");
    refused("probes", "g14_allargs_name", "Foo");
    refused("probes2", "s06_param_nested_arg", "Foo");
    refused("probes2", "s07_param_nested_outside", "Foo");
}

/// The same labels read from INSIDE the body that writes them, including a
/// forward read that runs before the definition. This is the half a rule that
/// only filed the definition would break: each expansion is at a different
/// address, so a read reaching the other expansion's copy is a different byte.
#[test]
fn a_substituted_label_reads_back_inside_its_own_body_before_and_after_it() {
    builds("probes2", "s01_brace_fwd_inside", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "s02_param_fwd_inside", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "s03_intlabel_fwd_inside", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "s04_brace_global_fwd_inside", "1111 0104 2222 4444");
    builds("probes2", "s05_brace_changes_in_body", "1111 2222 0102 4444");
    builds("probes", "g12_param_name_inside", "1111 2222 0102 2222 0106 4444");
    builds("probes", "g13_brace_inside", "1111 2222 0102 2222 0106 4444");
    builds("probes", "g11_intlabel_suffix_twice", "1111 2222 2222 3333 4444");
}

/// Two expansions writing the same substituted name each own a copy, so asl
/// does not collide them. Before this row sigil refused both as a linker
/// redefinition.
#[test]
fn two_expansions_may_write_the_same_substituted_label() {
    builds("probes", "g09_param_name_twice", "1111 2222 2222 3333 4444");
    builds("probes", "g10_brace_twice", "1111 2222 2222 3333 4444");
}

/// `enum` and `nextenum` members follow the label rule, in a macro body, in a
/// loop iteration (`en5`) and in a file included from a body (`en6`). The
/// running counter is NOT scoped: a file-level `nextenum` after the call
/// continues from the body's last member (`en4`, `$0006`).
#[test]
fn an_enum_member_written_in_a_body_or_a_loop_stays_there() {
    refused("probes", "h01_enum", "Eb");
    refused("probes", "h04_nextenum", "Eb");
    refused("probes3", "en5_enum_in_rept", "Eb");
    refused("probes3", "en6_enum_in_include_in_body", "Eb");
    builds("probes2", "en1_inside", "1111 0006 4444");
    builds("probes2", "en2_twice", "1111 3333 4444");
    builds("probes2", "en3_file_enum_then_body_nextenum_inside", "1111 0006 4444");
    builds("probes2", "en4_enum_counter_continues", "1111 0006 4444");
}

/// An `include` written in a body splices into THAT expansion, so its labels
/// are the expansion's: private to it (`i01`, `in6` in a loop), private per
/// expansion (`i02`, `in1`, `in2`), and in the same namespace as the body's own
/// labels in both directions (`in3`, `in4`).
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: before this row an include cleared
/// the namespace stack, so `i01` built to `$0102`, the twice-invoked shapes were
/// `symbol double defined`, and `in3` could not see the body's `Lb`.
#[test]
fn a_label_in_a_file_included_from_a_body_belongs_to_that_expansion() {
    refused("probes", "i01_include_in_body", "Linc");
    refused("probes2", "in6_include_in_rept", "Linc");
    builds("probes", "i02_include_in_body_twice", "1111 2222 2222 3333 4444");
    builds("probes2", "in1_inside_after", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "in2_inc_refs_own", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "in3_inc_reads_caller_body", "1111 2222 0102 2222 0106 4444");
    builds("probes2", "in4_inc_fwd_inside", "1111 0104 2222 0108 2222 4444");
    builds("probes2", "in5_file_include_control", "1111 2222 0102 4444");
}

/// `{GLOBALSYMBOLS}` opens no namespace: the body's plain, `.`-local and
/// nameless labels land wherever the same lines written in the caller would.
/// At file level that is global (readable after, forward from before, and a
/// second expansion collides). Inside a plain expansion it is THAT expansion's
/// (`gs1` reads it in the outer body, `gs2` cannot at file level, `gs3` two
/// outer expansions do not collide), and a plain macro or a loop inside it
/// still opens its own (`gs10`, `gs4`).
#[test]
fn a_globalsymbols_expansion_opens_no_namespace_of_its_own() {
    builds("probes", "f_globalsymbols", "1111 2222 0102 4444");
    builds("probes", "f_globalsymbols_forward", "1111 0104 2222 4444");
    builds("probes", "f_globalsymbols_lower", "1111 2222 0102 4444");
    builds("probes", "f_globalsymbols_dot", "1111 5555 2222 0104 4444");
    builds("probes", "f_globalsymbols_inside", "1111 2222 0102 4444");
    builds("probes", "f_globalsymbols_intlabel", "1111 2222 0102 4444");
    builds("probes", "c13_minus_globalsymbols", "1111 2222 0102 4444");
    builds("probes2", "n19_globalsymbols_fwd_from_outside", "1111 0104 2222 4444");
    builds("probes2", "gs1_global_inner_read_in_outer", "1111 2222 0102 4444");
    builds("probes2", "gs3_global_inner_twice_in_outer", "1111 2222 2222 3333 4444");
    builds("probes2", "gs5_global_body_scope_after", "1111 5555 2222 0002 4444");
    builds("probes2", "gs6_global_body_dot_then_base", "1111 5555 2222 0104 4444");
    builds("probes3", "gs11_global_value_binding_dot", "1111 5555 0007 4444");
    refused("probes2", "gs2_global_inner_read_outside", "Lg");
    refused("probes2", "gs4_rept_in_global_body", "Lr");
    refused("probes3", "gs10_plain_in_global_outer", "Ln");
    refused("probes3", "gs9_global_dot_in_plain_outer_read_outside", "Base.dl");
    refused("probes", "f_globalsymbols_nested_inner", "Ln");
    // asl `#1000 symbol double defined`, both, and sigil answers in the FRONT
    // END with the same words. The fragment is deliberately not just "defined":
    // the linker's `redefined by section` would satisfy that, and it is what a
    // front end that forgot the name was global would fall through to.
    refused("probes", "f_globalsymbols_twice", "double defined");
    refused("probes2", "gs7_global_body_dot_twice", "double defined");
}

/// Transparency is the same rule NESTED as at file level: a `{GLOBALSYMBOLS}`
/// body inside a plain expansion reads its own `.dl` back per expansion
/// (`gs13`) and its `.v :=` still reaches the real caller scope (`gs12`); a
/// plain macro called from a file-level `{GLOBALSYMBOLS}` body is the
/// OUTERMOST plain one, so its `.v :=` (`gs14`) and its `label` directive's
/// scope (`gs15`) reach the caller exactly as a direct call's would.
///
/// WHAT OTHER ANSWER COULD THESE HAVE GIVEN: `gs13` refused, had the `.dl`
/// read looked in the caller's scope while the write went to the plain
/// expansion's; `gs14` and `gs15` refused, had the plain frame under a
/// transparent one not recorded the caller's scope as its real scope.
#[test]
fn a_globalsymbols_expansion_nested_with_a_plain_one_is_transparent_there_too() {
    builds("probes4", "gs12_value_binding_in_global_in_plain", "1111 5555 0007 4444");
    builds("probes4", "gs13_dot_read_inside_global_in_plain", "1111 5555 2222 0104 2222 0108 4444");
    builds("probes4", "gs14_plain_value_binding_in_global", "1111 5555 0007 4444");
    builds("probes4", "gs15_plain_label_dir_in_global", "1111 5555 0003 4444");
}

/// A `.x` written under a plain label of the body is as private as the label:
/// `Lp.x` from outside is `#1010` (`a10`), two expansions do not collide
/// (`dx2`), and inside the body it answers to all three spellings: `.x`,
/// `Lp.x` (`dx3`), and `.x` handed to a nested macro as an argument (`dx5`).
/// A file-level loop's label is not a macro scope and keeps the plain rule
/// after the loop (`dx4`).
#[test]
fn a_dot_label_under_a_body_label_stays_in_the_expansion() {
    refused("probes", "a10_dot_under_body_label", "Lp.x");
    builds("probes2", "dx1_two_scopes_one_body", "1111 2222 0104 3333 0108 4444");
    builds("probes2", "dx2_twice", "1111 2222 3333 2222 3333 5555 4444");
    builds("probes2", "dx3_read_qualified_inside", "1111 2222 3333 0104 4444");
    builds("probes3", "dx4_body_label_scope_after_rept", "1111 2222 3333 0104 4444");
    builds("probes3", "dx5_body_label_dot_arg_to_nested", "1111 2222 3333 0104 4444");
}

/// The shapes this row must NOT move: the value-binding forms stay global, the
/// definedness probes still say "not defined" after the expansion, and every
/// refusal the earlier parcels pinned stays a refusal.
#[test]
fn the_shapes_asl_and_sigil_already_agreed_on_stay_agreed() {
    builds("probes", "h02_equ_control", "1111 0123 4444");
    builds("probes", "h03_label_dir_control", "1111 0102 4444");
    builds("probes2", "s08_param_value_binding", "1111 0123 4444");
    builds("probes2", "s09_param_label_dir", "1111 0102 4444");
    builds("probes", "a08_defined_fn", "1111 2222 bbbb 4444");
    builds("probes", "a09_ifdef", "1111 2222 bbbb 4444");
    for name in [
        "a01_colon",
        "a02_bare_col0",
        "a03_bare_alone",
        "a04_indented_colon",
        "a05_forward",
        "a06_later_other_macro",
        "a07_earlier_other_macro",
        "a13_twice_then_outside",
        "d01_rept1",
        "d02_rept2",
        "d03_irp",
        "d05_while",
        "d06_irp_var_name",
        "d08_rept_in_macro_read_outside",
        "e01_inner_label_outside",
        "e02_outer_label_outside",
        "e03_macro_defined_in_macro",
        "f_expand",
        "f_noexpand",
        "f_expif",
        "f_noexpif",
        "f_expmacro",
        "f_noexpmacro",
        "f_export",
        "f_noexport",
        "f_intlabel",
        "f_noglobalsymbols",
    ] {
        refused("probes", name, "unresolved");
    }
    refused("probes", "d07_rept_in_macro_read_in_body", "Lr");
    refused("probes", "g03_intlabel_literal", "__LABEL__Plc");
    for name in ["b01_dot_colon", "b02_dot_qualified", "b03_dot_no_scope", "b04_dot_bare_col0", "b05_dot_forward"] {
        refused("probes", name, "dl");
    }
    refused("probes2", "d04b_irpc", "Lc");
    refused("probes2", "d09b_irpc_var_name", "Q");
}
