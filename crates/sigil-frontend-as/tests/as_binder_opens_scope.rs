//! A value binder in AS's label field OPENS a local-label scope, exactly as a
//! PC label does.
//!
//! ```text
//! Parent:
//!         nop
//! Var     set     5
//! .lq:
//!         nop
//! ```
//!
//! `.lq` belongs to `Var`, not to `Parent`. Read straight out of the reference
//! build's own symbol table (md5 `61e672562465725a8c102288a7da9098`,
//! `s1disasm/build_tools/Linux-x86_64/asl`, flags `-xx -n -q -A -L -U -i .`,
//! exit 0, pass loop complete):
//!
//! ```text
//! *Parent :   1000 C |  *Var : 5 - |  *Var.lq : 1002 C
//! ```
//!
//! There is no `Parent.lq` row. Referenced the other way round, `dc.l Parent.lq`
//! is `error #1010: symbol undefined` and exit 2 on the same build.
//!
//! THE DIVERGENCE THIS FILE CLOSES IS SILENT IN ONE DIRECTION. Before the fix
//! sigil resolved `Parent.lq` and emitted `$00001002` for a symbol the reference
//! assembler says does not exist -- an answer, not a refusal.
//!
//! IT IS NOT ONLY `set`. Measured over twelve spellings
//! (`docs/superpowers/notes/2026-09-06-as-set-opens-scope-probes/matrix.sh`),
//! every value-binding form opens a scope: `set`, `equ`, `=`, `:=`, `eval`,
//! each with or without the decorative colon, the `set NAME,value` and
//! `eval NAME,value` operand forms, and the string-valued spellings of `set`
//! and `equ`. `label` and a plain PC label were already right and are the two
//! controls.
//!
//! WHAT EACH TEST MUST FAIL ON, before the fix. Seven fail and five pass, and
//! the split is the bar:
//!
//! * the four `_binds_the_local_to_the_binder` tests and
//!   `a_set_inside_a_macro_opens_the_scope_in_the_caller` refuse with
//!   `unresolved symbol \`Var.lq\`` / `\`Ms.lq\`` -- the symbol does not exist;
//! * the two `_is_no_longer_the_parent` tests ASSEMBLE, emitting `$00000002`
//!   for the reference assembler's undefined symbol, which is the silent half;
//! * the remaining five PASS before AND after, and they are what an over-broad
//!   fix breaks: a scope opened for a dotted binder, or opened before the
//!   binder's own operand was read, or opened by a line that bound nothing,
//!   fails one of them. All three of those shapes are in the corpus; the
//!   divergent one is not.

use sigil_frontend_as::{assemble, Options};

/// Assemble, expecting SUCCESS, and hand back the linked bytes.
fn bytes(src: &str) -> Vec<u8> {
    let module = assemble(src, &Options::default())
        .unwrap_or_else(|d| panic!("expected an assembly, refused: {d:?}"));
    let linked = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// Assemble AND LINK, expecting a refusal from either, and hand back the
/// diagnostic messages.
///
/// THE LINK IS NOT OPTIONAL HERE and getting it wrong cost this file a round.
/// A `dc.l` of a name the front end does not know is not a front-end error: it
/// is emitted as a symbolic fixup and refused by the linker,
/// `unresolved symbol \`X\` for fixup in section …`. A `refusal` that stopped
/// at [`assemble`] reported "the source assembled" for a source that does not
/// link, which reads as the defect being unfixed when it is closed.
fn refusal(src: &str) -> Vec<String> {
    let module = match assemble(src, &Options::default()) {
        Ok(m) => m,
        Err(diags) => return diags.into_iter().map(|d| d.message).collect(),
    };
    let diags = sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled AND linked"));
    diags.into_iter().map(|d| d.message).collect()
}

/// `Parent:` / `nop` / `<binder>` / `.lq:` / `nop` / `dc.l <reference>`.
///
/// Two `nop`s put `.lq` at offset 2, so a resolved reference emits
/// `00 00 00 02` and the whole image is `4E 71 4E 71 00 00 00 02`. `lq` occurs
/// exactly once in the source and `Parent` and `Var` are distinct names, so the
/// spelling that resolves names the parent outright -- a local that existed
/// under both candidates could not tell the two apart.
fn probe(binder: &str, reference: &str) -> String {
    format!(
        "\tcpu 68000\n\tphase 0\n\
         Parent:\n\tnop\n\
         {binder}\n\
         .lq:\n\tnop\n\
         \tdc.l\t{reference}\n"
    )
}

const RESOLVED: &[u8] = &[0x4E, 0x71, 0x4E, 0x71, 0x00, 0x00, 0x00, 0x02];

#[test]
fn a_set_binds_the_local_to_the_binder() {
    assert_eq!(bytes(&probe("Var\tset\t5", "Var.lq")), RESOLVED);
}

#[test]
fn an_equ_binds_the_local_to_the_binder() {
    assert_eq!(bytes(&probe("Var\tequ\t5", "Var.lq")), RESOLVED);
}

/// The operand-field spelling, where the name is not in asl's label field at
/// all. asl opens the scope for it just the same (probe `s05`).
#[test]
fn the_comma_operand_form_binds_the_local_to_the_binder() {
    assert_eq!(bytes(&probe("\tset\tVar,5", "Var.lq")), RESOLVED);
}

/// A string-valued binder binds a symbol like any other and opens a scope like
/// any other (probe `s07`).
#[test]
fn a_string_set_binds_the_local_to_the_binder() {
    assert_eq!(bytes(&probe("Var\tset\t\"ab\"", "Var.lq")), RESOLVED);
}

/// The silent half, and the reason this parcel is not cosmetic: sigil answered
/// `$00001002` here for a symbol asl calls undefined.
#[test]
fn the_preceding_label_is_no_longer_the_parent_after_a_set() {
    let msgs = refusal(&probe("Var\tset\t5", "Parent.lq"));
    assert!(
        msgs.iter().any(|m| m.contains("Parent.lq")),
        "the refusal must name `Parent.lq`, got {msgs:?}"
    );
}

#[test]
fn the_preceding_label_is_no_longer_the_parent_after_an_equ() {
    let msgs = refusal(&probe("Var\tequ\t5", "Parent.lq"));
    assert!(
        msgs.iter().any(|m| m.contains("Parent.lq")),
        "the refusal must name `Parent.lq`, got {msgs:?}"
    );
}

/// A DOTTED binder name opens nothing: `.b set 5` under `Outer:` leaves the
/// next local at `Outer.zz` (probe `s03`). This is the shape the corpus is
/// actually made of -- all five `set` sites the queue row named are dotted -- so
/// a fix that opened a scope for them would break real sources while closing
/// nothing.
///
/// PASSES BEFORE AND AFTER. It fails only against an over-broad fix.
#[test]
fn a_dotted_binder_opens_no_scope() {
    assert_eq!(bytes(&probe(".b\tset\t5", "Parent.lq")), RESOLVED);
}

/// The binder's OWN right-hand side is read in the PREVIOUS scope: `Vr set
/// .prev` under `Parent:` resolves `.prev` as `Parent.prev` and binds its value
/// (probe `s04` -- the reference build binds `Vr : 1002` there and only then
/// starts refusing `.prev` on the following line).
///
/// PASSES BEFORE AND AFTER. A fix that opened the scope before evaluating the
/// operand would refuse this, and would refuse it on every chained `set` in the
/// corpus.
#[test]
fn the_right_hand_side_is_evaluated_in_the_previous_scope() {
    let src = "\tcpu 68000\n\tphase 0\n\
               Parent:\n\tnop\n\
               .prev:\n\tnop\n\
               Vr\tset\t.prev\n\
               \tdc.l\tVr\n";
    assert_eq!(bytes(src), RESOLVED);
}

/// A `set` inside a macro expansion opens the scope in the CALLER, not in the
/// expansion's own unspellable frame (probe `s09`: the reference table reads
/// `Ms.uu : 1002 C`).
///
/// MUST FAIL before the fix, with `unresolved symbol \`Ms.lq\``.
#[test]
fn a_set_inside_a_macro_opens_the_scope_in_the_caller() {
    let src = "\tcpu 68000\n\tphase 0\n\
               opener\tmacro\n\
               Ms\tset\t5\n\
               \tendm\n\
               Parent:\n\tnop\n\
               \topener\n\
               .lq:\n\tnop\n\
               \tdc.l\tMs.lq\n";
    assert_eq!(bytes(src), RESOLVED);
}

/// A binder whose right-hand side does not evaluate opens NO scope: the
/// reference table for probe `s08` reads `Anchor.tt`, not `Bd.tt`, so the local
/// still belongs to the preceding label.
///
/// ONLY THE ATTACHMENT IS UNDER TEST HERE. asl also REFUSES this source
/// (`#1010 symbol undefined`, exit 2) and sigil assembles it silently; that is
/// a real divergence and a different one, it is not closed by this parcel, and
/// asserting a refusal here would make this test fail for a reason that has
/// nothing to do with scopes. What is asserted is the part this parcel owns:
/// the scope did not move on a line that bound nothing.
///
/// PASSES BEFORE AND AFTER. It fails against a fix that opens the scope from
/// the label field alone, without regard to whether a value was bound.
#[test]
fn a_binder_whose_value_does_not_resolve_opens_no_scope() {
    assert_eq!(bytes(&probe("Var\tset\tnosuchsymbol", "Parent.lq")), RESOLVED);
}

/// The control that was already right, kept in the file so the two halves of
/// the rule are read together: a plain PC label opens a scope, and always did.
#[test]
fn a_plain_label_still_opens_a_scope() {
    assert_eq!(bytes(&probe("Other:", "Other.lq")), RESOLVED);
}

/// The other control: the `label` DIRECTIVE, which also already opened a scope.
#[test]
fn the_label_directive_still_opens_a_scope() {
    assert_eq!(bytes(&probe("Other\tlabel\t*", "Other.lq")), RESOLVED);
}
