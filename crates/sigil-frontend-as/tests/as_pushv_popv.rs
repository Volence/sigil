//! asl's `pushv` / `popv`: save the values of reassignable symbols on a named
//! stack and restore them later.
//!
//! Sonic 2 brackets its last $15 Sonic mapping frames with `pushv ,SonicDplcVer`
//! / `SonicDplcVer := 4` / ... / `popv ,SonicDplcVer` (`s2.asm` 69387, 69774).
//! sigil did not know either directive.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe (`p4_*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte is from a run that exited 0; every refusal is that build's
//! non-zero answer, read for accept-or-refuse only, except the string case,
//! where asl aborts (exit 134) and so has no answer.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | a queue instead of a stack (nested saves restore the wrong value) | `nested_saves_unwind_last_in_first_out` |
//! | `popv` reading its list backwards, so `popv ,A,B` mirrors `pushv ,A,B` | `one_list_is_last_in_first_out_too` |
//! | one stack for every name | `each_stack_name_is_its_own_stack` |
//! | the directives missing | every test here |

use sigil_frontend_as::{assemble_root_located_warned, Options};
use sigil_span::Level;

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

struct Out {
    bytes: Vec<u8>,
    warnings: Vec<String>,
}

fn assemble_with(head: &str, body: &str) -> Result<Out, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    let a = assemble_root_located_warned(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&a.module.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    Ok(Out {
        bytes: sigil_link::flatten(&linked, 0x00).unwrap(),
        warnings: a
            .warnings
            .iter()
            .filter(|d| d.level == Level::Warning)
            .map(|d| d.message.clone())
            .collect(),
    })
}

fn check_all(head: &str, cases: &[(&str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| match assemble_with(head, body) {
            Ok(o) if o.bytes == *want => None,
            Ok(o) => Some(format!("  {body:?}\n    asl   {want:02X?}\n    sigil {:02X?}", o.bytes)),
            Err(d) => Some(format!("  {body:?}\n    asl   {want:02X?}\n    sigil refused {d:?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} cases differ from asl:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

fn check_refused(cases: &[&str]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble_with(HEAD, body) {
            Err(_) => None,
            Ok(o) => Some(format!("  {body:?} -> {:02X?}", o.bytes)),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} refusals were not refused:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

/// Sonic 2's own shape, and the same save and restore under every spelling
/// asl accepts: `set` for `:=`, upper case, a float, a label's value, a
/// constant restored to the value it already has, inside a macro, across a
/// forward reference that needs a second pass, and on the Z80.
#[test]
fn a_saved_value_comes_back() {
    check_all(
        HEAD,
        &[
            ("Ver := 2\n\tdc.b Ver\n\tpushv ,Ver\nVer := 4\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver", &[0x02, 0x04, 0x02]),
            ("Ver set 2\n\tpushv ,Ver\nVer set 4\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver", &[0x04, 0x02]),
            ("Ver := 2\n\tPUSHV ,Ver\nVer := 4\n\tdc.b Ver\n\tPOPV ,Ver\n\tdc.b Ver", &[0x04, 0x02]),
            ("F := 1.5\n\tpushv ,F\nF := 2.5\n\tpopv ,F\n\tdc.l int(F*2)", &[0x00, 0x00, 0x00, 0x03]),
            ("\tdc.b 0\nLbl:\tdc.b 1\nA := 7\n\tpushv ,Lbl\n\tpopv ,A\n\tdc.b A", &[0x00, 0x01, 0x01]),
            ("E equ 5\n\tpushv ,E\n\tpopv ,E\n\tdc.b E", &[0x05]),
            (
                "sv macro\n\tpushv ,Ver\nVer := 9\n\tdc.b Ver\n\tpopv ,Ver\n\tendm\nVer := 3\n\tsv\n\tdc.b Ver",
                &[0x09, 0x03],
            ),
            (
                "Ver := 2\n\tpushv ,Ver\nVer := 4\n\tbra.w Fwd\n\tdc.b Ver\n\tpopv ,Ver\nFwd:\tdc.b Ver",
                &[0x60, 0x00, 0x00, 0x03, 0x04, 0x02],
            ),
        ],
    );
    check_all(HEAD_Z80, &[("Ver := 2\n\tpushv ,Ver\nVer := 4\n\tdb Ver\n\tpopv ,Ver\n\tdb Ver", &[0x04, 0x02])]);
}

/// Two saves, two restores: the second `popv` gets the FIRST value.
#[test]
fn nested_saves_unwind_last_in_first_out() {
    check_all(
        HEAD,
        &[(
            "Ver := 2\n\tpushv ,Ver\nVer := 4\n\tpushv ,Ver\nVer := 6\n\tdc.b Ver\n\
             \tpopv ,Ver\n\tdc.b Ver\n\tpopv ,Ver\n\tdc.b Ver",
            &[0x06, 0x04, 0x02],
        )],
    );
}

/// One list is last-in-first-out as well: `pushv ,A,B` pushes A then B, so
/// `popv ,A,B` gives A the old B and B the old A, and `popv ,B,A` restores.
#[test]
fn one_list_is_last_in_first_out_too() {
    check_all(
        HEAD,
        &[
            ("A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,A,B\n\tdc.b A,B", &[0x02, 0x01]),
            ("A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,B,A\n\tdc.b A,B", &[0x01, 0x02]),
            (
                "A := 1\nB := 2\n\tpushv ,A,B\nA := 3\nB := 4\n\tpopv ,A\n\tdc.b A,B\n\tpopv ,A\n\tdc.b A,B",
                &[0x02, 0x04, 0x01, 0x04],
            ),
        ],
    );
}

/// Every stack name is its own stack, the default one included.
#[test]
fn each_stack_name_is_its_own_stack() {
    check_all(
        HEAD,
        &[
            ("A := 1\n\tpushv s1,A\nA := 2\n\tpushv s2,A\nA := 3\n\tpopv s1,A\n\tdc.b A\n\tpopv s2,A\n\tdc.b A", &[0x01, 0x02]),
            ("A := 1\n\tpushv ,A\nA := 2\n\tpushv s1,A\nA := 3\n\tpopv ,A\n\tdc.b A\n\tpopv s1,A\n\tdc.b A", &[0x01, 0x02]),
        ],
    );
    // Stack names are case-sensitive: `s1` was never pushed (asl `#1530`).
    check_refused(&["A := 1\n\tpushv S1,A\nA := 2\n\tpopv s1,A\n\tdc.b A"]);
}

/// What asl refuses: popping an empty stack (`#1530`), a line with no symbol
/// (`#1110`), an empty symbol, an undefined symbol either side (`#1010`), and a
/// string symbol, on which asl aborts. And one deliberate divergence: asl lets
/// `popv` overwrite a constant or a label with a different value
/// (`p4_pop_into_equ_diff`, `p4_pop_into_label_diff`); sigil refuses it.
#[test]
fn what_asl_refuses_is_refused() {
    check_refused(&[
        "A := 1\n\tpopv ,A\n\tdc.b A",
        "A := 1\n\tpushv ,A\n\tpopv ,A\n\tpopv ,A\n\tdc.b A",
        "Ver := 2\n\tpushv Ver\n\tpopv Ver\n\tdc.b Ver",
        "\tpushv ,",
        "\tpushv ,Nope\n\tpopv ,Nope",
        "A := 7\n\tpushv ,A\n\tpopv ,B\n\tdc.b A,B",
        "S := \"ab\"\n\tpushv ,S\nS := \"cd\"\n\tpopv ,S\n\tdc.b S",
        "E equ 5\nA := 7\n\tpushv ,A\n\tpopv ,E\n\tdc.b E",
        "Lbl:\tdc.b 1\nA := 7\n\tpushv ,A\n\tpopv ,Lbl\n\tdc.b Lbl",
    ]);
}

/// A stack left holding values is a WARNING in asl, one per stack (`warning
/// #230: stack is not empty`), and the assembly still succeeds.
#[test]
fn a_stack_left_full_is_one_warning_per_stack() {
    let o = assemble_with(HEAD, "A := 1\n\tpushv ,A\n\tpushv s1,A\n\tpushv s1,A\n\tdc.b A")
        .expect("asl assembles this with exit 0");
    assert_eq!(o.bytes, vec![0x01]);
    let stack_warnings: Vec<&String> = o.warnings.iter().filter(|w| w.contains("never popped")).collect();
    assert_eq!(stack_warnings.len(), 2, "one per stack, got {:?}", o.warnings);
}
