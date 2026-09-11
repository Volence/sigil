//! A struct member whose name starts with a digit.
//!
//! Sonic 2's sound driver declares `1upPlaying: ds.b 1` inside `zVar STRUCT
//! DOTS` and reads it back as `zAbsVar.1upPlaying` and `zVar.1upPlaying`. asl
//! accepts the member, because the symbol it defines is `zVar.1upPlaying`, and
//! refuses the same word as a plain label (`#1020 invalid symbol name`). sigil
//! refused the member line, which also shortened `zVar` by a byte, and so every
//! member after it.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe (`d2_*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte is from a run that exited 0; every refusal is that build's
//! non-zero answer, read for accept-or-refuse only.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | digit-led words lexed as identifiers everywhere (plain labels accepted too) | `a_digit_led_plain_label_is_still_refused` |
//! | the member's name column read at any indentation without a colon | `an_indented_digit_led_member_without_a_colon_is_refused` |
//! | the member line still refused | every acceptance test here |

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

fn assemble_with(head: &str, body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    // An undefined member is refused at the LINK, as an unresolved fixup, so a
    // link failure is a refusal here and not a harness panic.
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

fn check_all(head: &str, cases: &[(&str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| match assemble_with(head, body) {
            Ok(got) if got == *want => None,
            other => Some(format!("  {body:?}\n    asl   {want:02X?}\n    sigil {other:02X?}")),
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

fn check_refused(head: &str, cases: &[&str]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble_with(head, body) {
            Err(_) => None,
            other => Some(format!("  {body:?} -> {other:02X?}")),
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

/// The member takes its offset, and the members after it and the struct's
/// length are where asl puts them. Both separators (`DOTS` and the default
/// `_`), a column-0 name without a colon, a marker, word and long fields, and
/// names that are all digits or spell a hex number.
#[test]
fn a_digit_led_member_takes_its_offset_and_the_struct_its_length() {
    let s = |body: &str| format!("zV STRUCT DOTS\n{body}zV ENDSTRUCT\n");
    check_all(
        HEAD,
        &[
            (
                &(s("\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\n\tB:\t\tds.b 1\n")
                    + "\tdc.b zV.1upPlaying\n\tdc.b zV.B\n\tdc.b zV.len"),
                &[0x01, 0x02, 0x03],
            ),
            (
                "zV STRUCT\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n\
                 \tdc.b zV_1upPlaying\n\tdc.b zV_B\n\tdc.b zV_len",
                &[0x01, 0x02, 0x03],
            ),
            (
                &(s("\tA:\t\tds.b 2\n\t1upPlaying:\tds.b 1\n")
                    + "\tdc.b zV.1upPlaying+1\n\tdc.b 2*zV.1upPlaying"),
                &[0x03, 0x04],
            ),
            (
                &(s("\tA:\t\tds.b 1\n1upPlaying\tds.b 1\n\tB:\t\tds.b 1\n")
                    + "\tdc.b zV.1upPlaying\n\tdc.b zV.B\n\tdc.b zV.len"),
                &[0x01, 0x02, 0x03],
            ),
            (
                &(s("\tA:\t\tds.b 1\n\t1up:\n\tB:\t\tds.b 1\n")
                    + "\tdc.b zV.1up\n\tdc.b zV.B\n\tdc.b zV.len"),
                &[0x01, 0x01, 0x02],
            ),
            (
                &(s("\tA:\t\tds.b 1\n\t1up:\tds.w 1\n\t2up:\tds.l 2\n\tB:\t\tds.b 1\n")
                    + "\tdc.b zV.1up\n\tdc.b zV.2up\n\tdc.b zV.B\n\tdc.b zV.len"),
                &[0x01, 0x03, 0x0B, 0x0C],
            ),
            (&(s("\tA:\t\tds.b 1\n\t2:\tds.b 1\n") + "\tdc.b zV.len"), &[0x02]),
            (
                &(s("\tA:\t\tds.b 1\n\t12h:\tds.b 1\n\tB:\tds.b 1\n") + "\tdc.b zV.B\n\tdc.b zV.len"),
                &[0x02, 0x03],
            ),
        ],
    );
}

/// An instance hangs the member off its own label: `Inst.1upPlaying`.
#[test]
fn an_instance_carries_the_digit_led_member() {
    let body = "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n\
                \tdc.b $11\nInst:\tzV\n\tdc.l Inst.1upPlaying";
    let got = assemble_with(HEAD, body).expect("asl assembles this, exit 0");
    // asl: `dc.b $11` at 0, the instance at 1 (two bytes), `dc.l` at 3 is
    // `0000 0002`.
    assert_eq!(got.first(), Some(&0x11), "{got:02X?}");
    assert_eq!(got.get(3..7), Some(&[0x00, 0x00, 0x00, 0x02][..]), "{got:02X?}");
}

/// The Z80 reads it the way the driver does: absolute and indexed.
#[test]
fn a_z80_operand_reads_the_digit_led_member() {
    check_all(
        "\tcpu 68000\n",
        &[(
            "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n\
             \tcpu z80\n\torg 0\n\tld a,(zV.1upPlaying)\n\tld a,(ix+zV.1upPlaying)",
            &[0x3A, 0x01, 0x00, 0xDD, 0x7E, 0x01],
        )],
    );
}

/// THE OTHER HALF. The same word is not a symbol name outside a struct: asl
/// refuses it as a label with or without a colon, as an `equ` or `=` name, and
/// as a bare reference to the member (`#1020 invalid symbol name` each time).
/// A dotted label whose first character is a letter is fine (`Foo.1up`).
#[test]
fn a_digit_led_plain_label_is_still_refused() {
    check_refused(
        HEAD,
        &[
            "\tdc.b $11\n1up:\tdc.b $22",
            "\tdc.b $11\n1up\tdc.b $22",
            "1up equ 5",
            "1up = 5",
            "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n\tdc.b 1upPlaying",
        ],
    );
    check_all(HEAD, &[("\tdc.b $11\nFoo.1up:\tdc.b $22\n\tdc.b Foo.1up", &[0x11, 0x22, 0x01])]);
}

/// The name column follows asl's column rule: an indented word with no colon
/// is an instruction, and asl refuses `\t1upPlaying\tds.b 1` inside a struct as
/// `#1200 unknown instruction`. An undefined member is refused too.
#[test]
fn an_indented_digit_led_member_without_a_colon_is_refused() {
    check_refused(
        HEAD,
        &[
            "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying\tds.b 1\n\tB:\t\tds.b 1\nzV ENDSTRUCT\n\
             \tdc.b zV.B\n\tdc.b zV.len",
            "zV STRUCT DOTS\n\tA:\t\tds.b 1\n\t1upPlaying:\tds.b 1\nzV ENDSTRUCT\n\tdc.b zV.1foo",
        ],
    );
}
