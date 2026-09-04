//! Macro parameters declared with a default value: `macro loc,port=(4).l`.
//!
//! AS lets a parameter carry `NAME=text`, and substitutes that text wherever the
//! call supplies nothing for the slot. `s1disasm/Macros.asm(11)` is the corpus's
//! only declaration of one —
//!
//! ```text
//! locVRAM: macro loc,controlport=(vdp_control_port).l
//!          move.l  #($40000000+…),controlport
//! ```
//!
//! — and forty-eight call sites omit the second argument. Substituting nothing
//! there left `move.l #…,` with an empty destination, which is where fifty-four
//! of Sonic 1's diagnostics came from.
//!
//! Every expected value below is read off `asl` 1.42 Beta Bld 212's own listing
//! for the identical source, run with Sonic 1's own flags
//! (`asl -xx -n -q -A -L -U -i . probe.asm`, then `p2bin -p=0`). The listing's
//! second column is the PC and the third the emitted bytes.

use sigil_frontend_as::{assemble, Options};

/// `padding off` keeps a `dc.b` image packed; `phase 0` puts the section at
/// address 0 so an image offset is the PC.
const HEAD: &str = "\tcpu 68000\n\tpadding off\n\tphase 0\n";

/// Assemble, link and flatten, or panic naming the source. Linking is what makes
/// a byte assertion mean anything: the front end alone leaves references as
/// deferred fixups, so an unbound name would survive an `assemble`-only check.
fn image(body: &str) -> Vec<u8> {
    let src = format!("{HEAD}{body}");
    let m = assemble(&src, &Options::default()).unwrap_or_else(|d| {
        panic!(
            "did not assemble: {:?}\n{src}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .unwrap_or_else(|e| panic!("did not lay out: {e:?}\n{src}"));
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .unwrap_or_else(|e| panic!("did not link: {e:?}\n{src}"));
    sigil_link::flatten(&linked, 0x00)
}

/// The corpus macro, reduced to the shape under test. `vdp_control_port` is 4
/// so the default's rendered text is short in a listing.
const LOCVRAM: &str = concat!(
    "vdp_control_port equ 4\n",
    "zqm:\tmacro loc,port=(vdp_control_port).l\n",
    "\tmove.l\t#loc,port\n",
    "\tendm\n",
);

/// A call that omits the defaulted parameter takes the default text.
///
/// ```text
///    8/       0 : (MACRO)              	zqm	$1234
///    8/       0 : 23FC 0000 1234              move.l  #$1234,(vdp_control_port).l
///             6 : 0000 0004
/// ```
#[test]
fn an_omitted_argument_takes_the_default() {
    assert_eq!(
        image(&format!("{LOCVRAM}\tzqm\t$1234\n")),
        vec![0x23, 0xFC, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x04]
    );
}

/// A supplied argument wins over the default. Same call, `d0` written for the
/// second slot, and the instruction becomes a register move.
///
/// ```text
///   10/       A : (MACRO)              	zqm	$1234,d0
///   10/       A : 203C 0000 1234              move.l  #$1234,d0
/// ```
#[test]
fn a_supplied_argument_overrides_the_default() {
    assert_eq!(
        image(&format!("{LOCVRAM}\tzqm\t$1234,d0\n")),
        vec![0x20, 0x3C, 0x00, 0x00, 0x12, 0x34]
    );
}

/// Binding the FIRST parameter by keyword leaves the second unsupplied, and it
/// still takes its default.
///
/// ```text
///   12/      10 : (MACRO)              	zqm	loc=$5678
///   12/      10 : 23FC 0000 5678              move.l  #$5678,(vdp_control_port).l
///            16 : 0000 0004
/// ```
#[test]
fn a_keyword_call_still_defaults_the_slot_it_did_not_name() {
    assert_eq!(
        image(&format!("{LOCVRAM}\tzqm\tloc=$5678\n")),
        vec![0x23, 0xFC, 0x00, 0x00, 0x56, 0x78, 0x00, 0x00, 0x00, 0x04]
    );
}

/// A slot WRITTEN and left empty takes the default too — the rule is "no text
/// supplied", not "no slot written".
///
/// ```text
///   14/      1A : (MACRO)              	zqm	$1234,
///   14/      1A : 23FC 0000 1234              move.l  #$1234,(vdp_control_port).l
///            20 : 0000 0004
/// ```
#[test]
fn an_explicitly_empty_argument_takes_the_default() {
    assert_eq!(
        image(&format!("{LOCVRAM}\tzqm\t$1234,\n")),
        vec![0x23, 0xFC, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x04]
    );
}

/// Three parameters, a default on the middle one, and the call writes an empty
/// slot between two commas. Text form, so the binding itself is visible.
///
/// ```text
///   13/      3F : (MACRO)              	ac 11,,33
///   13/      40 : 3C31 317C 4445              dc.b "<11|DEF2|33>"
///            46 : 4632 7C33 333E
/// ```
#[test]
fn an_empty_middle_slot_takes_the_default_and_the_later_slots_still_bind() {
    let src = concat!(
        "ac:\tmacro p1,p2=DEF2,p3\n",
        "\tdc.b \"<p1|p2|p3>\"\n",
        "\tendm\n",
        "\tac 11,,33\n",
    );
    assert_eq!(image(src), b"<11|DEF2|33>".to_vec());
}

/// A keyword naming a LATER parameter: the skipped defaulted one takes its
/// default, and the skipped defaultless one stays empty.
///
/// ```text
///   14/      54 : (MACRO)              	ac p3=99
///   14/      55 : 3C7C 4445 4632              dc.b "<|DEF2|99>"
///            5B : 7C39 393E
/// ```
#[test]
fn a_later_keyword_leaves_the_defaultless_slot_empty() {
    let src = concat!(
        "ac:\tmacro p1,p2=DEF2,p3\n",
        "\tdc.b \"<p1|p2|p3>\"\n",
        "\tendm\n",
        "\tac p3=99\n",
    );
    assert_eq!(image(src), b"<|DEF2|99>".to_vec());
}

/// `ARGCOUNT` and an unshifted `ALLARGS` are read off the CALL, not off the
/// bindings: a defaulted slot contributes to neither.
///
/// ```text
///   10/       B : (MACRO)              	ac 11
///   10/       B : 01                          dc.b 1
///   10/       C : 3C31 317C 4445              dc.b "<11|DEF2|>"
///            12 : 4632 7C3E
///   10/      16 : 5B31 315D                   dc.b "[11]"
/// ```
#[test]
fn argcount_and_allargs_do_not_see_a_default() {
    let src = concat!(
        "ac:\tmacro p1,p2=DEF2,p3\n",
        "\tdc.b ARGCOUNT\n",
        "\tdc.b \"<p1|p2|p3>\"\n",
        "\tdc.b \"[ALLARGS]\"\n",
        "\tendm\n",
        "\tac 11\n",
    );
    let mut want = vec![1u8];
    want.extend_from_slice(b"<11|DEF2|>");
    want.extend_from_slice(b"[11]");
    assert_eq!(image(src), want);
}

/// The identifiers written INSIDE a default are not parameters.
///
/// This is the half a byte test of the corpus macro cannot see. Reading the
/// parameter list by harvesting every identifier in it made
/// `macro loc,slot=(vdp_ctrl).l` declare four parameters — `loc`, `slot`,
/// `vdp_ctrl` and `.l` — so a body that mentioned `vdp_ctrl` had it substituted
/// away to nothing, and `dc.b vdp_ctrl` became `dc.b` with no operand.
///
/// ```text
///   11/       0 : (MACRO)              	ph $77
///   11/       0 : 77                          dc.b $77
///   11/       1 : 12                          dc.b vdp_ctrl
///   11/       2 : 34                          dc.b widthsel
///   11/       3 : 3C28 7664 705F              dc.b "<(vdp_ctrl).l>"
///             9 : 6374 726C 292E
///             F : 6C3E
/// ```
#[test]
fn identifiers_inside_a_default_are_not_parameters() {
    let src = concat!(
        "vdp_ctrl equ $12\n",
        "widthsel equ $34\n",
        "ph:\tmacro loc,slot=(vdp_ctrl).l\n",
        "\tdc.b loc\n",
        "\tdc.b vdp_ctrl\n",
        "\tdc.b widthsel\n",
        "\tdc.b \"<slot>\"\n",
        "\tendm\n",
        "\tph $77\n",
    );
    let mut want = vec![0x77, 0x12, 0x34];
    want.extend_from_slice(b"<(vdp_ctrl).l>");
    assert_eq!(image(src), want);
}

/// A default may contain a comma inside parentheses; the parameter list splits
/// on TOP-LEVEL commas only, so `(1,2)` is one default and `r` is a second
/// parameter rather than the list running to five names.
///
/// ```text
///   17/      4A : (MACRO)              	b1
///   17/      4A : 3C28 312C 3229              dc.b "<(1,2)|ZZ>"
///            50 : 7C5A 5A3E
/// ```
#[test]
fn a_default_may_contain_a_parenthesised_comma() {
    let src = concat!(
        "b1:\tmacro q=(1,2),r=ZZ\n",
        "\tdc.b \"<q|r>\"\n",
        "\tendm\n",
        "\tb1\n",
    );
    assert_eq!(image(src), b"<(1,2)|ZZ>".to_vec());
}

/// A `{INTLABEL}` group still consumes no parameter position when the list also
/// carries a default. The lexer swallows the group without emitting a token, so
/// the comma split sees an EMPTY group there — which must contribute no
/// parameter rather than an unnamed one that would shift every later slot.
///
/// ```text
///   12/       0 : (MACRO)              Lb: im 11
///   12/       0 : 3C4C 627C 3131              dc.b "<Lb|11|QQ>"
///             6 : 7C51 513E
/// ```
#[test]
fn an_intlabel_group_consumes_no_slot_beside_a_default() {
    let src = concat!(
        "im:\tmacro {INTLABEL},pp,qq=QQ\n",
        "\tdc.b \"<__LABEL__|pp|qq>\"\n",
        "\tendm\n",
        "Lb:\tim 11\n",
    );
    assert_eq!(image(src), b"<Lb|11|QQ>".to_vec());
}
