//! A shift or rotate written with no size suffix is a WORD operation in asl,
//! in every form the 68000 has: memory, count in an immediate, count in a
//! register, and the one-operand register form.
//!
//! Sonic 2 writes the memory form eight times (`asl y_vel(a0)`, `asr y_vel(a0)`,
//! `asr y_vel(a1)`), and S3K 76 times. sigil refused each one with "instruction
//! needs an explicit size suffix".
//!
//! # Provenance
//!
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, `-xx -n -q -A -L -U -i .`, one construct
//! per probe (`s3_*.asm` in
//! `docs/superpowers/notes/2026-09-11-s2-as-small-features/probes/`). Every
//! expected byte is from a run that exited 0; every refusal is that build's
//! non-zero answer, read for accept-or-refuse only.
//!
//! # What a half-fix looks like, and which test goes red
//!
//! | half-fix | red here |
//! |---|---|
//! | the default given to `asl` (the corpus's spelling) and not the other seven | `every_memory_shift_and_rotate_defaults_to_word` |
//! | the default given to the memory form only | `every_register_shift_and_rotate_defaults_to_word` |
//! | a default of long (the register forms would encode `.l`) | `every_register_shift_and_rotate_defaults_to_word` |
//! | the memory form's word-only rule relaxed along the way | `a_memory_shift_is_still_word_only_and_counts_one` |

use sigil_frontend_as::{assemble_root_located, Options};

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, format!("{HEAD}{body}\n\tend\n")).expect("write probe");
    let m = assemble_root_located(&path, &Options::default())
        .map_err(|f| f.diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>())?;
    let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        .map_err(|e| vec![format!("{e:?}")])?;
    let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
        .map_err(|e| vec![format!("{e:?}")])?;
    Ok(sigil_link::flatten(&linked, 0x00).unwrap())
}

fn check_all(cases: &[(&str, &[u8])]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(body, want)| match assemble(body) {
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

/// All eight mnemonics in the memory form, on `(a0)` and across the
/// addressing modes the corpus and the probes use.
#[test]
fn every_memory_shift_and_rotate_defaults_to_word() {
    check_all(&[
        ("\tasl\t(a0)", &[0xE1, 0xD0]),
        ("\tasr\t(a0)", &[0xE0, 0xD0]),
        ("\tlsl\t(a0)", &[0xE3, 0xD0]),
        ("\tlsr\t(a0)", &[0xE2, 0xD0]),
        ("\trol\t(a0)", &[0xE7, 0xD0]),
        ("\tror\t(a0)", &[0xE6, 0xD0]),
        ("\troxl\t(a0)", &[0xE5, 0xD0]),
        ("\troxr\t(a0)", &[0xE4, 0xD0]),
        ("\tasl\t$1A(a0)", &[0xE1, 0xE8, 0x00, 0x1A]),
        ("\tasr\t$18(a0)", &[0xE0, 0xE8, 0x00, 0x18]),
        ("\tasr\t$1A(a1)", &[0xE0, 0xE9, 0x00, 0x1A]),
        ("\tasr\t(a1)", &[0xE0, 0xD1]),
        ("\tlsl\t(a2)+", &[0xE3, 0xDA]),
        ("\tlsr\t-(a3)", &[0xE2, 0xE3]),
        ("\trol\t$10(a4,d0.w)", &[0xE7, 0xF4, 0x00, 0x10]),
        ("\tlsr\t$10(a2)", &[0xE2, 0xEA, 0x00, 0x10]),
        ("\trol\t$10(a2)", &[0xE7, 0xEA, 0x00, 0x10]),
        ("\troxl\t$18(a0)", &[0xE5, 0xE8, 0x00, 0x18]),
        ("\troxr\t$1A(a0)", &[0xE4, 0xE8, 0x00, 0x1A]),
        ("\tASL\t$1A(a0)", &[0xE1, 0xE8, 0x00, 0x1A]),
        ("\tRoXr\t(a1)", &[0xE4, 0xD1]),
        // The explicit spelling is the same instruction.
        ("\tasl.w\t$1A(a0)", &[0xE1, 0xE8, 0x00, 0x1A]),
    ]);
}

/// The register forms default to word as well: count in an immediate, count
/// in a register, and the one-operand form (a count of 1). Each unsized line
/// is the same encoding asl gives the `.w` spelling.
#[test]
fn every_register_shift_and_rotate_defaults_to_word() {
    check_all(&[
        ("\tasl\t#1,d0", &[0xE3, 0x40]),
        ("\tasr\t#2,d1", &[0xE4, 0x41]),
        ("\tlsl\t#3,d2", &[0xE7, 0x4A]),
        ("\tlsr\t#4,d3", &[0xE8, 0x4B]),
        ("\trol\t#5,d4", &[0xEB, 0x5C]),
        ("\tror\t#6,d5", &[0xEC, 0x5D]),
        ("\troxl\t#7,d6", &[0xEF, 0x56]),
        ("\troxr\t#8,d7", &[0xE0, 0x57]),
        ("\tasl\td1,d0", &[0xE3, 0x60]),
        ("\tasr\td2,d1", &[0xE4, 0x61]),
        ("\tlsl\td3,d2", &[0xE7, 0x6A]),
        ("\tlsr\td4,d3", &[0xE8, 0x6B]),
        ("\trol\td5,d4", &[0xEB, 0x7C]),
        ("\tror\td6,d5", &[0xEC, 0x7D]),
        ("\troxl\td7,d6", &[0xEF, 0x76]),
        ("\troxr\td0,d7", &[0xE0, 0x77]),
        ("\tasl\td3", &[0xE3, 0x43]),
        ("\tasr\td3", &[0xE2, 0x43]),
        ("\tlsl\td3", &[0xE3, 0x4B]),
        ("\tlsr\td3", &[0xE2, 0x4B]),
        ("\trol\td3", &[0xE3, 0x5B]),
        ("\tror\td3", &[0xE2, 0x5B]),
        ("\troxl\td3", &[0xE3, 0x53]),
        ("\troxr\td3", &[0xE2, 0x53]),
        // An explicit size still wins.
        ("\tasl.b\td1,d0", &[0xE3, 0x20]),
        ("\tasl.l\t#3,d2", &[0xE7, 0x82]),
        ("\tlsr.b\td0", &[0xE2, 0x08]),
        ("\troxr.l\td7", &[0xE2, 0x97]),
    ]);
}

/// The memory form is word-only and shifts by one: asl refuses `.b` and `.l`
/// on it for all eight mnemonics (`#1130 invalid operand size`) and refuses a
/// count (`asl #2,(a0)` is `#1391 operand must be one`).
#[test]
fn a_memory_shift_is_still_word_only_and_counts_one() {
    let mut bodies: Vec<String> = Vec::new();
    for m in ["asl", "asr", "lsl", "lsr", "rol", "ror", "roxl", "roxr"] {
        bodies.push(format!("\t{m}.b\t(a0)"));
        bodies.push(format!("\t{m}.l\t(a0)"));
    }
    bodies.push("\tasl\t#2,(a0)".to_string());
    let accepted: Vec<String> = bodies
        .iter()
        .filter_map(|b| assemble(b).ok().map(|got| format!("  {b:?} -> {got:02X?}")))
        .collect();
    assert!(accepted.is_empty(), "asl refuses these, sigil assembled:\n{}", accepted.join("\n"));
}
