//! asl's two SYMBOL CLASSES, and the crossings it refuses.
//!
//! asl does not have one "bind a symbol" operation spelled several ways. It has
//! a CONSTANT class (`equ`, `=`, a colon label, a column-0 bare label, the
//! `label` directive, an `enum` member) and a VARIABLE class (`set`, `eval`,
//! `:=`), and it refuses every crossing between them:
//!
//! | | second declaration |
//! |---|---|
//! | const → const | `#1000 symbol double defined` |
//! | const → var | `#2030 constants cannot be redefined as variables` |
//! | var → const | `#2035 variables cannot be redefined as constants` |
//! | var → var | accepted, silently, value updates |
//!
//! This front end used to do NOTHING at either crossing. That is the permissive
//! direction, and a permissive front end is invisible to a diagnostic count: a
//! source asl refuses outright assembled here, emitted bytes, and exited 0. So
//! this file pins ACCEPTANCE and REFUSAL as a pair — a front end that refuses
//! everything fails it exactly as hard as one that refuses nothing.
//!
//! Every expectation is read off the listing of `asl` 1.42 Beta Bld 212 — the
//! binary committed at `s1disasm/build_tools/Linux-x86_64/asl` — run with Sonic
//! 1's own flags (`-xx -n -q -A -L -U -i .`) over the probes committed under
//! `docs/superpowers/notes/2026-09-04-as-symbol-class-probes/`. Nothing here is
//! a reading of the semantics; each row is a listing row.
//!
//! The two crossings, from probes `m1.asm` and `m2.asm`:
//!
//! ```text
//! > > > m1.asm(10): error #2030: constants cannot be redefined as variables
//! > > > Ce
//! > > > Ce set 2
//! > > > m2.asm(6): error #2035: variables cannot be redefined as constants
//! > > > As
//! > > > As equ 2
//! ```
//!
//! WHAT THIS FILE DOES NOT PIN. `#1000`, the SAME-class redefinition of a
//! constant, is not implemented and is not asserted here. asl raises it — and
//! raises it even when the value is unchanged (`Aq equ 1` written twice, probe
//! `m4.asm`) — but its population is every duplicated label and every
//! twice-included header, which is a different measurement and a different
//! parcel. Asserting a rule this front end does not have would make the file
//! describe a compiler nobody built.

use sigil_frontend_as::{assemble, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

fn diags(body: &str) -> Vec<String> {
    match assemble(&format!("{HEAD}{body}\n"), &Options::default()) {
        Ok(_) => Vec::new(),
        Err(ds) => ds.into_iter().map(|d| d.message).collect(),
    }
}

fn accepted(body: &str) -> bool {
    assemble(&format!("{HEAD}{body}\n"), &Options::default()).is_ok()
}

/// The whole point of the VARIABLE class, and the reason this rule cannot be
/// "refuse every redefinition". Probe `m2.asm` lines 9-14: `Cs set 1` /
/// `Cs set 2`, `Ds set 1` / `Ds eval 2` and `Es set 1` / `Es := 2` all list
/// clean, so the three spellings name ONE class rather than three directives
/// that merely resemble each other.
#[test]
fn a_variable_may_be_reassigned_and_the_three_spellings_are_one_class() {
    for body in [
        "Cs\tset\t1\nCs\tset\t2",
        "Ds\tset\t1\nDs\teval\t2",
        "Es\tset\t1\nEs\t:=\t2",
        "Es\t:=\t1\nEs\tset\t2\nEs\teval\t3",
        // The accumulator shape the corpus actually writes — a `set` whose
        // right-hand side reads its own current value.
        "i\tset\t0\n\tdc.b\ti\ni\tset\ti+5\n\tdc.b\ti",
    ] {
        assert_eq!(
            diags(body),
            Vec::<String>::new(),
            "asl lists `{}` clean — a reassignable symbol MUST stay reassignable",
            body.replace('\n', " / ")
        );
    }
}

/// `#2030`, every form that puts a name in the constant class. `equ`/`=` from
/// probe `m1.asm`; the colon label from `m3.asm`; the bare label, the `label`
/// directive and a label on a data line from `m4.asm`; the `enum` member from
/// `m5.asm`.
#[test]
fn a_constant_may_not_be_redefined_as_a_variable() {
    for (body, name, what) in [
        ("Ce\tequ\t1\nCe\tset\t2", "Ce", "equ then set"),
        ("De\tequ\t1\nDe\teval\t2", "De", "equ then eval"),
        ("Ee\tequ\t1\nEe\t:=\t2", "Ee", "equ then :="),
        ("Be\t=\t1\nBe\tset\t2", "Be", "= then set"),
        ("Cl:\n\tdc.w\t$1111\nCl\tset\t2", "Cl", "colon label then set"),
        ("Eq\n\tdc.w\t$1111\nEq\tset\t2", "Eq", "column-0 bare label then set"),
        ("Gq\tlabel\t$100\nGq\tset\t2", "Gq", "`label` directive then set"),
        ("Hq:\tdc.w\t$1111\nHq\tset\t2", "Hq", "label on a data line then set"),
        ("\tenum\tAr=5\nAr\tset\t2", "Ar", "enum member then set"),
    ] {
        let want = format!("constants cannot be redefined as variables: `{name}`");
        assert!(
            diags(body).contains(&want),
            "asl reports `#2030 constants cannot be redefined as variables` for {what}; \
             accepting it assembles a source the reference assembler will not, in silence. \
             Got: {:?}",
            diags(body)
        );
    }
}

/// `#2035`, the direction the booked row did not mention at all. `set` then
/// `equ`/`=` from probe `m2.asm`; `set` then each of the four constant-MAKING
/// forms from `m9.asm`, where asl refuses all four and leaves the variable's
/// value standing (`*Av : 1`, `*Bv : 1`, `*Cv : 1`, `*Dv : 1`).
#[test]
fn a_variable_may_not_be_redefined_as_a_constant() {
    for (body, name, what) in [
        ("As\tset\t1\nAs\tequ\t2", "As", "set then equ"),
        ("Bs\tset\t1\nBs\t=\t2", "Bs", "set then ="),
        ("Av\tset\t1\n\tdc.w\t$1111\nAv:\n\tdc.w\t$1111", "Av", "set then a colon label"),
        ("Bv\tset\t1\n\tdc.w\t$1111\nBv\n\tdc.w\t$1111", "Bv", "set then a bare label"),
        ("Cv\tset\t1\nCv\tlabel\t$100", "Cv", "set then the `label` directive"),
        ("Dv\tset\t1\n\tenum\tDv=5", "Dv", "set then an enum member"),
        ("Ds\teval\t1\nDs\tequ\t2", "Ds", "eval then equ"),
        ("Es\t:=\t1\nEs\tequ\t2", "Es", ":= then equ"),
    ] {
        let want = format!("variables cannot be redefined as constants: `{name}`");
        assert!(
            diags(body).contains(&want),
            "asl reports `#2035 variables cannot be redefined as constants` for {what}. \
             Got: {:?}",
            diags(body)
        );
    }
}

/// The class belongs to the DECLARING DIRECTIVE, never to the value. Probe
/// `m5.asm`: a STRING `equ` followed by a string `set` is #2030 and a FLOAT
/// `equ` followed by a float `set` is #2030, exactly as the integer forms are —
/// while the same two shapes written `set`-then-`set` list clean.
#[test]
fn the_class_is_the_directive_and_not_the_value_type() {
    for (body, what) in [
        ("Cr\tequ\t\"abc\"\nCr\tset\t\"def\"", "string equ then string set"),
        ("Er\tequ\t1.5\nEr\tset\t2.5", "float equ then float set"),
    ] {
        assert!(
            !accepted(body),
            "asl reports #2030 for {what} — the value's TYPE does not enter the rule"
        );
    }
    for (body, what) in [
        ("Dr\tset\t\"abc\"\nDr\tset\t\"def\"", "string set then string set"),
        ("Fr\tset\t1.5\nFr\tset\t2.5", "float set then float set"),
    ] {
        assert!(
            accepted(body),
            "asl lists {what} clean — a string or float variable reassigns like any other"
        );
    }
}

/// asl's rule is PER PASS: probe `m7.asm` forces two passes with a forward
/// branch, so its `Ap equ 1` and its `Lab:` each EXECUTE TWICE, and asl reports
/// `2 passes / 0 errors`.
///
/// WHAT THIS TEST CAN AND CANNOT PROVE, stated because the answer surprised the
/// author. The implementation keeps its class map per-pass rather than threading
/// it through `run_impl`'s env seeding, and that choice is correct — but it is
/// NOT OBSERVABLE under the crossing-only rule this front end implements, so
/// this test could not have gone red for a threaded map. Re-executing the same
/// declaration on a later pass gives a name the SAME class both times, and only
/// a CROSSING is checked; `#1000`, the rule that would notice a name declared
/// twice in one class, is not implemented (see the module doc). The per-pass
/// reset becomes load-bearing, and this test becomes a real gate on it, the day
/// `#1000` lands. Until then it is regression coverage for the ordinary
/// multi-pass shape and a record of asl's `2 passes / 0 errors`, and it is
/// labelled as such rather than presented as the proof it cannot be.
#[test]
fn re_executing_a_declaration_on_a_later_pass_is_not_a_redefinition() {
    assert_eq!(
        diags("\tbra.w\tFwd\nAp\tequ\t1\nLab:\n\tdc.w\tAp\nFwd:\n\tdc.w\t$4444"),
        Vec::<String>::new(),
        "asl assembles this in 2 passes with 0 errors; a forward reference must not \
         turn every constant in the file into a redefinition"
    );
}

/// The name the rule is keyed on is the QUALIFIED one. Probe `m6.asm` line 22
/// quotes `Sc1.loc` — the local `.loc` under scope `Sc1` — and probe `m9.asm`
/// shows two different scopes' `.loc` living side by side, so a rule keyed on
/// the written spelling would refuse a legal program.
#[test]
fn a_local_name_is_classed_per_scope_not_per_spelling() {
    assert!(
        diags("Sc1:\n.loc\tequ\t1\n.loc\tset\t2")
            .iter()
            .any(|m| m == "constants cannot be redefined as variables: `Sc1.loc`"),
        "asl quotes the QUALIFIED name `Sc1.loc` in its #2030 row"
    );
    assert!(
        accepted("Sc1:\n.loc\tset\t1\nSc2:\n.loc\tset\t3"),
        "two scopes' `.loc` are two symbols; asl lists this clean (probe `m6.asm`)"
    );
    assert!(
        accepted("Sc1:\n.loc\tequ\t1\nSc2:\n.loc\tset\t3"),
        "`Sc1.loc` being a constant says nothing about `Sc2.loc`"
    );
}

/// A macro expansion declares into the CALLER's class space, not a private one.
/// Probe `m6.asm`: `Am equ 1` then a macro whose body is `n set 9` called as
/// `mset Am` is #2030, and the mirror `Bm set 1` / `mequ Bm` is #2035. asl
/// attributes both to the expansion site (`m6.asm(12) mset(1)`).
#[test]
fn a_macro_body_declares_into_the_callers_class_space() {
    assert!(
        diags("mset\tmacro\tn\nn\tset\t9\n\tendm\nAm\tequ\t1\n\tmset\tAm")
            .iter()
            .any(|m| m == "constants cannot be redefined as variables: `Am`"),
        "asl reports #2030 at the expansion site for a macro-body `set` over a constant"
    );
    assert!(
        diags("mequ\tmacro\tn\nn\tequ\t9\n\tendm\nBm\tset\t1\n\tmequ\tBm")
            .iter()
            .any(|m| m == "variables cannot be redefined as constants: `Bm`"),
        "asl reports #2035 at the expansion site for a macro-body `equ` over a variable"
    );
    // …and the same macro over a name the caller has NOT declared is ordinary.
    assert!(
        accepted("mset\tmacro\tn\nn\tset\t9\n\tendm\n\tmset\tFresh\n\tdc.b\tFresh"),
        "the rule is about a CROSSING; a first declaration through a macro is not one"
    );
}

/// A `rept` re-running the same `set` line is reassignment, not redefinition
/// (probe `m6.asm` lines 16-18, listed clean), and a `set` seeded by
/// `Options::defines` is not a crossing either — sigil's defines carry NO class,
/// deliberately, so the first in-source declaration establishes one. That is a
/// stated deviation from asl, whose command-line `-D` define is a VARIABLE
/// (probe `m8.asm` under `-D Dw=1` makes `Dw equ 2` a #2035); the reason is that
/// `guarded_defines` are `.emp`-owned CONSTANTS with their own loud
/// `[defines.collision]` refusal, and classing them as variables would refuse
/// the one spelling that guard exists to permit.
#[test]
fn a_repeated_set_and_a_seeded_define_are_not_crossings() {
    assert!(
        accepted("Cm\tset\t0\n\trept\t2\nCm\tset\tCm+1\n\tendm\n\tdc.b\tCm"),
        "asl lists a `rept`ed `set` clean — the same line running twice is reassignment"
    );
    let opts = Options {
        defines: vec![("Seeded".to_string(), 1)],
        ..Options::default()
    };
    assert!(
        assemble(&format!("{HEAD}Seeded\tequ\t2\n\tdc.b\tSeeded\n"), &opts).is_ok(),
        "a seeded define carries no class, so an in-source `equ` of it establishes one \
         rather than colliding with a seed — `Options::defines`' own doc says an in-file \
         `=`/`equ` of such a name WINS, which the code-gate and game-config overrides rely on"
    );
}
