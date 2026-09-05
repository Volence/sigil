//! The half of `disp(An)`-versus-`function(args)` that byte vectors cannot hold:
//! the REFUSALS, and the shapes whose only evidence is which name a diagnostic
//! blames.
//!
//! `tests/asl_snippets.rs` carries the byte side from asl-minted goldens
//! (`as_disp_name_that_is_also_a_function`,
//! `as_insn2op_zero_offset_arms_with_id_also_a_function`). Bytes are silent
//! about a source asl refuses, and silent about WHICH of two candidate names a
//! refusal names — and that name is the whole rule here.
//!
//! THE RULE, and the asl rows it is derived from (AS V1.42 Beta [Bld 212],
//! `s2disasm/build_tools/Linux-x86_64/asl`, md5
//! `0dee1f98e6480a4783d27ffd8b90896f`, run `-xx -n -q -A -L -U -i .`). The
//! digest is the identity; four builds in this workspace print that same version
//! string, and this one substitutes an uninitialized word for any operand it
//! declined to give a value — see `docs/superpowers/notes/asl-reference/`.
//! In a 68000 operand asl peels the trailing `(An)`/`(An,Xn)` addressing-mode
//! group off BEFORE it evaluates anything, so the name in front of it is looked
//! up as a SYMBOL and never as a user `function`:
//!
//! ```text
//! dsp     = $2A
//! dsp     function p,(p*7)+$100
//!    7/    1000 : 337C 1234 002A         move.w  #$1234,dsp(a1)
//!    9/    1006 : 337C 1234 002B         move.w  #$1234,1+dsp(a1)
//! ```
//!
//! With no equate of that name asl states the same rule as a refusal — the
//! caret sits under `konst`, not under `a1`:
//!
//! ```text
//! konst   function p,$3C7
//! > > > ta.asm(5):16: error #1010: symbol undefined
//! > > > konst
//! > > >  move.w #$1234,konst(a1)
//! > > >                ~~~~~
//! ```
//!
//! The peel is driven by the BASE alone, not by whether the rest of the group is
//! a usable index — a two-element group is an addressing mode with a bad index,
//! never a two-argument call:
//!
//! ```text
//! > > > te.asm(6): error #1350: addressing mode not allowed here
//! > > >  move.w #$1234,1+dsp(a1,zz)
//! ```
//!
//! And the exclusion is the GROUP, not the operand: a call in the displacement
//! still expands, with `(a1)` still the base.
//!
//! ```text
//!    9/    1004 : 337C 1234 0117         move.w  #$1234,dsp(k)+2(a1)   ; k = 3
//! ```
//!
//! Where sigil's wording differs from asl's, these assert SIGIL's wording; the
//! two assemblers are not required to phrase a refusal alike, only to refuse the
//! same programs and to blame the same name.

use sigil_frontend_as::{assemble, Options};

/// Assemble, expecting REFUSAL, and hand back the diagnostic messages.
fn refusal(src: &str) -> Vec<String> {
    let diags = assemble(src, &Options::default())
        .err()
        .unwrap_or_else(|| panic!("expected a refusal, the source assembled"));
    diags.into_iter().map(|d| d.message).collect()
}

/// A name that is ONLY a `function`, written where a displacement belongs, is an
/// undefined SYMBOL — asl `error #1010`, underlining `konst`.
///
/// This is the row the byte goldens cannot carry, and the one that says the peel
/// is unconditional rather than a preference for whichever reading resolves: no
/// equate exists, the function does, and the operand is still refused.
///
/// MUST FAIL if the trailing group is handed to `expand_calls`: the body `$3C7`
/// never mentions its parameter, so the call folds to a constant and the line
/// assembles clean as `move.w #$1234,($3C7).w` — a refusal turning into six
/// bytes of a different instruction, with no diagnostic at all.
#[test]
fn a_function_only_name_over_an_address_register_is_an_undefined_symbol() {
    let msgs = refusal(
        "\tcpu 68000\n\
         konst\tfunction p,$3C7\n\
         \tmove.w\t#$1234,konst(a1)\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("konst")),
        "the refusal must blame `konst`, the name in displacement position — got {msgs:?}"
    );
    assert!(
        !msgs.iter().any(|m| m.contains("`a1`")),
        "`a1` is the addressing-mode base, never an expression leaf — got {msgs:?}"
    );
}

/// The same shape reached the way the corpus reaches it: through `insn2op`'s
/// `1+y` arm, with `id` spelled BOTH ways (`s2.constants.asm:15` an object-record
/// offset, `:438` a pointer-table index function). Before the peel this reported
/// `unresolved symbol \`a1\` in operand` 98 times at `s2.macrosetup.asm:127`
/// alone.
///
/// MUST FAIL if `1+id(a1)` is expanded as a call: `((a1-offset)/ptrsize+idstart)`
/// puts `a1` in expression position and the assembly is refused.
#[test]
fn the_insn2op_displacement_arm_takes_the_equate_not_the_function() {
    let src = "\tcpu 68000\n\
         id\t= 0\n\
         offset\t:= $40\n\
         ptrsize\t:= 2\n\
         idstart\t:= 6\n\
         id\tfunction ptr,((ptr-offset)/ptrsize+idstart)\n\
         \tmove.b\t#$B7,1+id(a1)\n";
    assemble(src, &Options::default()).unwrap_or_else(|d| {
        panic!(
            "`1+id(a1)` must read as a displacement — got {:?}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
}

/// A two-element trailing group is an addressing mode with a bad index register
/// (asl `error #1350`), never a two-argument call. Both assemblers refuse; the
/// test pins that sigil refuses it as an INDEX problem, which is only reachable
/// once the group has been peeled.
///
/// MUST FAIL if the group is expanded as a call: `dsp` takes one parameter, so
/// the two-argument form leaves `dsp` unexpanded and the complaint moves off the
/// index register entirely.
#[test]
fn a_two_element_trailing_group_is_an_addressing_mode_not_a_two_arg_call() {
    let msgs = refusal(
        "\tcpu 68000\n\
         dsp\t= $2A\n\
         dsp\tfunction p,(p*7)+$100\n\
         zz\t= 9\n\
         \tmove.w\t#$1234,1+dsp(a1,zz)\n",
    );
    assert!(
        msgs.iter().any(|m| m.contains("zz") && m.contains("index")),
        "must refuse `zz` as the index register of a peeled `(An,Xn)` group — got {msgs:?}"
    );
}

/// The exclusion is the trailing GROUP, not the operand: a `function` call that
/// sits in the displacement expression still expands, and `(a1)` is still the
/// base. asl: `337C 1234 0117` for `#$1234,dsp(k)+2(a1)` with `k = 3`.
///
/// This is the guard in the other direction. MUST FAIL if the fix is widened
/// from "hold the trailing group back" to "never expand calls in an operand":
/// `dsp` then resolves to its equate `$2A` and the displacement becomes `$2C`,
/// not `$117`.
#[test]
fn a_call_in_the_displacement_still_expands() {
    let module = assemble(
        "\tcpu 68000\n\
         \tphase 0\n\
         dsp\t= $2A\n\
         dsp\tfunction p,(p*7)+$100\n\
         k\t= 3\n\
         \tmove.w\t#$1234,dsp(k)+2(a1)\n",
        &Options::default(),
    )
    .unwrap_or_else(|d| {
        panic!(
            "the displacement's call must still expand — got {:?}",
            d.iter().map(|x| &x.message).collect::<Vec<_>>()
        )
    });
    let resolved =
        sigil_link::resolve_layout(&module.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
    assert_eq!(
        sigil_link::flatten(&linked, 0x00),
        vec![0x33, 0x7C, 0x12, 0x34, 0x01, 0x17],
        "asl emits 337C 1234 0117 — displacement (3*7)+$100+2"
    );
}
