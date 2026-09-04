//! A label written on the same line as a block directive.
//!
//! AS binds the label field of a line before the line's directive runs, and it
//! does so for a block head (`Lab: if …`, `Lab: rept …`) exactly as for an
//! instruction. sigil's `exec` used to route such a line straight to its block
//! handler, which never looks at the label column, so the name was neither
//! bound nor diagnosed — it simply did not exist, and every later reference to
//! it failed at LINK with no location. `s1disasm/sonic.asm(4121)` writes
//! `Map_Ring:   if Revision=0`, the whole Sonic 1 corpus's only instance.
//!
//! Every expectation below is read off `asl` 1.42 Beta Bld 212's own listing
//! and `p2bin` output for the identical source text
//! (`asl -cpu 68000 -q -U -L -o p.p probe.asm && p2bin p.p p.bin`). The
//! listing's second column is the PC, the third the emitted bytes; a symbol
//! table row `L : 2 C` is a relocatable code label at PC 2.

use sigil_frontend_as::{assemble, Options};

/// The common head. `padding off` keeps a `dc.b`-then-`dc.l` image packed, and
/// `phase 0` puts the section at address 0 so an image offset IS the PC.
const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";

/// Assemble AND LINK. An unbound name survives the front end as a deferred
/// fixup, so `assemble` alone returns `Ok` and a byte assertion on it would be
/// vacuous — the link is what refuses it.
fn linked(src: &str) -> Option<Vec<u8>> {
    let m = assemble(src, &Options::default()).ok()?;
    let resolved =
        sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true).ok()?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).ok()?;
    Some(sigil_link::flatten(&linked, 0x00))
}

/// The bytes, or a panic naming the source. Used where AS assembles clean.
fn image(body: &str) -> Vec<u8> {
    let src = format!("{HEAD}{body}");
    linked(&src).unwrap_or_else(|| panic!("did not assemble+link:\n{src}"))
}

/// Whether the source links at all. Used where AS leaves the name UNDEFINED and
/// reports `#1: symbol undefined` at the reference.
fn links(body: &str) -> bool {
    linked(&format!("{HEAD}{body}")).is_some()
}

/// The offset of a named label in the assembled module, or `None` where no
/// section carries it. A byte assertion cannot tell a placed label from a
/// folded constant of the same value; this can.
fn label_offset(body: &str, name: &str) -> Option<u32> {
    let m = assemble(&format!("{HEAD}{body}"), &Options::default()).ok()?;
    m.sections
        .iter()
        .flat_map(|s| s.labels.iter())
        .find(|l| l.name == name)
        .map(|l| l.offset)
}

/// The corpus shape, both ways round. The label binds to the PC of its own
/// line, and the condition does not enter into it.
///
/// ```text
///        4/       0 : AA                      dc.b $AA
///        5/       1 : =>TRUE               L:    if 1=1
///        6/       1 : BB                      dc.b $BB
///        7/       2 : =>FALSE                  else
///        9/       2 : [5]                      endif
///       10/       2 : 0000 0001               dc.l L
///   asl bytes:  aa bb 00 00 00 01
///
///        5/       1 : =>FALSE              L:    if 1=0
///        7/       1 : =>TRUE                   else
///        8/       1 : CC                      dc.b $CC
///       10/       2 : 0000 0001               dc.l L
///   asl bytes:  aa cc 00 00 00 01
/// ```
#[test]
fn if_line_label_binds_whichever_arm_runs() {
    let taken = "\tdc.b $AA\nL:\tif 1=1\n\tdc.b $BB\n\telse\n\tdc.b $CC\n\tendif\n\tdc.l L\n";
    let untaken = "\tdc.b $AA\nL:\tif 1=0\n\tdc.b $BB\n\telse\n\tdc.b $CC\n\tendif\n\tdc.l L\n";
    assert_eq!(image(taken), vec![0xAA, 0xBB, 0x00, 0x00, 0x00, 0x01]);
    assert_eq!(image(untaken), vec![0xAA, 0xCC, 0x00, 0x00, 0x00, 0x01]);
    // A relocatable label at PC 1, not a constant that folds to 1.
    assert_eq!(label_offset(taken, "L"), Some(1));
    assert_eq!(label_offset(untaken, "L"), Some(1));
}

/// `ifdef` and the loop heads bind the same way — the rule is the label field's,
/// not `if`'s.
///
/// ```text
///        6/       1 : =>DEFINED            L:    ifdef Z          asl:  aa bb 00 00 00 01
///        5/       1 :                     L:    rept 2           asl:  aa bb bb 00 00 00 01
///        5/       1 :                     L:    irp X,1,2        asl:  aa 01 02 00 00 00 01
///        6/       1 :                     L:    while W<2        asl:  aa bb bb 00 00 00 01
/// ```
#[test]
fn other_block_head_lines_bind_their_label_too() {
    assert_eq!(
        image("\tdc.b $AA\nZ:\tequ 1\nL:\tifdef Z\n\tdc.b $BB\n\tendif\n\tdc.l L\n"),
        vec![0xAA, 0xBB, 0x00, 0x00, 0x00, 0x01]
    );
    assert_eq!(
        image("\tdc.b $AA\nL:\trept 2\n\tdc.b $BB\n\tendm\n\tdc.l L\n"),
        vec![0xAA, 0xBB, 0xBB, 0x00, 0x00, 0x00, 0x01]
    );
    assert_eq!(
        image("\tdc.b $AA\nL:\tirp X,1,2\n\tdc.b X\n\tendm\n\tdc.l L\n"),
        vec![0xAA, 0x01, 0x02, 0x00, 0x00, 0x00, 0x01]
    );
    assert_eq!(
        image("\tdc.b $AA\nW:\tset 0\nL:\twhile W<2\nW:\tset W+1\n\tdc.b $BB\n\tendm\n\tdc.l L\n"),
        vec![0xAA, 0xBB, 0xBB, 0x00, 0x00, 0x00, 0x01]
    );
}

/// AS's column rule reaches the directive line as well: a colon-less name is a
/// label at column 0 and an unknown instruction anywhere else.
///
/// ```text
///        5/       1 : =>TRUE               L    if 1=1           asl:  aa bb 00 00 00 01
///
///        5/       1 :                       L    if 1=1
///   > > > t_indented.asm(5):3: error: unknown instruction
///   > > > t_indented.asm(7): error: ELSEIF/ENDIF without IF
/// ```
#[test]
fn colonless_if_line_label_obeys_the_column_rule() {
    assert_eq!(
        image("\tdc.b $AA\nL\tif 1=1\n\tdc.b $BB\n\tendif\n\tdc.l L\n"),
        vec![0xAA, 0xBB, 0x00, 0x00, 0x00, 0x01]
    );
    // Indented, asl does not even process the `if` — the name is certainly not
    // a label, so nothing may resolve `L`.
    assert!(!links("\tdc.b $AA\n  L\tif 1=1\n\tdc.b $BB\n\tendif\n\tdc.l L\n"));
    assert_eq!(
        label_offset("\tdc.b $AA\n  L\tif 1=1\n\tdc.b $BB\n\tendif\n", "L"),
        None
    );
}

/// The line that closes the TAKEN arm is read while the assembler is still
/// emitting, so its label binds — at the PC the arm ended on, not the PC the
/// `if` started at.
///
/// ```text
///        6/       1 : BB                      dc.b $BB
///        7/       2 : =>FALSE              L:    else             asl:  aa bb 00 00 00 02
///        7/       2 : =>FALSE              L:    elseif 1=1       asl:  aa bb 00 00 00 02
///        7/       2 : [5]                  L:    endif            asl:  aa bb 00 00 00 02
/// ```
#[test]
fn the_line_closing_a_taken_arm_binds_its_label() {
    for closer in ["\telse\n\tdc.b $CC", "\telseif 1=1\n\tdc.b $CC", "\tendif"] {
        let (kw, rest) = closer.split_once('\n').unwrap_or((closer, ""));
        let tail = if rest.is_empty() {
            String::new()
        } else {
            format!("{rest}\n\tendif\n")
        };
        let body = format!(
            "\tdc.b $AA\n\tif 1=1\n\tdc.b $BB\nL:{}\n{}\tdc.l L\n",
            kw.trim_start_matches('\t'),
            tail
        );
        assert_eq!(
            image(&body),
            vec![0xAA, 0xBB, 0x00, 0x00, 0x00, 0x02],
            "closer {kw:?}"
        );
        assert_eq!(label_offset(&body, "L"), Some(2), "closer {kw:?}");
    }
}

/// …and the line that closes an arm that was SKIPPED binds nothing: it was read
/// inside a region the assembler was not emitting from. asl leaves the name out
/// of the symbol table entirely and reports `#1: symbol undefined` at the
/// reference.
///
/// ```text
///        5/       1 : =>FALSE                  if 1=0
///        7/       1 : =>TRUE               L:    else       ⇒ (absent)
///   > > > t_else_inact.asm(10):7: error: symbol undefined
///
///        5/       1 : =>FALSE                  if 1=0
///        7/       1 : [5]                  L:    endif      ⇒ (absent)
///   > > > t_endif_in.asm(8):7: error: symbol undefined
/// ```
#[test]
fn a_line_closing_a_skipped_arm_binds_nothing() {
    let else_skipped = "\tdc.b $AA\n\tif 1=0\n\tdc.b $BB\nL:\telse\n\tdc.b $CC\n\tendif\n";
    let endif_skipped = "\tdc.b $AA\n\tif 1=0\n\tdc.b $BB\nL:\tendif\n";
    assert!(!links(&format!("{else_skipped}\tdc.l L\n")));
    assert!(!links(&format!("{endif_skipped}\tdc.l L\n")));
    assert_eq!(label_offset(else_skipped, "L"), None);
    assert_eq!(label_offset(endif_skipped, "L"), None);
}

/// `endm` is not one of them. It terminates a CAPTURED body — the collector
/// consumes the line and the expansion never replays it — so asl binds no label
/// there, and neither does sigil.
///
/// ```text
///        5/       1 :                         rept 2
///        7/       1 :                     L:    endm       ⇒ (absent)
///   > > > t_endm_rept.asm(8):7: error: symbol undefined
/// ```
#[test]
fn a_label_on_endm_is_not_bound() {
    let body = "\tdc.b $AA\n\trept 2\n\tdc.b $BB\nL:\tendm\n";
    assert!(!links(&format!("{body}\tdc.l L\n")));
    assert_eq!(label_offset(body, "L"), None);
}

/// A block head inside a region that is not being assembled is not executed at
/// all, so its label is not bound either.
///
/// ```text
///        5/       1 : =>FALSE                  if 1=0
///        6/       1 :                     N:    if 1=1     ⇒ (absent)
///       10/       1 : DD                      dc.b $DD
///   asl bytes:  aa dd
/// ```
#[test]
fn a_block_head_in_a_skipped_region_binds_nothing() {
    let body = "\tdc.b $AA\n\tif 1=0\nN:\tif 1=1\n\tdc.b $BB\n\tendif\n\tendif\n\tdc.b $DD\n";
    assert_eq!(image(body), vec![0xAA, 0xDD]);
    assert_eq!(label_offset(body, "N"), None);
    assert!(!links(&format!("{body}\tdc.l N\n")));
}

/// The label an `if` line places is an ordinary PC label in every respect,
/// including that it opens the scope the `.local` names after it hang off.
///
/// ```text
///        5/       1 : =>TRUE               L:    if 1=1       ⇒  L : 1 C
///        7/       2 : CC                  .loc:    dc.b $CC     ⇒  L.loc : 2 C
///        9/       3 : 0000 0002               dc.l L.loc
///   asl bytes:  aa bb cc 00 00 00 02
/// ```
#[test]
fn an_if_line_label_opens_the_local_scope() {
    let body = "\tdc.b $AA\nL:\tif 1=1\n\tdc.b $BB\n.loc:\tdc.b $CC\n\tendif\n\tdc.l L.loc\n";
    assert_eq!(image(body), vec![0xAA, 0xBB, 0xCC, 0x00, 0x00, 0x00, 0x02]);
    assert_eq!(label_offset(body, "L.loc"), Some(2));
}

/// The directives that CONSUME the label field keep consuming it. `M: macro`
/// names the macro; it does not place `M` at the PC, and asl's symbol table
/// carries no location for it.
///
/// ```text
///        5/       1 :                     M:    macro
///        8/       1 : (MACRO)                  M
///        8/       1 : BB                          dc.b $BB
///        9/       2 : DD                      dc.b $DD
///   asl bytes:  aa bb dd
/// ```
#[test]
fn macro_still_consumes_the_name_in_its_label_field() {
    let body = "\tdc.b $AA\nM:\tmacro\n\tdc.b $BB\n\tendm\n\tM\n\tdc.b $DD\n";
    assert_eq!(image(body), vec![0xAA, 0xBB, 0xDD]);
    assert_eq!(label_offset(body, "M"), None);
}
