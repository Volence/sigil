//! `(expr)` in a 68k EA position is the SAME width-selecting absolute-address
//! operand as a bare `expr`, and the register-indirect family it shares its
//! parentheses with must not be pulled in with it.
//!
//! ## What was refused
//!
//! ```text
//!     move.w  (v_limitright2),d0
//!     error: absolute address operand `(expr)` needs an explicit `.w`/`.l`
//!            width suffix (width-selecting bare `(expr)` is out of scope)
//! ```
//!
//! An OVER-REFUSAL on the AS-replacement path: asl assembles that line, and it
//! is the single site that put 120 of Sonic 1's 288 documented build-option
//! corners out of sigil's reach (`_incObj/DebugMode.asm:245`, reachable only
//! under `FixBugs = 1`).
//!
//! ## The reference, and where every byte below comes from
//!
//! asl, reference build md5 `61e672562465725a8c102288a7da9098`, invoked through
//! `asl_run` (`docs/superpowers/notes/asl-reference/asl_ref.sh`), exit 0 and
//! `ASL_DIAG=complete` on every probe quoted here, so no byte below is read out
//! of a failed run. The probes and their listings are reproduced in
//! `docs/superpowers/notes/2026-09-16-as-width-suffix-bare-expr.md`.
//!
//! asl gives `(expr)` and `expr` the same encoding everywhere it was asked:
//! both sides of a `move`, every width boundary, `lea`/`clr`/`tst`/`pea`/
//! `movem`/`cmpi`/`btst`/`jmp`/`jsr`, a forward reference, and a nested
//! `((Sym))`.
//!
//! ## The poisons, which are the load-bearing half
//!
//! Parentheses are shared with `(An)`, `(An)+`, `-(An)`, `(d16,An)`,
//! `(d8,An,Xn)` and `(d16,PC)`. A change that swallowed any of those into
//! absolute addressing would be byte-silent on programs that build today, so
//! each is pinned here against asl's encoding.
//!
//! And one spelling is refused rather than accepted, deliberately. asl's
//! register names are case-insensitive even under `-U`: with `A0: equ $1234`
//! in scope it still assembles `move.w (A0),d0` as `3010`, a0 indirect. sigil's
//! operand classifier claims `(a0)`..`(a7)`/`(sp)` in LOWER CASE only, so
//! `(A0)` arrives at the absolute-address arm. Reading it as an address would
//! turn a loud refusal into a silently different encoding, which is worse than
//! the refusal this parcel removed, so it stays a refusal until uppercase
//! register indirect is implemented (`AS-UPPERCASE-REGISTER-INDIRECT` in the
//! campaign gap ledger).

use sigil_frontend_as::{assemble_root_relocating_warned, Options};
use sigil_ir::{Module, SymbolTable};

fn assemble(body: &str) -> Result<Module, Vec<(u32, String)>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_relocating_warned(&path, &Options::default()) {
        Ok(a) => Ok(a.module),
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

fn bytes(body: &str) -> Vec<u8> {
    let m = assemble(body).expect("front end accepts");
    let stubs = SymbolTable::new();
    let resolved =
        sigil_link::resolve_layout(&m.sections, &stubs, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &stubs).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

fn refused(body: &str, needles: &[&str]) {
    match assemble(body) {
        Ok(_) => panic!("assembled instead of refusing:\n{body}"),
        Err(diags) => {
            let hit = diags
                .iter()
                .any(|(_, m)| needles.iter().all(|n| m.contains(n)));
            assert!(hit, "no diagnostic naming {needles:?}; got {diags:?}");
        }
    }
}

const CPU: &str = "\tcpu 68000\n";

/// `b` is asl's byte column, written as it appears in the listing.
fn hex(b: &str) -> Vec<u8> {
    let s: String = b.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex pair"))
        .collect()
}

/// The defect's own line, in the shape the corpus writes it.
#[test]
fn the_corpus_line_assembles_and_selects_abs_w() {
    // `v_limitright2` is a `ds.w` in s1disasm's `_Variables.asm` RAM block,
    // which lives in the `$FFFF8000..$FFFFFF` window where asl's
    // sign-extension rule selects abs.w. The address here is a representative
    // one from that window rather than the corpus's own; the corpus's own
    // address is proved by the whole-ROM byte identity in the note.
    // asl p7.lst 8: `3038 F728`.
    assert_eq!(
        bytes(&format!(
            "{CPU}v_limitright2:\tequ $FFFFF728\n\tmove.w (v_limitright2),d0\n"
        )),
        hex("3038 F728")
    );
}

/// `(expr)` and `expr` encode identically, at every width boundary asl's
/// sign-extension rule has.
#[test]
fn paren_and_bare_absolute_select_the_same_width() {
    // asl p1.lst lines 15/16, 19/20, 22..29.
    for (equ, asl) in [
        ("$1000", "3038 1000"),
        ("$00A00000", "3039 00A0 0000"),
        ("$7FFE", "3038 7FFE"),
        ("$8000", "3039 0000 8000"),
        ("$FF8000", "3038 8000"),
        ("$FFFFFE", "3038 FFFE"),
    ] {
        let paren = bytes(&format!("{CPU}S:\tequ {equ}\n\tmove.w (S),d0\n"));
        let bare = bytes(&format!("{CPU}S:\tequ {equ}\n\tmove.w S,d0\n"));
        assert_eq!(paren, hex(asl), "paren form of {equ}");
        assert_eq!(bare, hex(asl), "bare form of {equ}");
    }
}

/// Every EA position and mnemonic the probe asked asl about.
#[test]
fn paren_absolute_in_every_position_asl_was_asked_about() {
    const EQUS: &str = "S:\tequ $1000\nB:\tequ $00A00000\n";
    for (src, asl) in [
        // p1.lst 38, 40: an expression and a nested paren inside the parens.
        ("\tmove.w (S+2),d0\n", "3038 1002"),
        ("\tmove.w ((S)),d0\n", "3038 1000"),
        // p1.lst 42, 44: the DESTINATION side.
        ("\tmove.w d0,(S)\n", "31C0 1000"),
        ("\tmove.w d0,(B)\n", "33C0 00A0 0000"),
        // p1.lst 46, 48, 49, 50, 51.
        ("\tlea (S),a1\n", "43F8 1000"),
        ("\tlea (B),a1\n", "43F9 00A0 0000"),
        ("\tclr.w (S)\n", "4278 1000"),
        ("\ttst.b (B)\n", "4A39 00A0 0000"),
        ("\tmove.b (S),(B)\n", "13F8 1000 00A0 0000"),
    ] {
        assert_eq!(bytes(&format!("{CPU}{EQUS}{src}")), hex(asl), "{src}");
    }
}

/// `jmp`/`jsr`/`pea`/`movem`/`cmpi`/`btst`, which reach their EA by paths of
/// their own, and `jmp (a0)`, which must stay register indirect right beside
/// them. asl p4.lst, `Tbl` at $0.
#[test]
fn control_flow_and_the_special_ea_paths() {
    const HEAD: &str = "\torg 0\nTbl:\tdc.w 1,2\n";
    for (src, asl) in [
        ("\tjmp (Tbl)\n", "0001 0002 4EF8 0000"),
        ("\tjsr (Tbl)\n", "0001 0002 4EB8 0000"),
        ("\tjmp (a0)\n", "0001 0002 4ED0"),
        ("\tjmp (Tbl).l\n", "0001 0002 4EF9 0000 0000"),
        ("\tpea (Tbl)\n", "0001 0002 4878 0000"),
        ("\tmovem.w d0-d1,(Tbl)\n", "0001 0002 48B8 0003 0000"),
        ("\tmovem.w (Tbl),d0-d1\n", "0001 0002 4CB8 0003 0000"),
        ("\tcmpi.w #5,(Tbl)\n", "0001 0002 0C78 0005 0000"),
        ("\tbtst #3,(Tbl)\n", "0001 0002 0838 0003 0000"),
        ("\tmove.w (Tbl),(Tbl)\n", "0001 0002 31F8 0000 0000"),
    ] {
        assert_eq!(bytes(&format!("{CPU}{HEAD}{src}")), hex(asl), "{src}");
    }
}

/// A FORWARD reference inside the parens takes the same optimistic-abs.w pass
/// loop the bare form takes, and converges on asl's width. asl p8.lst 7-10:
/// `FwdLab` lands at $14 and the operand is `3038 0014`, abs.w, in both
/// spellings.
#[test]
fn a_forward_reference_inside_the_parens_converges_to_asls_width() {
    // p8.asm verbatim, so the label's address is asl's and not this test's
    // arithmetic: the `(Tbl,pc)` line is part of the probe and carries the
    // forward label four bytes further along.
    let out = bytes(&format!(
        "{CPU}\torg 0\nTbl:\tdc.w 1,2\n\tmove.w (Tbl,pc),d1\n\tmove.w (FwdLab),d0\n\tmove.w FwdLab,d0\n\tmove.l (FwdLab),d1\nFwdLab:\tdc.w 0\n"
    ));
    assert_eq!(
        out,
        hex("0001 0002 323A FFFA 3038 0014 3038 0014 2238 0014 0000")
    );
    // The two spellings agree operand for operand, which is the property the
    // parcel claims and is not visible from either encoding alone.
    assert_eq!(out[8..12], out[12..16]);
}

// ---------------------------------------------------------------------------
// POISONS: what must NOT have moved.
// ---------------------------------------------------------------------------

/// The register-indirect family wears the same parentheses. If any of these
/// starts encoding as an absolute address the change is byte-silent on every
/// program that builds today, so each is pinned against asl. p1.lst 31-36,
/// p6.lst, p4.lst 10.
#[test]
fn the_register_indirect_family_is_untouched() {
    for (src, asl) in [
        ("\tmove.w (a0),d0\n", "3010"),
        ("\tmove.w (a7),d0\n", "3017"),
        ("\tmove.w (sp),d0\n", "3017"),
        ("\tmove.w (a0)+,d0\n", "3018"),
        ("\tmove.w -(a0),d0\n", "3020"),
        ("\tmove.w (4,a0),d0\n", "3028 0004"),
        ("\tmove.w 4(a0),d0\n", "3028 0004"),
        ("\tmove.w (4,a0,d1.w),d0\n", "3030 1004"),
    ] {
        assert_eq!(bytes(&format!("{CPU}{src}")), hex(asl), "{src}");
    }
}

/// And it stays register indirect even when a SYMBOL of that name is in scope,
/// which is the case where a regression would be silent rather than loud. asl
/// p2.lst: with `A0: equ $1234` defined, `move.w (a0),d0` is still `3010`.
#[test]
fn a_symbol_named_like_a_register_does_not_capture_the_indirect_form() {
    assert_eq!(
        bytes(&format!("{CPU}a0:\tequ $1234\n\tmove.w (a0),d0\n")),
        hex("3010")
    );
    assert_eq!(
        bytes(&format!("{CPU}sp:\tequ $1234\n\tmove.w (sp),d0\n")),
        hex("3017")
    );
}

/// PC-relative is a different row and this parcel does not touch it: both
/// `(d16,PC)` and `(d8,PC,Xn)` must still encode PC-relative rather than
/// absolute. asl p4.lst 6-7, with `Tbl` at $0 and the instruction at $4 / $8.
#[test]
fn pc_relative_is_still_pc_relative() {
    assert_eq!(
        bytes(&format!("{CPU}\torg 0\nTbl:\tdc.w 1,2\n\tmove.w (Tbl,pc,d0.w),d1\n")),
        hex("0001 0002 323B 00FA")
    );
    assert_eq!(
        bytes(&format!("{CPU}\torg 0\nTbl:\tdc.w 1,2\n\tmove.w (Tbl,pc),d1\n")),
        hex("0001 0002 323A FFFA")
    );
}

/// An UPPERCASE register indirect is REFUSED, not read as an absolute address.
/// asl reads `(A0)` as a0 indirect (`3010`, p2.lst 6) even with `A0: equ $1234`
/// in scope, so accepting it here would emit a different instruction silently.
/// The refusal names the register, so the reader is told which of the two
/// readings sigil declined to guess between.
#[test]
fn an_uppercase_register_indirect_is_refused_rather_than_read_as_an_address() {
    for spelling in ["A0", "A7", "SP", "Sp"] {
        refused(
            &format!("{CPU}{spelling}:\tequ $1234\n\tmove.w ({spelling}),d0\n"),
            &["names a 68k register", spelling],
        );
        // ... and with no such symbol in scope either, so the refusal is about
        // the register name and not about symbol resolution.
        refused(
            &format!("{CPU}\tmove.w ({spelling}),d0\n"),
            &["names a 68k register", spelling],
        );
    }
}

/// `(dN)` is not a 68k addressing mode at all (asl p5: `error #1505: addressing
/// mode not supported on 68000`), so reading it as an absolute address would be
/// an OVER-acceptance in the other direction. Refused in both cases.
#[test]
fn a_data_register_in_parens_is_refused_in_either_case() {
    for spelling in ["d0", "D0", "d7"] {
        refused(
            &format!("{CPU}{spelling}:\tequ $1234\n\tmove.w ({spelling}),d0\n"),
            &["names a 68k register", spelling],
        );
    }
}

/// `(pc)` alone is PC-relative at zero displacement in asl (`303A FFFE`,
/// p6.lst), an operand sigil has no spelling for. It must not become an
/// absolute address through a symbol named `pc`.
#[test]
fn a_bare_pc_in_parens_is_refused_rather_than_read_as_an_address() {
    for spelling in ["pc", "PC"] {
        refused(
            &format!("{CPU}{spelling}:\tequ $1234\n\tmove.w ({spelling}),d0\n"),
            &["names a 68k register", spelling],
        );
    }
}

/// `sr`/`ccr`/`usp` are deliberately NOT guarded: asl reads them as ordinary
/// symbols inside parens, so absolute addressing is the faithful answer and a
/// guard there would be a fresh over-refusal. asl p6.lst 9-11.
#[test]
fn sr_ccr_and_usp_in_parens_are_ordinary_symbols() {
    for (name, equ, asl) in [
        ("sr", "$2000", "3038 2000"),
        ("ccr", "$2002", "3038 2002"),
        ("usp", "$2004", "3038 2004"),
    ] {
        assert_eq!(
            bytes(&format!("{CPU}{name}:\tequ {equ}\n\tmove.w ({name}),d0\n")),
            hex(asl),
            "({name})"
        );
    }
}

/// The EXPLICIT-width absolute arm is a different arm and keeps its own window
/// refusal: `(addr).w` outside `[0,$7FFF] u [$FF8000,$FFFFFF]` is still an
/// error rather than falling back to the newly-available width selection. asl:
/// `error #1340: short addressing not allowed`.
#[test]
fn an_explicit_width_suffix_still_pins_the_width() {
    refused(
        &format!("{CPU}\tmove.w d0,($C00004).w\n"),
        &["$C00004", "abs.w"],
    );
    // And `.l` still forces the long form where the width rule would have
    // picked short. asl: `move.w ($1000).l,d0` is `3039 0000 1000`.
    assert_eq!(
        bytes(&format!("{CPU}\tmove.w ($1000).l,d0\n")),
        hex("3039 0000 1000")
    );
}
