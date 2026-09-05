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
//! `#1000` — the SAME-class redefinition of a CONSTANT — is pinned here too, and
//! it is the row with the sharpest edge, because asl raises it EVEN WHEN THE TWO
//! VALUES ARE IDENTICAL (`Aq equ 1` written twice, probe `m4.asm`). Every fixture
//! for it below therefore comes in both flavours: one where the value changes and
//! one where it does not, so a front end that merely notices a value moving fails
//! the equal-value row.
//!
//! THE ONE EXEMPTION, and it is not a softening — it is what asl does. A PC label
//! and an `enum` member written inside a macro / `rept` / `irp` / `while`
//! expansion are LOCAL to that expansion: they never enter asl's symbol table
//! (probes `m17.asm`, `m19.asm`, both `#1010 symbol undefined` from outside), so
//! a second expansion redeclaring one is silent and a later file-level constant
//! of the same name is a FIRST declaration (probe `m20.asm`). `equ`, `=` and the
//! `label` DIRECTIVE are global wherever they are written and keep the full rule
//! in the same position (probes `m18.asm`, `m20.asm`). Without the exemption the
//! rule refuses 97 sites in the s2 corpus that asl accepts, every one of them a
//! colon label in a twice-expanded macro.

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
/// THIS IS NOW A REAL GATE, and it was not one before. Under the crossing-only
/// rule a threaded class map was unobservable — re-executing a declaration gives
/// a name the SAME class both times, and only a crossing was checked, so no
/// mutation could have turned this red. With `#1000` in, the per-pass reset is
/// load-bearing: a class map seeded from the previous pass makes the SECOND pass
/// of every multi-pass program a wall of `symbol double defined`, which is
/// exactly the shape asl calls `2 passes / 0 errors`.
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

/// `#1000`, every constant-making form redeclared by every constant-making form
/// at FILE LEVEL. `equ`/`=` and the same-value row from probe `m4.asm`, the
/// colon label from `m3.asm` (`Fl:` written twice), the `label` directive and
/// the bare label from `m4.asm`, and the `phase`d label from `m15.asm`.
///
/// EVERY SHAPE COMES IN BOTH FLAVOURS. The SAME-value rows redeclare with the
/// identical value and the others with a different one, because asl's rule is
/// about a second DECLARATION and not about a value moving (`Aq equ 1` twice is
/// `#1000`; so is `m15.asm`'s `Bp:`, whose two addresses differ in both the
/// logical and the physical counter). A front end that refused only on a changed
/// value would pass half of this and fail the other half — which is the whole
/// reason both halves are here.
#[test]
fn a_constant_may_not_be_redefined_as_a_constant() {
    for (body, name, what) in [
        ("Aq\tequ\t1\nAq\tequ\t1", "Aq", "equ then equ, SAME value"),
        ("Aq\tequ\t1\nAq\tequ\t2", "Aq", "equ then equ, different value"),
        ("Be\tequ\t1\nBe\t=\t2", "Be", "equ then ="),
        ("Be\t=\t1\nBe\tequ\t1", "Be", "= then equ, SAME value"),
        ("Fl:\n\tdc.w\t$1111\nFl:\n\tdc.w\t$1111", "Fl", "colon label then colon label"),
        ("Dq\n\tdc.w\t$1111\nDq\tequ\t2", "Dq", "bare label then equ"),
        ("Fq\tlabel\t$100\nFq\tequ\t2", "Fq", "`label` directive then equ"),
        ("Fq\tlabel\t$100\nFq\tlabel\t$100", "Fq", "`label` twice, SAME value"),
        ("Hq:\tdc.w\t$1111\nHq\tequ\t2", "Hq", "label on a data line then equ"),
        ("Ge\tequ\t1\nGe:\n\tdc.w\t$1111", "Ge", "equ then a colon label"),
        ("\tenum\tBr=5\nBr\tequ\t2", "Br", "enum member then equ"),
        ("\tenum\tBr=5\n\tenum\tBr=5", "Br", "enum member twice, SAME value"),
        (
            "Ap:\n\tdc.w\t$1111\n\tphase\t$1000\nBp:\n\tdc.w\t$2222\n\tdephase\nBp:\n\tdc.w\t$2222",
            "Bp",
            "a label redeclared across phase/dephase, both counters moved",
        ),
    ] {
        let want = format!("symbol double defined: `{name}`");
        assert!(
            diags(body).contains(&want),
            "asl reports `#1000 symbol double defined` for {what}; accepting it binds a \
             name the reference assembler leaves at its FIRST value, so the two assemblers \
             read different numbers out of the same source. Got: {:?}",
            diags(body)
        );
    }
}

/// The rule counts EXECUTED declarations, not written ones. Probe `m16.asm`: the
/// same two-`equ` shape under `if 0` lists clean and under `if 1` is `#1000`, and
/// the surviving value is the FIRST one (`Bi : 7` in asl's symbol table after the
/// refused `Bi equ 9`).
///
/// This separates "a second declaration RAN" from "the name appears twice in the
/// file"; a front end keyed on the latter would refuse every `ifdef`-guarded
/// constant in the corpus.
#[test]
fn a_declaration_the_pass_never_reaches_is_not_a_redefinition() {
    assert!(
        accepted("Ai\tequ\t7\n\tif\t0\nAi\tequ\t9\n\tendif\n\tdc.w\tAi"),
        "asl lists the unexecuted arm clean (probe `m16.asm`)"
    );
    assert!(
        diags("Bi\tequ\t7\n\tif\t1\nBi\tequ\t9\n\tendif\n\tdc.w\tBi")
            .iter()
            .any(|m| m == "symbol double defined: `Bi`"),
        "the EXECUTED twin of the same shape IS #1000 — without this row the test above \
         proves only that the front end refuses nothing"
    );
}

/// THE NARROWING, and the measurements it is drawn on.
///
/// A PC label (colon, colon-less column-0, or on a data line) and an `enum`
/// member are LOCAL to the macro / `rept` / `irp` / `while` expansion they are
/// written in. asl expands one macro twice over each of those spellings and stays
/// silent (probe `m18.asm`), the names are absent from its symbol table, and
/// reading one from outside the expansion is `#1010 symbol undefined` (probes
/// `m17.asm`, `m19.asm`). The `label` DIRECTIVE and `equ` in the same position
/// are NOT localized: they list in the table, they resolve from outside, and a
/// second expansion of them IS `#1000` (`m18.asm` `Al`, `m12.asm` `Ar`,
/// `m13.asm` `Am`).
///
/// WHAT THIS GATE MUST FAIL: the un-narrowed rule — "any second constant
/// declaration is `#1000`" — which refuses 97 sites in the s2 corpus that asl
/// assembles, every one a colon label in a twice-expanded macro (`start:`/`end:`
/// in `s2.macrosetup.asm`, `start:` in `zoneanimdecl`, `__LABEL__Plc:` in
/// `plrlistheader`). It must equally fail the opposite over-correction — an
/// exemption drawn around "anything inside an expansion" — which would accept the
/// `equ` and `label` rows asl refuses.
#[test]
fn an_expansion_localizes_a_pc_label_and_an_enum_member_but_not_an_equ() {
    for (body, what) in [
        ("m\tmacro\nBm:\n\tdc.w\t$1111\n\tendm\n\tm\n\tm", "colon label, macro twice"),
        ("m\tmacro\nCl\n\tdc.w\t$1111\n\tendm\n\tm\n\tm", "bare label, macro twice"),
        ("m\tmacro\nDl:\tdc.w\t$2222\n\tendm\n\tm\n\tm", "label on a data line, macro twice"),
        ("m\tmacro\n\tenum\tBe=5\n\tendm\n\tm\n\tm", "enum member, macro twice"),
        ("\trept\t2\nBr:\n\tdc.w\t$1111\n\tendm", "colon label, rept 2"),
        ("\tirp\tn,1,2\nEl:\n\tdc.w\tn\n\tendm", "colon label, irp over two items"),
        (
            "Wc\tset\t0\n\twhile\tWc<2\nFl:\n\tdc.w\t$3333\nWc\tset\tWc+1\n\tendm",
            "colon label, while over two iterations",
        ),
        // THE MIXED ORDERS (probe `m20.asm`). These are what force the exemption
        // to skip the RECORDING and not merely the refusal: an expansion-local
        // name is not a symbol asl has, in either direction.
        (
            "Zr:\n\tdc.w\t$3333\n\trept\t1\nZr:\n\tdc.w\t$3333\n\tendm",
            "a file label, then the same name in a rept",
        ),
        (
            "\trept\t1\nPr:\n\tdc.w\t$1111\n\tendm\nPr\tequ\t$99\n\tdc.w\tPr",
            "a rept label, then a file-level equ of the same name",
        ),
        (
            "m\tmacro\n\tenum\tQe=5\n\tendm\n\tm\nQe\tequ\t$99\n\tdc.w\tQe",
            "a macro enum member, then a file-level equ of the same name",
        ),
    ] {
        assert_eq!(
            diags(body),
            Vec::<String>::new(),
            "asl lists {what} clean — the name is local to the expansion and never enters \
             its symbol table, so refusing it refuses source the reference assembler builds \
             (this exact shape is 97 sites in the s2 corpus)"
        );
    }
    // …and the two forms that are global wherever they are written keep the rule.
    for (body, name, what) in [
        ("m\tmacro\nAl\tlabel\t$100\n\tendm\n\tm\n\tm", "Al", "`label` directive, macro twice"),
        ("m\tmacro\nAm\tequ\t7\n\tendm\n\tm\n\tm", "Am", "equ, macro twice"),
        ("\trept\t2\nAr\tequ\t7\n\tendm", "Ar", "equ, rept 2"),
        (
            "m\tmacro\nRl\tlabel\t$100\n\tendm\n\tm\nRl\tequ\t$99",
            "Rl",
            "a macro `label`, then a file-level equ",
        ),
        ("m\tmacro\nSl\tequ\t7\n\tendm\n\tm\nSl\tequ\t$99", "Sl", "a macro equ, then a file-level equ"),
    ] {
        let want = format!("symbol double defined: `{name}`");
        assert!(
            diags(body).contains(&want),
            "asl reports #1000 for {what} — the exemption is for PC labels and `enum` \
             members, NOT for everything an expansion happens to contain. Got: {:?}",
            diags(body)
        );
    }
}
