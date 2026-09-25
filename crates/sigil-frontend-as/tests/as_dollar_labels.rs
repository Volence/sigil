//! `$$name`: AS's NAMED TEMPORARY symbols.
//!
//! Sonic 3 & Knuckles writes its short loops with them (`$$compareChars:`,
//! `$$loop:`, `$$done:`), 93 lines of `sonic3k.asm`, and reuses each name in
//! routine after routine. A `$$name` is keyed by the most recent NON-temporary
//! symbol the pass wrote, by that symbol's NAME, and nothing else bounds it.
//!
//! ## The rules, each measured
//!
//! - Two routines each own their `$$x`, read backward and forward.
//! - A plain label, `equ`, `set`, `:=`, `=`, the `label` directive, an `enum`
//!   member and `endstruct` end the scope; so do `cpu` (`PADDING` for a 68000,
//!   `INLWORDMODE` for a Z80), `padding` (`PADDING`), `supmode` (`INSUPMODE`),
//!   `listing` (`LISTON`) and `restore` (`MACEXP`), which write asl's own symbols.
//! - A `.`-local, a nameless label, a `$$` binding, `save`, `page`, `charset`,
//!   `phase`, `org`, `if`, `rept`, a macro definition and a macro call whose body
//!   writes no label do not.
//! - The scope is the NAME: `V set 2` after `V set 1` returns to `V`'s `$$` names,
//!   and `cpu 68000` then `padding off` share `PADDING`.
//! - A binder reads its right-hand side in the scope above it.
//! - A PC `$$x:` in a macro body or loop iteration is that instance's; a body
//!   reads a caller's `$$x` when it has filed none of its own in that scope, and
//!   a body label stays the scope after the call. A `$$` value binding in a body
//!   is global, as a plain one is.
//! - Names are case sensitive, take `_` and `.` in the tail, and work under
//!   `cpu z80` as under `cpu 68000`; `$$`, `$$1` and `$$.x` are not names.
//! - `DEFINED($$x)` is 0 and `ifdef $$x` false, bound or not.
//!
//! ## Provenance
//!
//! Every expected byte string is asl's own image, never computed:
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, through
//! `docs/superpowers/notes/asl-reference/asl_ref.sh`'s `asl_run -xx -n -q -A -L -U
//! -i .`, exit 0 with the pass loop complete, then its `p2bin -p=0`, as hex from
//! `$1200`, where every probe orgs. The probe sources are the `*_SRC` constants
//! verbatim and `docs/superpowers/notes/2026-09-25-s3k-dollar-labels.md` tables
//! them. A refusal probe is one asl exited 2 on (`#1010 symbol undefined`,
//! `#1000 symbol double defined`, `#1020 invalid symbol name`); no byte of those
//! runs is used, and each needle names the key sigil built, so the refusal is
//! shown to come from this scoping and not from a neighbouring path.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        // An undefined name surfaces at the link, as it does in the CLI, so the
        // layout and link diagnostics are refusals here too.
        Ok(m) => {
            let msgs = |d: Vec<sigil_span::Diagnostic>| -> Vec<String> { d.into_iter().map(|d| d.message).collect() };
            let stubs = sigil_ir::SymbolTable::new();
            let resolved = sigil_link::resolve_layout(&m.sections, &stubs, true).map_err(msgs)?;
            let linked = sigil_link::link(&resolved, &stubs).map_err(msgs)?;
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

/// Assemble `src` and require asl's bytes from `$1200`, and zeros below it.
fn assert_asl(src: &str, asl_hex: &str) {
    let got = match assemble(src) {
        Ok(b) => b,
        Err(d) => panic!("expected asl's bytes {asl_hex}, got diagnostics: {d:?}"),
    };
    assert!(got.len() > 0x1200, "image ends at {:#x}, below $1200", got.len());
    assert!(got[..0x1200].iter().all(|&b| b == 0), "nonzero byte below $1200");
    let got_hex: String = got[0x1200..].iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(got_hex, asl_hex, "sigil's image from $1200 differs from asl's");
}

/// Assemble `src` and require a refusal whose text contains `needle`.
fn assert_refused(src: &str, needle: &str) {
    let d = match assemble(src) {
        Ok(b) => panic!("expected a refusal naming {needle:?}, got {} bytes", b.len()),
        Err(d) => d,
    };
    let all = d.join("\n");
    assert!(all.contains(needle), "refusal does not name {needle:?}:\n{all}");
}

#[test]
fn two_routines_each_own_their_names_backward_and_forward() {
    assert_asl(D01_SRC, D01_ASL);
    assert_asl(D09_SRC, D09_ASL);
}

#[test]
fn a_plain_label_ends_the_scope() {
    assert_refused(D04_SRC, "unresolved symbol `$$x@A1`");
    assert_refused(D10_SRC, "unresolved symbol `$$x@B1`");
    assert_refused(I16_SRC, "unresolved symbol `$$x@A1`");
    assert_refused(F16_SRC, "unresolved symbol `$$x@B1`");
}

#[test]
fn a_value_binder_ends_the_scope_and_reads_its_own_operand_in_the_old_one() {
    assert_refused(D05_SRC, "unresolved symbol `$$x@A2`");
    assert_refused(D06_SRC, "unresolved symbol `$$x@A2`");
    assert_refused(G10_SRC, "unresolved symbol `$$x@V`");
    assert_asl(E06_SRC, E06_ASL);
}

#[test]
fn the_scope_is_the_name_so_a_set_symbol_returns_to_its_names() {
    assert_asl(F01_SRC, F01_ASL);
    assert_asl(J06_SRC, J06_ASL);
    assert_refused(E01_SRC, "symbol double defined: `$$x@V`");
}

#[test]
fn locals_nameless_labels_and_non_symbol_directives_keep_the_scope() {
    assert_asl(D02_SRC, D02_ASL);
    assert_asl(D03_SRC, D03_ASL);
    assert_asl(E05_SRC, E05_ASL);
    assert_asl(E12_SRC, E12_ASL);
    assert_asl(E13_SRC, E13_ASL);
    assert_asl(E19_SRC, E19_ASL);
    assert_asl(F13_SRC, F13_ASL);
    assert_asl(F19_SRC, F19_ASL);
    assert_asl(F20_SRC, F20_ASL);
    assert_asl(G05_SRC, G05_ASL);
    assert_asl(G07_SRC, G07_ASL);
}

#[test]
fn a_temporary_label_opens_no_local_scope() {
    assert_asl(E07_SRC, E07_ASL);
}

#[test]
fn directives_that_write_asl_symbols_end_the_scope() {
    assert_refused(E08_SRC, "unresolved symbol `$$x@PADDING`");
    assert_refused(G20_SRC, "unresolved symbol `$$x@PADDING`");
    assert_refused(I26_SRC, "unresolved symbol `$$x@PADDING`");
    assert_refused(F07_SRC, "unresolved symbol `$$x@PADDING`");
    assert_refused(G11_SRC, "unresolved symbol `$$x@PADDING`");
    assert_refused(F08_SRC, "unresolved symbol `$$x@INSUPMODE`");
    assert_refused(G12_SRC, "unresolved symbol `$$x@INSUPMODE`");
    assert_refused(F09_SRC, "unresolved symbol `$$x@LISTON`");
    assert_refused(G13_SRC, "unresolved symbol `$$x@LISTON`");
    assert_refused(F12_SRC, "unresolved symbol `$$x@MACEXP`");
}

#[test]
fn directives_ending_on_the_same_symbol_share_a_scope() {
    assert_asl(I01_SRC, I01_ASL);
    assert_refused(G02_SRC, "symbol double defined: `$$x@PADDING`");
    assert_refused(G04_SRC, "symbol double defined: `$$x@PADDING`");
}

#[test]
fn enum_members_and_endstruct_end_the_scope() {
    assert_refused(G06_SRC, "unresolved symbol `$$x@E2`");
    assert_refused(G16_SRC, "unresolved symbol `$$x@E3`");
    assert_refused(G14_SRC, "unresolved symbol `$$x@Str_len`");
}

#[test]
fn temporary_value_bindings_are_scoped_and_end_nothing() {
    assert_asl(D08_SRC, D08_ASL);
    assert_asl(Y01_SRC, Y01_ASL);
    assert_asl(Y03_SRC, Y03_ASL);
    assert_asl(Y04_SRC, Y04_ASL);
    assert_asl(I05_SRC, I05_ASL);
}

#[test]
fn a_macro_body_reads_the_callers_names_and_keeps_its_own() {
    assert_asl(E02_SRC, E02_ASL);
    assert_asl(E11_SRC, E11_ASL);
    assert_asl(F02_SRC, F02_ASL);
    assert_asl(F03_SRC, F03_ASL);
    assert_asl(F11_SRC, F11_ASL);
    assert_asl(F14_SRC, F14_ASL);
    assert_asl(G17_SRC, G17_ASL);
    assert_asl(I27_SRC, I27_ASL);
    assert_asl(I30_SRC, I30_ASL);
    assert_refused(F10_SRC, "unresolved symbol `$$q@A1`");
}

#[test]
fn a_nested_body_reads_the_enclosing_instance() {
    assert_asl(I25_SRC, I25_ASL);
}

#[test]
fn a_body_label_is_still_the_scope_after_the_call() {
    assert_refused(E03_SRC, "unresolved symbol `$$x@Lb`");
    assert_refused(G01_SRC, "symbol double defined: `$$z@Lb`");
    assert_asl(I28_SRC, I28_ASL);
}

#[test]
fn body_ownership_is_by_scope_not_by_spelling() {
    assert_asl(K02_SRC, K02_ASL);
    assert_refused(K06_SRC, "unresolved symbol `$$q@Lab1`");
    assert_asl(K07_SRC, K07_ASL);
}

#[test]
fn a_value_binding_in_a_body_is_global() {
    assert_asl(K03_SRC, K03_ASL);
    assert_asl(K04_SRC, K04_ASL);
}

#[test]
fn each_loop_iteration_owns_its_names() {
    assert_asl(I02_SRC, I02_ASL);
}

#[test]
fn names_are_case_sensitive_and_take_underscores_and_dots() {
    assert_asl(E10_SRC, E10_ASL);
    assert_refused(D07_SRC, "unresolved symbol `$$X@A1`");
    assert_asl(F17_SRC, F17_ASL);
    assert_refused(F15_SRC, "unresolved symbol `$$x.y@A1`");
    assert_refused(E20_SRC, "unresolved symbol `$$z@A1`");
}

#[test]
fn temporary_symbols_work_in_every_operand_shape() {
    assert_asl(F18_SRC, F18_ASL);
    assert_asl(I31_SRC, I31_ASL);
    assert_asl(J02_SRC, J02_ASL);
    assert_asl(J03_SRC, J03_ASL);
    assert_asl(J04_SRC, J04_ASL);
    assert_asl(J05_SRC, J05_ASL);
}

#[test]
fn temporary_symbols_work_under_z80() {
    assert_asl(F05_SRC, F05_ASL);
    assert_asl(I10_SRC, I10_ASL);
}

#[test]
fn a_temporary_symbol_is_never_defined() {
    assert_asl(I07_SRC, I07_ASL);
    assert_asl(J01_SRC, J01_ASL);
}

#[test]
fn a_dollar_pair_without_a_name_is_refused() {
    assert_refused(E14_SRC, "`$` with no hex digits");
    assert_refused(E15_SRC, "`$` with no hex digits");
    assert_refused(E18_SRC, "`$` with no hex digits");
    assert_refused(K01_SRC, "`$$.` does not start a temporary symbol name");
}

/// All 93 `$$` lines of `sonic3k.asm`'s 68000 code, in ten verbatim ranges with
/// the macros they use and stand-in equates (`mk_s3kprobe.sh` in the note's
/// directory says exactly what is not verbatim). The expected image is asl's
/// own, 2,466 bytes, CRC32 `a3146e4a`.
#[test]
fn the_sonic_3_temporary_labels_match_asl() {
    let src = include_str!("vectors/s3k_dollar_labels/s3kdollar.asm");
    let want: &[u8] = include_bytes!("vectors/s3k_dollar_labels/s3kdollar.asl.bin");
    let got = assemble(src).unwrap_or_else(|d| panic!("diagnostics: {d:?}"));
    assert_eq!(got.len(), want.len(), "image length");
    let differ: Vec<usize> = (0..got.len()).filter(|&i| got[i] != want[i]).collect();
    assert!(differ.is_empty(), "{} bytes differ from asl's, first at {:#x}", differ.len(), differ[0]);
}

// ── Probe sources and asl's bytes (generated by gen_tests.py) ─────────────────

const D01_SRC: &str = r#"	cpu 68000
	org $1200
A1:	dc.w $$x
	nop
$$x:	dc.w $$x,$$y
$$y:	dc.w $$y
B1:	nop
	nop
	nop
$$x:	dc.w $$x
	dc.w $$y
$$y:	dc.w $$x,$$y
"#;
const D01_ASL: &str = "12044e711204120812084e714e714e711210121412101214";
const D02_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
.loc:	dc.w $$x
$$x2:	nop
.loc2:	dc.w $$x2
"#;
const D02_ASL: &str = "4e714e7112024e711206";
const D03_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
+	dc.w $$x
-	nop
	dc.w $$x
"#;
const D03_ASL: &str = "4e714e7112024e711202";
const D08_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
$$v	equ	$$x+$21
	dc.w $$v,$$x*2
B1:	nop
$$x:	nop
$$v	equ	$$x+$43
	dc.w $$v
"#;
const D08_ASL: &str = "4e714e71122324044e714e71124d";
const D09_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	bra.s	$$x
	beq.w	$$y
	nop
$$y:	nop
"#;
const D09_ASL: &str = "4e714e7160fc670000044e714e71";
const E02_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
$$l:	nop
	dc.w $$l
	endm
A1:	nop
$$x:	nop
	m
	nop
	m
	dc.w $$x
"#;
const E02_ASL: &str = "4e714e714e7112044e714e71120a1202";
const E05_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	phase $3400
	nop
	dc.w $$x
$$y:	dc.w $$y
	dephase
	dc.w $$x,$$y
"#;
const E05_ASL: &str = "4e714e714e711202340412023404";
const E06_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
B	equ	$$x
	dc.w B
"#;
const E06_ASL: &str = "4e714e711202";
const E07_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
.l:	nop
	dc.w A1.l,$$x
"#;
const E07_ASL: &str = "4e714e714e7112041202";
const E10_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
$$X:	nop
	dc.w $$x,$$X
"#;
const E10_ASL: &str = "4e714e714e7112021204";
const E11_SRC: &str = r#"	cpu 68000
	org $1200
m2	macro arg
	nop
	dc.w arg
	endm
A1:	nop
$$x:	nop
	m2 $$x
"#;
const E11_ASL: &str = "4e714e714e711202";
const E12_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
m3	macro
	nop
	endm
	dc.w $$x
"#;
const E12_ASL: &str = "4e714e711202";
const E13_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	rept 2
	nop
	endm
	dc.w $$x
"#;
const E13_ASL: &str = "4e714e714e714e711202";
const E19_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	charset $41,$11
	dc.w $$x
"#;
const E19_ASL: &str = "4e714e711202";
const F01_SRC: &str = r#"	cpu 68000
	org $1200
V	set	1
$$y:	nop
W	set	1
	nop
V	set	2
	dc.w $$y
"#;
const F01_ASL: &str = "4e714e711200";
const F02_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$x
	endm
A1:	nop
$$x:	nop
	m
"#;
const F02_ASL: &str = "4e714e714e711202";
const F03_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
$$x:	nop
	dc.w $$x
	endm
A1:	nop
$$x:	nop
	m
	dc.w $$x
"#;
const F03_ASL: &str = "4e714e714e7112041202";
const F05_SRC: &str = r#"	cpu z80
	org 1200h
A1:	nop
$$x:	nop
	dw $$x
B1:	nop
	nop
$$x:	nop
	dw $$x
"#;
const F05_ASL: &str = "000001120000000612";
const F11_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$q
	endm
A1:	nop
	m
$$q:	nop
"#;
const F11_ASL: &str = "4e714e7112064e71";
const F13_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	if 1
	nop
	endif
	dc.w $$x
"#;
const F13_ASL: &str = "4e714e714e711202";
const F14_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
Lb:	nop
$$q:	nop
	dc.w $$q
	endm
A1:	nop
	m
	nop
	m
"#;
const F14_ASL: &str = "4e714e714e7112044e714e714e71120c";
const F17_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$a_b:	nop
$$c.d:	nop
	dc.w $$a_b,$$c.d
"#;
const F17_ASL: &str = "4e714e714e7112021204";
const F18_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w $$x
	nop
	dc.w -$$x+$$x*2
"#;
const F18_ASL: &str = "4e714e7112024e711202";
const F19_SRC: &str = r#"	cpu 68000
	org $1200
	phase $2400
A1:	nop
$$x:	nop
	dephase
	dc.w $$x
"#;
const F19_ASL: &str = "4e714e712402";
const F20_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	org $1300
	dc.w $$x
"#;
const F20_ASL: &str = "4e714e710000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001202";
const G05_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	page 0
	dc.w $$x
"#;
const G05_ASL: &str = "4e714e711202";
const G07_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	save
	dc.w $$x
	restore
"#;
const G07_ASL: &str = "4e714e711202";
const G17_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$x
	endm
A1:	nop
	m
$$x:	nop
	m
"#;
const G17_ASL: &str = "4e714e7112064e714e711206";
const I01_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	cpu 68000
$$x:	nop
	padding off
	dc.w $$x
"#;
const I01_ASL: &str = "4e714e711202";
const I02_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	rept 2
	nop
$$r:	nop
	dc.w $$r
	endm
"#;
const I02_ASL: &str = "4e714e714e7112044e714e71120a";
const I05_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	nop
$$x	label	*
	dc.w $$x
"#;
const I05_ASL: &str = "4e714e711204";
const I07_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w defined($$x),defined($$y)
	ifdef $$x
	dc.w $1111
	endif
	ifndef $$y
	dc.w $2222
	endif
"#;
const I07_ASL: &str = "4e714e71000000002222";
const I10_SRC: &str = r#"	cpu z80
	org 1200h
A1:	nop
$$x:	nop
	jr	$$x
	ld	hl,$$x
	jr	$
	dw	$+2,$$x
	ld	a,($$x)
"#;
const I10_ASL: &str = "000018fd21011218fe0b1201123a0112";
const I25_SRC: &str = r#"	cpu 68000
	org $1200
inner	macro
	nop
	dc.w $$a
	endm
outer	macro
	nop
$$a:	nop
	inner
	endm
A1:	nop
$$a:	nop
	outer
	outer
	dc.w $$a
"#;
const I25_ASL: &str = "4e714e714e714e714e7112064e714e714e71120e1202";
const I27_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
	dc.w $$q
	endm
A1:	nop
$$q:	nop
	m
B1:	nop
	nop
$$q:	nop
	m
"#;
const I27_ASL: &str = "4e714e714e7112024e714e714e714e71120c";
const I28_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
$$x:	nop
	m
A1b	set	1
$$x:	nop
	dc.w $$x
"#;
const I28_ASL: &str = "4e714e714e714e711206";
const I30_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
$$x:	nop
	dc.w $$x
	endm
A1:	nop
	m
	m
	m
"#;
const I30_ASL: &str = "4e714e7112024e7112064e71120a";
const I31_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	nop
$$x:	nop
	cmpi.w	#$$x,d0
	dc.w $$x,$1234
"#;
const I31_ASL: &str = "4e714e714e710c40120412041234";
const J01_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	dc.w defined($$x)
$$x:	nop
$$v	set	5
	dc.w defined($$x),defined($$v),defined(A1)
	if defined($$v)
	dc.w $1111
	else
	dc.w $2222
	endif
"#;
const J01_ASL: &str = "4e7100004e710000000000012222";
const J02_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	move.w	#$$x,d0
	lea	$$x(pc),a0
	move.w	($$x).w,d2
	dc.l	$$x+$10000
"#;
const J02_ASL: &str = "4e714e71303c120241fafff83438120200011202";
const J03_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$x:	nop
	dc.w	$$x-Lab1+$10
"#;
const J03_ASL: &str = "4e714e710012";
const J04_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$x:	nop
	move.w	#$$x-Lab1,d1
	move.w	#($$x-Lab1)*3,d2
"#;
const J04_ASL: &str = "4e714e71323c0002343c0006";
const J05_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	move.w	#$$x+$10,d1
"#;
const J05_ASL: &str = "4e714e71323c1212";
const J06_SRC: &str = r#"	cpu 68000
	org $1200
V	set	$10
$$x:	nop
W	set	$20
	nop
V	set	$30
	dc.w $$x
	nop
$$y:	nop
W	set	$40
	nop
V	set	$50
	dc.w $$y
"#;
const J06_ASL: &str = "4e714e7112004e714e714e711208";
const K02_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	dc.w $$q
Lb:	nop
$$q:	nop
	endm
Lab1:	nop
$$q:	nop
	m
"#;
const K02_ASL: &str = "4e714e7112024e714e71";
const K03_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
$$v	equ	*+$20
$$w	set	*+$30
	endm
Lab1:	nop
	m
	dc.w $$v,$$w
"#;
const K03_ASL: &str = "4e714e7112241234";
const K04_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
$$x	label	*
	endm
Lab1:	nop
	m
	dc.w $$x
"#;
const K04_ASL: &str = "4e714e711204";
const K07_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	dc.w $$q
Lb:	nop
	nop
$$q:	nop
	dc.w $$q
	endm
Lab1:	nop
	nop
$$q:	nop
	m
	m
"#;
const K07_ASL: &str = "4e714e714e7112044e714e714e71120c12164e714e714e711216";
const Y01_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$v	set	$1234
	dc.w $$v
"#;
const Y01_ASL: &str = "4e711234";
const Y03_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$v	:=	$1234
	dc.w $$v
"#;
const Y03_ASL: &str = "4e711234";
const Y04_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$v	=	$1234
	dc.w $$v
"#;
const Y04_ASL: &str = "4e711234";
const D04_SRC: &str = r#"	cpu 68000
	org $1200
$$x:	nop
	dc.w $$x
A1:	nop
	dc.w $$x
"#;
const D05_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
A2	equ	$4321
	dc.w $$x
"#;
const D06_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
A2	set	$4321
	dc.w $$x
"#;
const D07_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w $$X
"#;
const D10_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
B1:	dc.w $$x
"#;
const E01_SRC: &str = r#"	cpu 68000
	org $1200
V	set	1
	nop
$$x:	nop
	dc.w $$x
V	set	2
	nop
	nop
$$x:	nop
	dc.w $$x
"#;
const E03_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
$$x:	nop
	m
	dc.w $$x
"#;
const E08_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	cpu 68000
	dc.w $$x
"#;
const E14_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$:	nop
	dc.w $$
"#;
const E15_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$1:	nop
	dc.w $$1
"#;
const E18_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	nop
	dc.w $$
"#;
const E20_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w $$x
	dc.w $$z
"#;
const F07_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	padding off
	dc.w $$x
"#;
const F08_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	supmode on
	dc.w $$x
"#;
const F09_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	listing on
	dc.w $$x
"#;
const F10_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	nop
$$q:	nop
	endm
A1:	nop
	m
	dc.w $$q
"#;
const F12_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	save
	restore
	dc.w $$x
"#;
const F15_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	dc.w $$x
	dc.w $$x.y
"#;
const F16_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
B1	label	*
	dc.w $$x
"#;
const G01_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
Lb:	nop
	endm
A1:	nop
	m
$$z:	nop
	dc.w $$z
	m
$$z:	nop
	dc.w $$z
"#;
const G02_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	cpu 68000
$$x:	nop
	cpu 68000
$$x:	nop
	dc.w $$x
"#;
const G04_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	cpu 68000
$$x:	nop
	padding off
$$x:	nop
	dc.w $$x
"#;
const G06_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	enum	E1,E2
	dc.w $$x
"#;
const G10_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
V	set	1
	dc.w A1
	dc.w $$x
"#;
const G11_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	padding on
	dc.w $$x
"#;
const G12_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	supmode off
	dc.w $$x
"#;
const G13_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	listing off
	listing on
	dc.w $$x
"#;
const G14_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
Str	struct
f1	ds.w	1
Str	endstruct
	dc.w $$x
"#;
const G16_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	nextenum	E3
	dc.w $$x
"#;
const G20_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	cpu z80
	cpu 68000
	dc.w $$x
"#;
const I16_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
	dc.w $$x
B1:	nop
$$x:	nop
"#;
const I26_SRC: &str = r#"	cpu 68000
	org $1200
A1:	nop
$$x:	nop
	cpu z80
$$x:	nop
	cpu 68000
	dc.w $$x
"#;
const K01_SRC: &str = r#"	cpu 68000
	org $1200
Lab1:	nop
$$_x:	nop
$$.y:	nop
	dc.w $$_x,$$.y
"#;
const K06_SRC: &str = r#"	cpu 68000
	org $1200
m	macro
	dc.w $$q
Lb:	nop
$$q:	nop
	endm
Lab1:	nop
	m
"#;
