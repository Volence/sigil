//! A PC label written in a macro / `rept` / `irp` / `while` BODY belongs to the
//! expansion INSTANCE it is written in, and to nothing else.
//!
//! asl gives each instance its own namespace for these names, chained inward to
//! outward: the body that writes the label reads it back, a nested expansion
//! reads its enclosing one, and everybody else — the outer body after the inner
//! expansion returned, a sibling expansion, file level — gets
//! `#1010 symbol undefined`. Sigil bound every one of them globally, which had
//! two visible consequences: a name asl says does not exist resolved, and a
//! macro invoked twice produced `symbol redefined by section` from the linker on
//! programs asl assembles without complaint.
//!
//! Expectations derived from asl 1.42 Beta [Bld 212]
//! (`s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`) with Sonic 1's own flags minus `-E`/`-c`.
//! Probes `p1`–`p9` are committed under
//! `docs/superpowers/notes/2026-09-05-as-macro-body-label-probes/` with their
//! verbatim listings and a three-run stability hash for every one of them.
//!
//! EVERY FIXTURE HERE INVOKES ITS MACRO MORE THAN ONCE, AT ADDRESSES THAT
//! DIFFER. A macro invoked once cannot distinguish "each instance owns the name"
//! from "the last definition wins", and a label whose address is the same under
//! both readings proves nothing — the byte is identical either way. A
//! single-invocation fixture in this file, or one whose two expansions sit at the
//! same address, is a bug in the file.

use sigil_frontend_as::{assemble, Options};

/// Assemble AND LINK, for the cells whose evidence is BYTES.
fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default()).expect("assemble");
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble and link, expecting a REFUSAL from either stage, and hand back the
/// messages. Both stages are in the helper because the refusal this file is
/// about arrives from the LINKER — the front end leaves the unresolvable name as
/// a fixup target and the link is where it has nowhere to go — while a front-end
/// refusal for the same source would be just as correct an answer.
fn refusal(src: &str) -> Vec<String> {
    let module = match assemble(src, &Options::default()) {
        Ok(m) => m,
        Err(diags) => return diags.into_iter().map(|d| d.message).collect(),
    };
    match sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()) {
        Ok(_) => panic!("expected a refusal, the source assembled and linked"),
        Err(diags) => diags.into_iter().map(|d| d.message).collect(),
    }
}

fn head() -> &'static str {
    "\tcpu\t68000\n\tpadding\toff\n\torg\t0\n"
}

/// probe `p1`. The macro runs twice, four bytes apart, and reads its own label
/// back each time.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `$0004` on BOTH read lines, which is
/// what a global binding whose second expansion overwrote the first produces —
/// and is what sigil would emit if the definition were global and the linker did
/// not refuse it first. The two expansions are deliberately at different
/// addresses so that answer is a different byte string, not the same one.
#[test]
fn a_macro_body_label_reads_back_per_expansion_instance() {
    let src = format!(
        "{}mi\tmacro\nLi:\tdc.w\t$1111\n\tdc.w\tLi\n\tendm\n\tmi\n\tmi\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x11, 0x11, 0x00, 0x00, 0x11, 0x11, 0x00, 0x04, 0x44, 0x44],
        "asl p1: 1111 0000 / 1111 0004 / 4444"
    );
}

/// probe `p2`, the `end-start` shape s2's `s2.macrosetup.asm` actually writes.
/// The two expansions have DIFFERENT SIZES, so the difference `Ef-Sf` is $0003
/// in the first and $0005 in the second.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: any binding that did not give each
/// instance its own `Sf` and `Ef` produces the same difference twice, whichever
/// value it picks. The unequal `ds.b` sizes are what make the pair of
/// differences the discriminator rather than the pair of addresses.
#[test]
fn a_forward_reference_inside_the_body_reads_its_own_expansion() {
    let src = format!(
        "{}mf\tmacro\tn\nSf:\tds.b\tn\n\tdc.w\tEf-Sf\nEf:\n\tendm\n\tmf\t1\n\tmf\t3\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x05, 0x44, 0x44],
        "asl p2: ds.b 1 / 0003 / ds.b 3 / 0005 / 4444"
    );
}

/// probe `e12` of the `exitm` set — the shape that opened this parcel. A macro
/// body's label, read from FILE LEVEL.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `A0 A1 FF 00 00 00 01`, which is
/// what sigil emitted before this file existed, having bound `lbl12` at address
/// 1. asl reports `symbol undefined` and its symbol table holds nothing.
#[test]
fn an_expansion_label_is_undefined_at_file_level() {
    let src = format!(
        "{}m12\tmacro\n\tdc.b\t$A0\nlbl12:\tdc.b\t$A1\n\tendm\n\tm12\n\tm12\n\tdc.b\t$FF\n\tdc.l\tlbl12\n",
        head()
    );
    let msgs = refusal(&src);
    assert!(
        msgs.iter().any(|m| m.contains("lbl12") && m.contains("unresolved")),
        "expected an unresolved-symbol refusal naming `lbl12`, got: {msgs:?}"
    );
}

/// probe `p3`. NESTED: the inner macro writes the label, the OUTER body reads it
/// after the inner expansion has returned. asl: `#1010`, twice.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `1111 0000 1111 0004 4444` — the
/// bytes an "anything inside any expansion is visible to anything else inside an
/// expansion" rule produces. That rule passes the p4 test below and fails here,
/// and the two are the pair that pins the chain to the LIVE stack.
#[test]
fn an_inner_expansions_label_is_undefined_in_the_outer_body() {
    let src = format!(
        "{}inner3\tmacro\nNi:\tdc.w\t$1111\n\tendm\nouter3\tmacro\n\tinner3\n\tdc.w\tNi\n\tendm\n\touter3\n\touter3\n\tdc.w\t$4444\n",
        head()
    );
    let msgs = refusal(&src);
    assert!(
        msgs.iter().any(|m| m.contains("Ni") && m.contains("unresolved")),
        "expected an unresolved-symbol refusal naming `Ni`, got: {msgs:?}"
    );
}

/// probe `p4`. NESTED the other way: the OUTER macro writes the label and the
/// INNER macro it calls reads it. asl resolves it, per instance.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: a refusal, which is what an
/// exemption drawn around "the innermost expansion only" produces — and `$0004`
/// on both read lines, which is what a global binding produces. Three candidate
/// rules, three different observable answers.
#[test]
fn an_enclosing_expansions_label_is_visible_to_a_nested_one() {
    let src = format!(
        "{}inner4\tmacro\n\tdc.w\tNo\n\tendm\nouter4\tmacro\nNo:\tdc.w\t$2222\n\tinner4\n\tendm\n\touter4\n\touter4\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x22, 0x22, 0x00, 0x00, 0x22, 0x22, 0x00, 0x04, 0x44, 0x44],
        "asl p4: 2222 0000 / 2222 0004 / 4444"
    );
}

/// probe `p5`. A SIBLING expansion's label — both reads happen inside an
/// expansion, neither inside the one that wrote the name.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `$0000` and `$0002`, the addresses
/// the two `d5a` expansions wrote, under a rule that kept expansion labels in one
/// shared pool instead of one pool per instance.
#[test]
fn a_sibling_expansions_label_is_undefined() {
    let src = format!(
        "{}d5a\tmacro\nDx:\tdc.w\t$1111\n\tendm\nd5b\tmacro\n\tdc.w\tDx\n\tendm\n\td5a\n\td5b\n\td5a\n\td5b\n\tdc.w\t$4444\n",
        head()
    );
    let msgs = refusal(&src);
    assert!(
        msgs.iter().any(|m| m.contains("Dx") && m.contains("unresolved")),
        "expected an unresolved-symbol refusal naming `Dx`, got: {msgs:?}"
    );
}

/// probe `p6`. All three PC-label SPELLINGS, in one twice-run body: a colon
/// label on its own line, a colon-less column-0 label, and a label on a data
/// line.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: the second expansion's three
/// addresses ($0008/$000A/$000C) printed in the first expansion too. It also
/// discriminates a rule drawn around ONE spelling: exempting only the colon form
/// leaves `Cb` reading `$000A` in both expansions while `Ca` and `Cc` are right.
#[test]
fn all_three_pc_label_spellings_localize() {
    let src = format!(
        "{}msp\tmacro\nCa:\n\tdc.w\tCa\nCb\n\tdc.w\tCb\nCc:\tdc.w\t$1111\n\tdc.w\tCc\n\tendm\n\tmsp\n\tmsp\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x00, 0x00, 0x00, 0x02, 0x11, 0x11, 0x00, 0x04, 0x00, 0x08, 0x00, 0x0A, 0x11, 0x11,
            0x00, 0x0C, 0x44, 0x44
        ],
        "asl p6: 0000 0002 1111 0004 / 0008 000A 1111 000C / 4444"
    );
}

/// probe `p7`. The other three expansion DRIVERS, each running its body twice.
/// The namespace is per ITERATION, not per loop: `Ra` reads `$0000` then
/// `$0002`.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `$0000` twice (one namespace for the
/// whole loop, the first iteration winning) or `$0002` twice (one namespace, the
/// last winning) — and, for a change that localized macro bodies only, the
/// second address in both slots for all three loops.
#[test]
fn rept_irp_and_while_localize_per_iteration() {
    let src = format!(
        "{}\trept\t2\nRa:\n\tdc.w\tRa\n\tendm\n\tirp\tn,$11,$22\nIa:\n\tdc.b\tn,0\n\tdc.w\tIa\n\tendm\nWc\tset\t0\n\twhile\tWc<2\nWa:\n\tdc.w\tWa\nWc\tset\tWc+1\n\tendm\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![
            0x00, 0x00, 0x00, 0x02, 0x11, 0x00, 0x00, 0x04, 0x22, 0x00, 0x00, 0x08, 0x00, 0x0C,
            0x00, 0x0E, 0x44, 0x44
        ],
        "asl p7: rept 0000 0002 / irp 1100 0004 2200 0008 / while 000C 000E / 4444"
    );
}

/// probe `p8`. THE DIRECTION THAT MUST NOT BREAK — a file-level label read from
/// inside an expansion, backward and forward. If this stopped resolving nothing
/// would assemble, which is exactly why it is a fixture and not an assumption:
/// the exemption is drawn around the DEFINITION site, never the read site.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: a refusal, from a rule that treated
/// "reference evaluated inside an expansion" as the trigger. That version of the
/// change passes every other test in this file.
#[test]
fn a_file_level_label_still_resolves_from_inside_an_expansion() {
    let src = format!(
        "{}Gb:\n\tdc.w\t$1111\nmread\tmacro\n\tdc.w\tGb\n\tdc.w\tGf\n\tendm\n\tmread\n\tmread\nGf:\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x11, 0x11, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x0A, 0x44, 0x44],
        "asl p8: 1111 / 0000 000A / 0000 000A / 4444"
    );
}

/// probes `m18`/`m19` of the symbol-class set, from the other side. The VALUE-
/// BINDING forms written in a macro body are GLOBAL wherever they are written,
/// and must stay reachable from outside the expansion — asl resolves the `label`
/// directive's `Al` to `$0100` from file level while refusing the PC label
/// beside it.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: a refusal on `Al`, which is what an
/// exemption drawn around "any name declared inside an expansion" produces. That
/// is the single most likely way to get this change wrong, because "labels" and
/// "the `label` directive" are one word apart and land on opposite sides.
#[test]
fn a_value_binding_form_in_a_macro_body_stays_global() {
    let src = format!(
        "{}mlabdir\tmacro\nAl\tlabel\t$100\n\tendm\n\tmlabdir\nmpc\tmacro\nPc:\tdc.w\t$1111\n\tendm\n\tmpc\n\tdc.w\tAl\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x11, 0x11, 0x01, 0x00, 0x44, 0x44],
        "the `label` directive's Al reads $0100 from outside the expansion"
    );

    // …and the PC label beside it does not, in the same source shape.
    let src = format!(
        "{}mpc\tmacro\nPc:\tdc.w\t$1111\n\tendm\n\tmpc\n\tmpc\n\tdc.w\tPc\n\tdc.w\t$4444\n",
        head()
    );
    let msgs = refusal(&src);
    assert!(
        msgs.iter().any(|m| m.contains("Pc") && m.contains("unresolved")),
        "expected an unresolved-symbol refusal naming `Pc`, got: {msgs:?}"
    );
}

/// probe `p11`. The `label` DIRECTIVE read from INSIDE the body that declares
/// it — the direction the test above does not reach, and the one that actually
/// depends on `scan_plain_labels` excluding the directive.
///
/// This fixture exists because the mutation that removes `"label"` from that
/// exclusion list APPLIED CLEANLY AND LEFT THE FILE GREEN. The exclusion is
/// inert on the outside read: `directive_label` binds through the builder rather
/// than through `define_label`, so the name is written bare either way, and by
/// the time the outside read happens the namespace stack is empty. It is the
/// INSIDE read that breaks — the reference mangles to the expansion's key while
/// the declaration wrote the bare name, and nothing resolves.
///
/// INVOKED ONCE, and that is forced rather than sloppy: `m18` measured a second
/// `Al label $100` as `#1000 symbol double defined` even with the value
/// unchanged, so no two-instance version of this shape assembles. The
/// discriminator is the VALUE — `$100` is nowhere near the PC here.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `$0000`/`$0002`, if the directive
/// were read as a PC label; and a refusal, which is what removing `"label"` from
/// the exclusion produces and what this fixture was added to catch.
#[test]
fn the_label_directive_reads_back_from_inside_its_own_body() {
    let src = format!(
        "{}mlab\tmacro\nAl\tlabel\t$100\n\tdc.w\tAl\n\tendm\n\tmlab\n\tdc.w\tAl\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x01, 0x00, 0x01, 0x00, 0x44, 0x44],
        "asl p11: 0100 (inside the body) / 0100 (outside) / 4444"
    );
}

/// probe `p12`. The `label` directive with `*` — the PROGRAM COUNTER — as its
/// operand, read from outside the body that declares it. `p11` and the m18/m19
/// pair use `$100`, a constant nowhere near the PC; this is the spelling where
/// "the directive escaped the expansion" and "the reference fell back to
/// something else" can land on the same number.
///
/// THE EXPANSION SITS AT `$4`, NOT AT `$0`, AND THAT IS THE WHOLE FIXTURE. With
/// the macro first in a file at `org 0`, `*` is `$0000_0000` — and so is a
/// global default, so is an unresolved fixup left zeroed, and so is a zero-fill.
/// A pass at address zero is indistinguishable from three separate failures.
/// Four bytes of filler make the correct answer `$0000_0004`, which none of
/// them produces.
///
/// INVOKED ONCE, forced by the same measurement `p11` records: a second
/// `Xl label *` is `#1000 symbol double defined` under asl even though the two
/// expansions sit at different addresses. That refusal is its own test below.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `00 00 00 00`, from any of the
/// three failures above; `00 00 00 0A`, if `*` were read at the `dc.l` rather
/// than at the expansion; and a refusal, if the directive were localized to the
/// expansion that wrote it.
///
/// The same source is minted from asl as snippet vector
/// `as_label_directive_star_escapes_the_macro_body`; that vector carries asl's
/// own bytes, this test carries the rule and what else it could have said.
#[test]
fn the_label_directive_with_a_pc_operand_escapes_the_macro_body() {
    let src = format!(
        "{}\tdc.w\t$1111\n\tdc.w\t$2222\nmx\tmacro\nXl\tlabel\t*\n\tendm\n\tmx\n\tdc.l\tXl\n\tdc.w\t$4444\n",
        head()
    );
    assert_eq!(
        bytes(&src),
        vec![0x11, 0x11, 0x22, 0x22, 0x00, 0x00, 0x00, 0x04, 0x44, 0x44],
        "asl p12: 1111 / 2222 / 0000 0004 / 4444"
    );
}

/// The refusal half of the rule above, and the only evidence that reaches a
/// question BYTES cannot ask. A byte vector shows where the name resolved; it
/// cannot show the name lives in ONE namespace rather than one per expansion,
/// because a per-expansion `Xl` and a global `Xl` read back the same value
/// inside the body that wrote it.
///
/// Two expansions each declaring `Xl label *` collide: asl answers
/// `#1000 symbol double defined` and keeps the FIRST binding (`$2`), sigil
/// answers `symbol double defined: Xl`. Under an expansion-local rule there is
/// no collision to report at all, so the refusal is the discriminator.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: a clean assembly emitting
/// `00 00 00 02` or `00 00 00 04` — which is exactly what localizing the
/// `label` directive produces, and what this test exists to refuse.
#[test]
fn two_expansions_declaring_the_same_label_directive_collide() {
    let src = format!(
        "{}mx\tmacro\nXl\tlabel\t*\n\tendm\n\tdc.w\t$1111\n\tmx\n\tdc.w\t$2222\n\tmx\n\tdc.l\tXl\n",
        head()
    );
    let msgs = refusal(&src);
    assert!(
        msgs.iter()
            .any(|m| m.contains("Xl") && m.contains("double defined")),
        "expected a double-definition refusal naming `Xl`, got: {msgs:?}"
    );
}

/// probe `p13`. A CONSTANT-valued `label` in a macro body reached by a fixup the
/// front end must DEFER — and the only fixture in this file whose reference is
/// not folded in-pass.
///
/// EVERY OTHER FIXTURE HERE READS ITS NAME WITH A BACKWARD `dc.w`, WHICH IS
/// EXACTLY WHY THIS ONE EXISTS. `directive_label` writes the name into the
/// symbol environment either way; a `dc.w` behind the definition folds straight
/// out of that environment, so no such fixture can see whether the name was
/// ALSO placed as a relocatable label at the expansion's address. A `bra.w`
/// resolves from the SECTION's symbol table instead, and the two readings are
/// different displacements: `$0100` is the directive's value, `$0004` is where
/// the expansion sits.
///
/// This was found by a mutation that made every `label` claim a position at the
/// current PC. It applied cleanly and left the whole crate green, which reads
/// exactly like a line nothing depends on.
///
/// WHAT OTHER ANSWER COULD THIS HAVE GIVEN: `60 00 00 02`, the displacement to
/// the expansion, which is what a `label` that claims a position produces; and
/// a refusal, if the deferred reference found no symbol at all.
#[test]
fn a_constant_valued_label_directive_is_not_placed_at_the_expansion() {
    let src = "\tcpu\t68000\n\tpadding\toff\n\tphase\t0\n\
               mk\tmacro\nAl\tlabel\t$100\n\tnop\n\tendm\n\tbra.w\tAl\n\tmk\n\tdc.w\tAl\n";
    assert_eq!(
        bytes(src),
        vec![0x60, 0x00, 0x00, 0xFE, 0x4E, 0x71, 0x01, 0x00],
        "asl p13: bra.w to $0100 (6000 00FE) / nop / 0100"
    );
}
