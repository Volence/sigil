//! Two refusals asl makes that sigil once did not: an ON/OFF switch directive
//! (`padding`, `supmode`) given anything but ON or OFF, and a `save` left open
//! when the source ends.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, run through `asl_run` from
//! `docs/superpowers/notes/asl-reference/asl_ref.sh` with
//! `-xx -n -q -A -L -U -i .`, one construct per probe file. Every probe here
//! is read for ACCEPT or REFUSE and the refusal's class only; no byte is quoted
//! from any run. The probe table is in
//! `docs/superpowers/notes/2026-09-25-as-overaccept-silent-three.md`.
//!
//! | asl verdict | shapes |
//! |---|---|
//! | accept | `on`, `off`, `ON`, `OFF`, `On`, `Off`, `oFf`, `off ; a comment`, and `on` with `ON equ 5` in scope, for `padding` and `supmode` alike |
//! | `#1520 only ON/OFF allowed` | `maybe`, `onx`, `1`, `0`, `(on)`, `"on"`, an undefined name, for `padding` and `supmode` alike |
//! | `#1110 wrong number of operands` | a bare `padding` / `supmode`; `padding on,off` / `supmode on,off` |
//! | accept | `save restore`; `save save restore restore`; a macro holding `save restore` called twice; `save` in an included file with `restore` in the root; `save` inside a false `if` |
//! | `#1460 missing RESTORE` | lone `save`; `save` with no `end` line; `save` in a macro body; `save` in an included file alone; `save save restore` (also after a `nop`) |
//! | `#1450 RESTORE without SAVE` | lone `restore`; `save restore restore` |
//!
//! # What each test catches
//!
//! | half-fix or regression | red here |
//! |---|---|
//! | any non-`off` word read as ON (the old rule) | `a_switch_refuses_every_argument_but_on_or_off` |
//! | the check put on `padding` and not on `supmode` | the same test, which runs both |
//! | case folded away or ON/OFF matched as an expression | `a_switch_accepts_on_and_off_in_any_case_and_as_a_keyword` |
//! | an open `save` at the end of the unit not checked | `a_save_left_open_at_the_end_of_the_source_is_refused` |
//! | the stack checked per FILE or with skipped lines counted | `balanced_or_skipped_saves_are_accepted` |
//! | the refusal not placed on the open `save` line | `the_open_save_refusal_names_the_save_line` |

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\torg 0\n";

/// Assemble `body` (plus any extra files) as `probe.asm`; `Ok(())` on a clean
/// run through the front end, layout, link and flatten, otherwise every
/// diagnostic rendered as `label: message`.
fn assemble(body: &str, extra: &[(&str, &str)]) -> Result<(), Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, text) in extra {
        std::fs::write(dir.path().join(name), text).expect("write extra file");
    }
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{HEAD}{body}\tdc.b 0\n\tend\n")).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            sigil_link::flatten(&linked, 0x00).expect("flatten");
            Ok(())
        }
        Err(f) => Err(f
            .diags
            .iter()
            .map(|d| {
                let at = f.sources.label(d.primary).unwrap_or_else(|| "<no label>".into());
                format!("{at}: {}", d.message)
            })
            .collect()),
    }
}

/// Every body must be refused with at least one diagnostic containing `needle`.
/// Reports every case that failed that, not just the first.
fn check_refused(cases: &[&str], needle: &str) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble(body, &[]) {
            Err(d) if d.iter().any(|x| x.contains(needle)) => None,
            other => Some(format!("  {body:?} -> {other:?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} shapes asl refuses were not refused naming {needle:?}:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

fn check_accepted(cases: &[&str]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble(body, &[]) {
            Ok(()) => None,
            Err(d) => Some(format!("  {body:?} -> {d:?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} shapes asl accepts were refused:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

#[test]
fn a_switch_refuses_every_argument_but_on_or_off() {
    for d in ["padding", "supmode"] {
        let vocab: Vec<String> = ["maybe", "onx", "1", "0", "(on)", "\"on\"", "zzundef"]
            .iter()
            .map(|a| format!("\t{d} {a}\n"))
            .collect();
        let vocab: Vec<&str> = vocab.iter().map(String::as_str).collect();
        check_refused(&vocab, &format!("`{d}` accepts only ON or OFF"));

        let count = [format!("\t{d}\n"), format!("\t{d} on,off\n")];
        let count: Vec<&str> = count.iter().map(String::as_str).collect();
        check_refused(&count, &format!("`{d}` takes exactly one argument"));
    }
}

#[test]
fn a_switch_accepts_on_and_off_in_any_case_and_as_a_keyword() {
    let mut cases = Vec::new();
    for d in ["padding", "supmode"] {
        for a in ["on", "off", "ON", "OFF", "On", "oFf"] {
            cases.push(format!("\t{d} {a}\n"));
        }
        cases.push(format!("ON equ 5\n\t{d} on\n"));
        cases.push(format!("\t{d} off ; a trailing comment\n"));
    }
    let cases: Vec<&str> = cases.iter().map(String::as_str).collect();
    check_accepted(&cases);
}

#[test]
fn a_save_left_open_at_the_end_of_the_source_is_refused() {
    check_refused(
        &[
            "\tsave\n",
            "\tsave\n\tsave\n\trestore\n",
            "M\tmacro\n\tsave\n\tendm\n\tM\n",
        ],
        "`save` with no matching `restore`",
    );
    // asl `#1450`, which sigil already refused; kept beside its mirror so the
    // pair is measured together.
    check_refused(
        &["\trestore\n", "\tsave\n\trestore\n\trestore\n"],
        "`restore` with no matching `save`",
    );
    // An included file that opens a `save` and nothing closes it.
    let r = assemble("\tinclude \"inc_save.inc\"\n", &[("inc_save.inc", "\tsave\n")]);
    assert!(
        matches!(&r, Err(d) if d.iter().any(|x| x.contains("`save` with no matching `restore`"))),
        "a `save` in an included file with no `restore` anywhere must be refused, got {r:?}"
    );
}

#[test]
fn balanced_or_skipped_saves_are_accepted() {
    check_accepted(&[
        "\tsave\n\trestore\n",
        "\tsave\n\tsave\n\trestore\n\trestore\n",
        "\tif 0\n\tsave\n\tendif\n",
        "M\tmacro\n\tsave\n\trestore\n\tendm\n\tM\n\tM\n",
    ]);
    // The stack belongs to the unit, not to a file.
    let r = assemble(
        "\tinclude \"inc_save.inc\"\n\trestore\n",
        &[("inc_save.inc", "\tsave\n")],
    );
    assert_eq!(r, Ok(()), "a `save` in an include closed by the root's `restore` is balanced");
}

#[test]
fn the_open_save_refusal_names_the_save_line() {
    // Line 4 is the unmatched outer `save`; line 5's is closed by line 6.
    let r = assemble("\tnop\n\tsave\n\tsave\n\trestore\n", &[]);
    let Err(d) = r else {
        panic!("an open `save` must be refused");
    };
    let hits: Vec<&String> = d.iter().filter(|x| x.contains("`save` with no matching")).collect();
    assert_eq!(hits.len(), 1, "exactly one open `save`, got {d:?}");
    assert!(
        hits[0].split(": ").next().is_some_and(|at| at.ends_with("probe.asm(4):2")),
        "the refusal must be in the AS shape `file(line):` at the open `save`, got {:?}",
        hits[0]
    );
    for m in &d {
        assert!(
            !m.contains('\u{2014}') && !m.contains('\u{2013}'),
            "diagnostic carries an em or en dash: {m:?}"
        );
    }
}
