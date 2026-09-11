//! An empty parenthesised group, `()`, is the value 0 in asl.
//!
//! Sonic 2 reaches it through a macro argument the caller left off: the sound
//! driver's `music_metadata macro DATA,FLAGS` writes `(FLAGS)` into a `db`, and
//! 30 of its 31 calls omit FLAGS, so asl assembles `...|()|...` 30 times. sigil
//! refused every one as `bad byte expression`.
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe file (`e1_*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte below is read out of a run that exited 0; every refusal is that
//! build's non-zero answer, read for accept-or-refuse only.
//!
//! # Where asl does NOT read `()` as a plain 0, and what a half-fix breaks
//!
//! | half-fix | red here |
//! |---|---|
//! | a Z80 `()` operand read as an indirection through address 0 (`ld a,()` as `3A 00 00`) | `a_z80_operand_of_empty_parens_is_an_immediate_not_an_indirection` |
//! | an empty OPERAND read as 0 too (`dc.b 1,,2`) | `an_empty_operand_is_still_refused` |
//! | a user function's empty argument substituted without asl's argument count (`f()` folding) | `a_user_function_counts_its_arguments_the_way_asl_does` |
//! | the count taken from token groups alone, blind to blanks (`f( )` refused, `f(1,)` folded) | `a_user_function_counts_its_arguments_the_way_asl_does` |
//! | `()` read in the expression parser but not the typed evaluator | `an_empty_group_is_zero_in_every_data_width` (`int(()+1.5)`) |

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
const HEAD_Z80: &str = "\tcpu z80\n\torg 0\n";

fn assemble_with(head: &str, body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{head}{body}\n\tend\n")).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

/// Assemble every `(body, asl's bytes)` case and report ALL the mismatches at
/// once, so a regression names each form it broke rather than the first.
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

/// Assemble every body and require a refusal; report every one that came back
/// as bytes.
fn check_refused(head: &str, cases: &[&str], must_mention: Option<&str>) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|body| match assemble_with(head, body) {
            Err(d) if must_mention.is_none_or(|m| d.iter().any(|x| x.contains(m))) => None,
            other => Some(format!("  {body:?} -> {other:02X?}")),
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} refusals were not refused{}:\n{}",
        wrong.len(),
        cases.len(),
        must_mention.map(|m| format!(" with a diagnostic naming `{m}`")).unwrap_or_default(),
        wrong.join("\n")
    );
}

/// Every operator position a `()` can take in a data directive, at every
/// width. `~()` is `FF` and `~~()` is `01`, so the group is a real integer 0,
/// not an absent operand the operator skips.
#[test]
fn an_empty_group_is_zero_in_every_data_width() {
    check_all(
        HEAD,
        &[
            ("\tdc.b $10|()|$02", &[0x12]),
            ("\tdc.b ()", &[0x00]),
            ("\tdc.b ( )", &[0x00]),
            ("\tdc.b (())", &[0x00]),
            ("\tdc.b -()", &[0x00]),
            ("\tdc.b ~()", &[0xFF]),
            ("\tdc.b ~~()", &[0x01]),
            ("\tdc.b ()+1", &[0x01]),
            ("\tdc.b 1+()", &[0x01]),
            ("\tdc.b 5*()", &[0x00]),
            ("\tdc.b ()<<1", &[0x00]),
            ("\tdc.b ()=0", &[0x01]),
            ("\tdc.w ()", &[0x00, 0x00]),
            ("\tdc.l ()", &[0x00, 0x00, 0x00, 0x00]),
            ("\tdc.l int(()+1.5)", &[0x00, 0x00, 0x00, 0x01]),
        ],
    );
}

/// 68000 operands. `()(a0)` is `0(a0)`, which asl (and sigil) shorten to
/// `(a0)`; `().w` and a bare `()` are the absolute address 0.
#[test]
fn an_empty_group_is_zero_in_68000_operands() {
    check_all(
        HEAD,
        &[
            ("\tmove.w #(),d0", &[0x30, 0x3C, 0x00, 0x00]),
            ("\tmove.w #1|(),d0", &[0x30, 0x3C, 0x00, 0x01]),
            ("\tmove.w ()(a0),d0", &[0x30, 0x10]),
            ("\tmove.w ().w,d0", &[0x30, 0x38, 0x00, 0x00]),
            ("\tmove.w (),d0", &[0x30, 0x38, 0x00, 0x00]),
        ],
    );
}

/// The directives that want a constant: a binder, a condition, a repeat count
/// and a reservation all see 0.
#[test]
fn an_empty_group_is_zero_where_a_directive_wants_a_constant() {
    check_all(
        HEAD,
        &[
            ("X equ ()\n\tdc.b X", &[0x00]),
            ("X set ()\n\tdc.b X", &[0x00]),
            ("X := ()\n\tdc.b X", &[0x00]),
            ("\tif ()\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif", &[0x02]),
            ("\tif ()|1\n\tdc.b 1\n\telse\n\tdc.b 2\n\tendif", &[0x01]),
            ("\trept ()\n\tdc.b 1\n\tendm\n\tdc.b $EE", &[0xEE]),
            ("\tds.b ()\n\tdc.b $EE", &[0xEE]),
        ],
    );
}

/// THE HALF-FIX TRAP. A Z80 operand written as parentheses is normally an
/// indirection, and `(0)` is one (`3A 00 00`). An EMPTY group is not: asl
/// assembles `ld a,()` as the immediate `3E 00` and `ld hl,()` as `21 00 00`.
/// Reading `()` as 0 inside the parentheses gives `3A 00 00`, a load from
/// memory, with exit 0.
#[test]
fn a_z80_operand_of_empty_parens_is_an_immediate_not_an_indirection() {
    check_all(
        HEAD_Z80,
        &[
            ("\tld a,()", &[0x3E, 0x00]),
            ("\tld hl,()", &[0x21, 0x00, 0x00]),
            ("\tld a,1|()", &[0x3E, 0x01]),
            ("\tld a,(1|())", &[0x3A, 0x01, 0x00]),
            ("\tld a,(0)", &[0x3A, 0x00, 0x00]),
            ("\tdb 10h|()|02h", &[0x12]),
            ("\tdb ()", &[0x00]),
            ("\tdw ()", &[0x00, 0x00]),
        ],
    );
}

/// The corpus shape: an argument the call leaves off pastes as nothing, so a
/// body's `(pb)` becomes `()`. The last case is Sonic 2's own `music_metadata`
/// line with its helper functions, one call omitting FLAGS and one passing it.
#[test]
fn an_omitted_macro_argument_pastes_as_zero() {
    check_all(HEAD, &[("m macro pa,pb\n\tdc.b (pa)|(pb)\n\tendm\n\tm 5", &[0x05])]);
    check_all(
        HEAD_Z80,
        &[
            ("m macro pa,pb\n\tdb (pa)|(pb)\n\tendm\n\tm 5", &[0x05]),
            (
                "z80_bank_size = 8000h\n\
                 getZ80BankOffset function label, label # z80_bank_size\n\
                 getZ80BankBase function label, label - getZ80BankOffset(label)\n\
                 withinSameZ80Bank function label1, label2, getZ80BankBase(label1) == getZ80BankBase(label2)\n\
                 MusFlag_SlowerOnPAL = 1 << 6\n\
                 music_metadata macro DATA,FLAGS\n\
                 \tdb\t(withinSameZ80Bank(DATA.pointer, MusicPoint2)<<7)|((~~DATA.is_compressed)<<5)|(FLAGS)|(getZ80BankOffset(DATA.pointer)/2)\n\
                 \x20   endm\n\
                 Mus_EHZ.pointer = 10020h\nMus_EHZ.is_compressed = 1\n\
                 Mus_X.pointer = 8030h\nMus_X.is_compressed = 0\n\
                 MusicPoint2 = 10000h\n\
                 \tmusic_metadata Mus_EHZ\n\
                 \tmusic_metadata Mus_X,MusFlag_SlowerOnPAL",
                &[0x90, 0x78],
            ),
        ],
    );
}

/// An empty OPERAND is not an empty group. asl refuses `dc.b 1,,2` and `dc.b 1,`
/// (`#2050 empty argument`) and a macro body that pastes a bare omitted argument
/// into `dc.b` (`#1110`). And `()` is a real zero, so dividing by it is refused.
#[test]
fn an_empty_operand_is_still_refused() {
    check_refused(
        HEAD,
        &[
            "\tdc.b 1,,2",
            "\tdc.b 1,",
            "m macro pa,pb\n\tdc.b pb\n\tendm\n\tm 5",
        ],
        None,
    );
    check_refused(HEAD, &["\tdc.b 4/()"], Some("division by zero"));
}

/// A user `function`'s empty argument is `()`, so what decides whether one
/// exists is asl's COUNT of argument text: the text after the last comma counts
/// only when it is not empty, and a blank is not empty.
#[test]
fn a_user_function_counts_its_arguments_the_way_asl_does() {
    let one = "f function x,x+1\n";
    let two = "f function x,y,x+y\n";
    let three = "f function x,y,z,x+y+z\n";
    check_all(
        HEAD,
        &[
            (&format!("{one}\tdc.b f(())"), &[0x01]),
            (&format!("{one}\tdc.b f( )"), &[0x01]),
            (&format!("{two}\tdc.b f(,1)"), &[0x01]),
            (&format!("{two}\tdc.b f((),1)"), &[0x01]),
            (&format!("{three}\tdc.b f(1,,2)"), &[0x03]),
        ],
    );
    check_refused(
        HEAD,
        &[
            &format!("{one}\tdc.b f()"),
            &format!("{two}\tdc.b f(1,)"),
            &format!("{two}\tdc.b f(,)"),
        ],
        Some("wrong number of function arguments"),
    );
}
